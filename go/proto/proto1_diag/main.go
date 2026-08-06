// Proto 1 Diagnostic — Raw HDF5 byte inspection to pinpoint root attr failure.
//
// Reads the root group object header address from the superblock, checks
// the 4-byte signature at that address, finds AttributeInfo messages,
// and dumps enough info to distinguish between:
//   (A) traditional/SNOD path → address never set on Group struct
//   (B) modern/OHDR path → address set, but dense attr parsing fails
//
// Run from go/ dir: go run ./proto/proto1_diag/
package main

import (
	"encoding/binary"
	"fmt"
	"log"
	"os"
	"path/filepath"
)

func mustFindFixture(name string) string {
	cwd, _ := os.Getwd()
	candidates := []string{
		filepath.Join(cwd, "fixtures", name),
		filepath.Join(cwd, "..", "fixtures", name),
		filepath.Join(cwd, "..", "..", "..", "fixtures", name),
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			return c
		}
	}
	log.Fatalf("cannot find fixture %q", name)
	return ""
}

func main() {
	path := mustFindFixture("reference_config.h5")
	f, err := os.Open(path)
	if err != nil {
		log.Fatalf("open: %v", err)
	}
	defer f.Close()

	// ── 1. Read and validate HDF5 signature ──────────────────────────────────
	sig := make([]byte, 8)
	if _, err := f.ReadAt(sig, 0); err != nil {
		log.Fatalf("read signature: %v", err)
	}
	expected := []byte{0x89, 0x48, 0x44, 0x46, 0x0d, 0x0a, 0x1a, 0x0a}
	for i, b := range expected {
		if sig[i] != b {
			log.Fatalf("not an HDF5 file; first 8 bytes: %x", sig)
		}
	}
	fmt.Println("✓ HDF5 signature OK")

	// ── 2. Read superblock ────────────────────────────────────────────────────
	sb := make([]byte, 64)
	if _, err := f.ReadAt(sb, 0); err != nil {
		log.Fatalf("read superblock: %v", err)
	}

	version := sb[8]
	fmt.Printf("  Superblock version: %d\n", version)

	offsetSize := sb[9]
	lengthSize := sb[10]
	fmt.Printf("  OffsetSize: %d  LengthSize: %d\n", offsetSize, lengthSize)

	// Determine endianness (byte 9 = 0 for little-endian in v2/v3)
	// In v0: byte 13 has byte-order flag; in v2/v3 there's a flags byte
	order := binary.LittleEndian // h5py always writes little-endian on x86
	var rootGroupAddr uint64
	switch version {
	case 0, 1:
		// v0: root group is at offset 40 as a "Symbol Table Entry" (16 bytes)
		// First 8 bytes of the entry is the name offset in local heap (usually 0)
		// Next 8 bytes is the object header address
		// Actually for v0: bytes 40-47 = name offset, 48-55 = obj header addr
		rootGroupAddr = order.Uint64(sb[48:56])
	case 2, 3:
		// v2/v3: at byte 36 (after base=12, superext=20, eof=28, root=36)
		rootGroupAddr = order.Uint64(sb[36:44])
	default:
		log.Fatalf("unsupported superblock version %d", version)
	}
	fmt.Printf("  Root group object header address: 0x%X\n", rootGroupAddr)

	// ── 3. Check signature at root group address ──────────────────────────────
	rootSig := make([]byte, 4)
	if _, err := f.ReadAt(rootSig, int64(rootGroupAddr)); err != nil {
		log.Fatalf("read at root addr 0x%X: %v", rootGroupAddr, err)
	}
	fmt.Printf("  Signature at root addr: %q (0x%X 0x%X 0x%X 0x%X)\n",
		string(rootSig), rootSig[0], rootSig[1], rootSig[2], rootSig[3])

	switch string(rootSig) {
	case "SNOD":
		fmt.Println("  ⚠  ROOT GROUP = SNOD (traditional format)")
		fmt.Println("     loadTraditionalGroup will be called → group.address stays 0")
		fmt.Println("     Attributes() guard 'if g.address == 0' returns empty immediately")
		fmt.Println("  ✗ This confirms issue A: address never set on Group struct")
		diagSNOD(f, rootGroupAddr, order)
	case "OHDR":
		fmt.Println("  ✓ ROOT GROUP = OHDR (modern format)")
		fmt.Println("     loadModernGroup will be called → group.address will be set")
		fmt.Println("     Failure must be inside ParseAttributesFromMessages / readDenseAttributes")
		fmt.Println("  → Checking object header messages...")
		diagOHDR(f, rootGroupAddr, offsetSize, order)
	default:
		// Could be v1 object header (no signature, starts with version byte)
		fmt.Printf("  ? Unknown signature — possibly v1 object header (byte 0 = 0x%02X)\n", rootSig[0])
		if rootSig[0] == 0x01 {
			fmt.Println("    v1 object header detected → loadModernGroup path, but v1 header parsing")
		}
	}
}

func diagSNOD(f *os.File, addr uint64, order binary.ByteOrder) {
	// Read SNOD header to show entry count
	buf := make([]byte, 8)
	if _, err := f.ReadAt(buf, int64(addr)); err != nil {
		return
	}
	// SNOD: sig(4) ver(1) reserved(1) numSymbols(2)
	ver := buf[4]
	numSym := order.Uint16(buf[6:8])
	fmt.Printf("  SNOD version=%d numSymbols=%d\n", ver, numSym)
	fmt.Println("  NOTE: scigolib loadTraditionalGroup does not set group.address")
	fmt.Println("  FIX: patch loadTraditionalGroup to read the root object header address")
	fmt.Println("       from the symbol table entry and store it in group.address")
}

