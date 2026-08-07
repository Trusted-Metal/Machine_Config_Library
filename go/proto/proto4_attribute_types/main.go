// Proto 4 — Write and read back string, int64, and float64 dataset attributes.
//
// Validates attribute-type fidelity for the three Go primitives the machine
// config reader/writer will use most:
//   • string  — machine names, version strings, serial numbers
//   • int64   — integer metadata fields
//   • float64 — numeric metadata fields
//
// Run from repo root:  go run ./go/proto/proto4_attribute_types/
// Run from go/ dir:    go run ./proto/proto4_attribute_types/
package main

import (
	"fmt"
	"log"
	"math"
	"os"
	"strings"

	"github.com/scigolib/hdf5"
)

func main() {
	tmp, err := os.CreateTemp("", "proto4_*.h5")
	if err != nil {
		log.Fatalf("temp file: %v", err)
	}
	tmpPath := tmp.Name()
	tmp.Close()
	defer os.Remove(tmpPath)

	// ── Write ────────────────────────────────────────────────────────────────
	fw, err := hdf5.CreateForWrite(tmpPath, hdf5.CreateTruncate)
	if err != nil {
		log.Fatalf("CreateForWrite: %v", err)
	}

	// Attach attributes to a small sentinel dataset at the root.
	dw, err := fw.CreateDataset("/meta", hdf5.Float64, []uint64{1})
	if err != nil {
		log.Fatalf("CreateDataset /meta: %v", err)
	}
	if err := dw.Write([]float64{0.0}); err != nil {
		log.Fatalf("dw.Write sentinel: %v", err)
	}
	if err := dw.WriteAttribute("str_val", "hello"); err != nil {
		log.Fatalf("WriteAttribute str_val: %v", err)
	}
	if err := dw.WriteAttribute("int_val", int64(42)); err != nil {
		log.Fatalf("WriteAttribute int_val: %v", err)
	}
	if err := dw.WriteAttribute("float_val", float64(3.14159)); err != nil {
		log.Fatalf("WriteAttribute float_val: %v", err)
	}

	if err := fw.Close(); err != nil {
		log.Fatalf("fw.Close: %v", err)
	}

	// ── Read back ─────────────────────────────────────────────────────────────
	f, err := hdf5.Open(tmpPath)
	if err != nil {
		log.Fatalf("reopen: %v", err)
	}
	defer f.Close()

	var metaDS *hdf5.Dataset
	f.Walk(func(path string, obj hdf5.Object) {
		if path == "/meta" {
			if ds, ok := obj.(*hdf5.Dataset); ok {
				metaDS = ds
			}
		}
	})
	if metaDS == nil {
		fmt.Fprintln(os.Stderr, "FAIL: /meta dataset not found after reopen")
		os.Exit(1)
	}

	rawStr, err := metaDS.ReadAttribute("str_val")
	if err != nil {
		log.Fatalf("ReadAttribute str_val: %v", err)
	}
	rawInt, err := metaDS.ReadAttribute("int_val")
	if err != nil {
		log.Fatalf("ReadAttribute int_val: %v", err)
	}
	rawFloat, err := metaDS.ReadAttribute("float_val")
	if err != nil {
		log.Fatalf("ReadAttribute float_val: %v", err)
	}

	fail := false

	// String check — trim null padding that fixed-length strings may carry.
	sv, ok := rawStr.(string)
	if !ok {
		fmt.Fprintf(os.Stderr, "FAIL: str_val type = %T, want string\n", rawStr)
		fail = true
	} else {
		sv = strings.TrimRight(sv, "\x00")
		if sv != "hello" {
			fmt.Fprintf(os.Stderr, "FAIL: str_val = %q, want \"hello\"\n", sv)
			fail = true
		}
	}

	// Int64 check.
	iv, ok := rawInt.(int64)
	if !ok {
		fmt.Fprintf(os.Stderr, "FAIL: int_val type = %T, want int64\n", rawInt)
		fail = true
	} else if iv != 42 {
		fmt.Fprintf(os.Stderr, "FAIL: int_val = %d, want 42\n", iv)
		fail = true
	}

	// Float64 check.
	fv, ok := rawFloat.(float64)
	if !ok {
		fmt.Fprintf(os.Stderr, "FAIL: float_val type = %T, want float64\n", rawFloat)
		fail = true
	} else if math.Abs(fv-3.14159) > 1e-10 {
		fmt.Fprintf(os.Stderr, "FAIL: float_val = %.10f, want 3.14159\n", fv)
		fail = true
	}

	if fail {
		os.Exit(1)
	}
	fmt.Printf("PASS: string=%q  int64=%d  float64=%v — all attribute types roundtrip correctly\n",
		sv, iv, fv)
}
