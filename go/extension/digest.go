package extension

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"math"
	"sort"
)

// Digest is the hex SHA-256 of the tree's canonical encoding, identical in every language
// (all little-endian):
//
//	tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
//	name    = u32 byteLength, UTF-8 bytes
//	attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
//	dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
//	value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
//
// Entries are sorted by the UTF-8 bytes of their names. NaN is the quiet NaN and -0 is +0.
// It returns an error if the tree violates the contract.
func Digest(t *Tree) (string, error) {
	if err := Validate(t, ""); err != nil {
		return "", err
	}
	var buf bytes.Buffer
	putTree(&buf, t)
	sum := sha256.Sum256(buf.Bytes())
	return hex.EncodeToString(sum[:]), nil
}

func putU32(buf *bytes.Buffer, n uint32) {
	var b [4]byte
	binary.LittleEndian.PutUint32(b[:], n)
	buf.Write(b[:])
}

func putU64(buf *bytes.Buffer, n uint64) {
	var b [8]byte
	binary.LittleEndian.PutUint64(b[:], n)
	buf.Write(b[:])
}

func putName(buf *bytes.Buffer, s string) {
	putU32(buf, uint32(len(s)))
	buf.WriteString(s)
}

func putSigned(buf *bytes.Buffer, dtype string, v int64) {
	switch dtype {
	case I8:
		buf.WriteByte(byte(int8(v)))
	case I16:
		var b [2]byte
		binary.LittleEndian.PutUint16(b[:], uint16(int16(v)))
		buf.Write(b[:])
	case I32:
		putU32(buf, uint32(int32(v)))
	default:
		putU64(buf, uint64(v))
	}
}

func putUnsigned(buf *bytes.Buffer, dtype string, v uint64) {
	switch dtype {
	case U8:
		buf.WriteByte(byte(v))
	case U16:
		var b [2]byte
		binary.LittleEndian.PutUint16(b[:], uint16(v))
		buf.Write(b[:])
	case U32:
		putU32(buf, uint32(v))
	default:
		putU64(buf, v)
	}
}

func putFloat(buf *bytes.Buffer, dtype string, v float64) {
	if dtype == F32 {
		if math.IsNaN(v) {
			putU32(buf, 0x7FC00000)
			return
		}
		f := float32(v)
		if f == 0 {
			f = 0 // folds -0 into +0
		}
		putU32(buf, math.Float32bits(f))
		return
	}
	if math.IsNaN(v) {
		putU64(buf, 0x7FF8000000000000)
		return
	}
	if v == 0 {
		v = 0
	}
	putU64(buf, math.Float64bits(v))
}

// putElements writes value (a scalar, or a slice when asSlice) element by element.
func putElements(buf *bytes.Buffer, dtype string, value any) {
	switch v := value.(type) {
	case string:
		putName(buf, v)
	case []string:
		for _, s := range v {
			putName(buf, s)
		}
	case int64:
		putSigned(buf, dtype, v)
	case []int64:
		for _, n := range v {
			putSigned(buf, dtype, n)
		}
	case uint64:
		putUnsigned(buf, dtype, v)
	case []uint64:
		for _, n := range v {
			putUnsigned(buf, dtype, n)
		}
	case float64:
		putFloat(buf, dtype, v)
	case []float64:
		for _, n := range v {
			putFloat(buf, dtype, n)
		}
	}
}

func isSliceValue(value any) (int, bool) {
	switch v := value.(type) {
	case []string:
		return len(v), true
	case []int64:
		return len(v), true
	case []uint64:
		return len(v), true
	case []float64:
		return len(v), true
	}
	return 0, false
}

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys) // Go compares strings bytewise, which is UTF-8 byte order
	return keys
}

func putAttrs(buf *bytes.Buffer, attrs map[string]Attr) {
	names := sortedKeys(attrs)
	putU32(buf, uint32(len(names)))
	for _, n := range names {
		a := attrs[n]
		putName(buf, n)
		buf.WriteByte(dtypeTag[a.Dtype])
		if count, isSlice := isSliceValue(a.Value); isSlice {
			buf.WriteByte(1)
			putU32(buf, uint32(count))
		} else {
			buf.WriteByte(0)
		}
		putElements(buf, a.Dtype, a.Value)
	}
}

func putTree(buf *bytes.Buffer, t *Tree) {
	putAttrs(buf, t.Attrs)
	children := sortedKeys(t.Children)
	putU32(buf, uint32(len(children)))
	for _, n := range children {
		putName(buf, n)
		putTree(buf, t.Children[n])
	}
	datasets := sortedKeys(t.Datasets)
	putU32(buf, uint32(len(datasets)))
	for _, n := range datasets {
		ds := t.Datasets[n]
		putName(buf, n)
		buf.WriteByte(dtypeTag[ds.Dtype])
		putU32(buf, uint32(len(ds.Shape)))
		for _, d := range ds.Shape {
			putU64(buf, d)
		}
		count, _ := dataLen(ds.Data)
		putU64(buf, uint64(count))
		putElements(buf, ds.Dtype, ds.Data)
		putAttrs(buf, ds.Attrs)
	}
}