func diagOHDR(f *os.File, addr uint64, offsetSize byte, order binary.ByteOrder) {
	// Read OHDR v2 header:
	// sig(4) ver(1) flags(1) ...messages
	buf := make([]byte, 256)
	n, _ := f.ReadAt(buf, int64(addr))
	if n < 8 {
		fmt.Println("  ✗ Could not read enough bytes for OHDR")
		return
	}

	ver := buf[4]
	flags := buf[5]
	fmt.Printf("  OHDR version=%d flags=0x%02X\n", ver, flags)

	// Scan messages looking for AttributeInfo (type 0x000F) and Attribute (type 0x000C)
	// v2 messages: type(1) flags(1) data_size(2) data(N) [with creation order if flags bit 2]
	offset := 6
	hasTimeFields := (flags & 0x10) != 0 // bit 4 = store timestamps
	hasAttrPhase := (flags & 0x04) != 0  // bit 2 = track creation order
	_ = hasAttrPhase

	if hasTimeFields {
		offset += 16 // 4× uint32 timestamps
	}

	// For chunk 0 size
	chunkSizeBytes := 1 + int(flags&0x03) // bits 0-1: 0→1 byte, 1→2 bytes, 2→4 bytes, 3→8 bytes
	if chunkSizeBytes > n-offset {
		fmt.Printf("  Object header chunk size field: need %d bytes, only %d available\n", chunkSizeBytes, n-offset)
		return
	}
	var chunk0Size uint32
	switch chunkSizeBytes {
	case 1:
		chunk0Size = uint32(buf[offset])
	case 2:
		chunk0Size = uint32(order.Uint16(buf[offset : offset+2]))
	case 4:
		chunk0Size = order.Uint32(buf[offset : offset+4])
	}
	offset += chunkSizeBytes
	fmt.Printf("  OHDR chunk0 size: %d bytes\n", chunk0Size)
	fmt.Printf("  Messages start at offset %d from file addr 0x%X\n", offset, addr)

	// Read chunk0 messages
	msgBuf := make([]byte, chunk0Size)
	if _, err := f.ReadAt(msgBuf, int64(addr)+int64(offset)); err != nil {
		fmt.Printf("  ✗ Could not read messages: %v\n", err)
		return
	}

	foundAttrInfo := false
	foundAttr := false
	compactAttrCount := 0
	msgOffset := 0
	for msgOffset+4 <= len(msgBuf) {
		msgType := uint8(msgBuf[msgOffset])
		msgFlags := msgBuf[msgOffset+1]
		msgSize := order.Uint16(msgBuf[msgOffset+2 : msgOffset+4])
		msgOffset += 4
		_ = msgFlags
		if int(msgOffset)+int(msgSize) > len(msgBuf) {
			break
		}
		switch msgType {
		case 0x0C: // Attribute message
			compactAttrCount++
			foundAttr = true
		case 0x0F: // AttributeInfo message (dense storage pointer)
			foundAttrInfo = true
			// Parse the AttributeInfo to show addresses
			data := msgBuf[msgOffset : msgOffset+int(msgSize)]
			if len(data) >= 1+2+int(offsetSize)+int(offsetSize) {
				aiFlags := data[0]
				aiOff := 1
				if (aiFlags & 0x01) != 0 {
					aiOff += 2 // max creation order index
				}
				if aiOff+int(offsetSize)*2 <= len(data) {
					var fractalHeap, bTreeAddr uint64
					if offsetSize == 8 {
						fractalHeap = order.Uint64(data[aiOff : aiOff+8])
						bTreeAddr = order.Uint64(data[aiOff+8 : aiOff+16])
					} else {
						fractalHeap = uint64(order.Uint32(data[aiOff : aiOff+4]))
						bTreeAddr = uint64(order.Uint32(data[aiOff+4 : aiOff+8]))
					}
					fmt.Printf("  ✓ AttributeInfo msg found: fractalHeap=0x%X  btree=0x%X\n",
						fractalHeap, bTreeAddr)
					// Check BTHD signature at btree addr
					btSig := make([]byte, 4)
					if _, err := f.ReadAt(btSig, int64(bTreeAddr)); err == nil {
						fmt.Printf("    Signature at btree addr: %q\n", string(btSig))
					}
					// Check FRHP signature at heap addr
					hpSig := make([]byte, 4)
					if _, err := f.ReadAt(hpSig, int64(fractalHeap)); err == nil {
						fmt.Printf("    Signature at heap addr:  %q\n", string(hpSig))
					}
				}
			}
		}
		msgOffset += int(msgSize)
	}
	if foundAttrInfo {
		fmt.Println("  → Dense attribute storage in use (> 8 attrs in fractal heap + B-tree v2)")
		fmt.Println("    The library calls readDenseAttributes — check if B-tree/heap parsing works")
	}
	if foundAttr {
		fmt.Printf("  → %d compact attribute message(s) found in object header\n", compactAttrCount)
	}
	if !foundAttrInfo && !foundAttr {
		fmt.Println("  ✗ No attribute messages found in object header chunk")
	}
}
