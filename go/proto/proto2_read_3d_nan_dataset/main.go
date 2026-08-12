//go:build ignore

// Proto 2 — Read a 3-D float64 correction-data dataset from a real fixture.
//
// Validates that scigolib/hdf5 can:
//   1. Navigate a deeply nested HDF5 group hierarchy.
//   2. Read a [257, 257, 2] float64 dataset (131 842 elements).
//   3. Find IEEE-754 NaN values that represent out-of-field positions.
//
// Run from repo root:  go run ./go/proto/proto2_read_3d_nan_dataset/
// Run from go/ dir:    go run ./proto/proto2_read_3d_nan_dataset/
package main

import (
	"fmt"
	"log"
	"math"
	"os"
	"path/filepath"

	"github.com/scigolib/hdf5"
)

const correctionDataPath = "/Machine/Optical_Trains/Optical_Train_01/Optional_Components/ClearBox/Correction_Data"

const expectedElements = 257 * 257 * 2 // 131 842

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
	log.Fatalf("cannot find fixture %q; run from repo root, go/, or proto sub-dir", name)
	return ""
}

func main() {
	path := mustFindFixture("reference_config.h5")
	f, err := hdf5.Open(path)
	if err != nil {
		log.Fatalf("open %s: %v", path, err)
	}
	defer f.Close()

	// Walk the file tree and capture the correction dataset.
	var corrDS *hdf5.Dataset
	f.Walk(func(p string, obj hdf5.Object) {
		if p == correctionDataPath {
			if ds, ok := obj.(*hdf5.Dataset); ok {
				corrDS = ds
			}
		}
	})

	if corrDS == nil {
		// Print what we actually found to aid debugging.
		fmt.Fprintln(os.Stderr, "FAIL: correction data dataset not found; listing datasets found:")
		f.Walk(func(p string, obj hdf5.Object) {
			if _, ok := obj.(*hdf5.Dataset); ok {
				fmt.Fprintf(os.Stderr, "  %s\n", p)
			}
		})
		os.Exit(1)
	}

	data, err := corrDS.Read()
	if err != nil {
		log.Fatalf("ds.Read(): %v", err)
	}

	if len(data) != expectedElements {
		fmt.Fprintf(os.Stderr, "FAIL: expected %d elements, got %d\n", expectedElements, len(data))
		os.Exit(1)
	}

	var nanCount, finiteCount int
	for _, v := range data {
		if math.IsNaN(v) {
			nanCount++
		} else {
			finiteCount++
		}
	}

	if nanCount == 0 {
		fmt.Fprintln(os.Stderr, "FAIL: no NaN values found — correction grid should have out-of-field NaN positions")
		os.Exit(1)
	}

	fmt.Printf("PASS: len=%d  NaN=%d  finite=%d\n", len(data), nanCount, finiteCount)
}
