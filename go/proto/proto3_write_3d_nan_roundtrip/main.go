// Proto 3 — Write a 3-D float64 dataset containing NaN, close, reopen, verify.
//
// This is the single most critical proto: it confirms that IEEE-754 NaN values
// survive the full write→close→open→read cycle in scigolib/hdf5.  NaN
// preservation is a hard requirement for the correction-grid data path.
//
// Run from repo root:  go run ./go/proto/proto3_write_3d_nan_roundtrip/
// Run from go/ dir:    go run ./proto/proto3_write_3d_nan_roundtrip/
package main

import (
	"fmt"
	"log"
	"math"
	"os"

	"github.com/scigolib/hdf5"
)

func main() {
	// Create a temporary file that is cleaned up regardless of outcome.
	tmp, err := os.CreateTemp("", "proto3_*.h5")
	if err != nil {
		log.Fatalf("temp file: %v", err)
	}
	tmpPath := tmp.Name()
	tmp.Close()
	defer os.Remove(tmpPath)

	// ── Write ────────────────────────────────────────────────────────────────
	// 3×3×2 float64 grid — 18 elements total.
	// Index 0 and 17 are NaN (simulate out-of-field correction positions).
	writeData := make([]float64, 18)
	for i := range writeData {
		writeData[i] = float64(i) * 0.5 // 0.0, 0.5, 1.0, …, 8.5
	}
	writeData[0] = math.NaN()
	writeData[17] = math.NaN()

	fw, err := hdf5.CreateForWrite(tmpPath, hdf5.CreateTruncate)
	if err != nil {
		log.Fatalf("CreateForWrite: %v", err)
	}
	dw, err := fw.CreateDataset("/grid", hdf5.Float64, []uint64{3, 3, 2})
	if err != nil {
		log.Fatalf("CreateDataset /grid: %v", err)
	}
	if err := dw.Write(writeData); err != nil {
		log.Fatalf("dw.Write: %v", err)
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

	var readData []float64
	f.Walk(func(path string, obj hdf5.Object) {
		if path == "/grid" {
			if ds, ok := obj.(*hdf5.Dataset); ok {
				readData, _ = ds.Read()
			}
		}
	})

	if readData == nil {
		fmt.Fprintln(os.Stderr, "FAIL: /grid dataset not found after reopen")
		os.Exit(1)
	}
	if len(readData) != 18 {
		fmt.Fprintf(os.Stderr, "FAIL: expected 18 elements, got %d\n", len(readData))
		os.Exit(1)
	}

	fail := false
	if !math.IsNaN(readData[0]) {
		fmt.Fprintf(os.Stderr, "FAIL: readData[0] expected NaN, got %v\n", readData[0])
		fail = true
	}
	if !math.IsNaN(readData[17]) {
		fmt.Fprintf(os.Stderr, "FAIL: readData[17] expected NaN, got %v\n", readData[17])
		fail = true
	}
	// Spot-check a finite value: index 2 should be 1.0 (2 * 0.5).
	if math.Abs(readData[2]-1.0) > 1e-12 {
		fmt.Fprintf(os.Stderr, "FAIL: readData[2] expected 1.0, got %v\n", readData[2])
		fail = true
	}
	if fail {
		os.Exit(1)
	}

	fmt.Println("PASS: IEEE-754 NaN preserved in 3-D float64 write→close→open→read roundtrip")
}
