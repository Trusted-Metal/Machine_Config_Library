// Proto 1 — Read root-level string attributes from a real h5py-written file.
//
// Validates that scigolib/hdf5 can open fixtures/reference_config.h5 (written
// by Python h5py / real machine software) and retrieve root-group attributes,
// specifically the "machine_name" string attribute that every machine config
// file must contain.
//
// Run from repo root:     go run ./go/proto/proto1_read_root_attrs/
// Run from go/ dir:       go run ./proto/proto1_read_root_attrs/
package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/scigolib/hdf5"
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

	version := f.SuperblockVersion()
	fmt.Printf("PASS: opened %s  (superblock version %d)\n", path, version)
	attrs, err := f.Root().Attributes()
	// if err != nil {
	// 	log.Fatalf("root.Attributes(): %v", err)
	// }
	// if len(attrs) == 0 {
	// 	fmt.Fprintln(os.Stderr, "FAIL: root group has no attributes — check HDF5 format compatibility")
	// 	os.Exit(1)
	// }
	fmt.Println("Root attrs error: ", err, "  len=", len(attrs))
	for _, attr := range attrs {
		fmt.Printf(" %+v\n", attr)
	}

	var machineName string
	for _, attr := range attrs {
		if attr.Name == "machine_name" {
			val, err := attr.ReadValue()
			if err != nil {
				log.Fatalf("ReadValue(machine_name): %v", err)
			}
			if s, ok := val.(string); ok {
				machineName = strings.TrimRight(s, "\x00")
			}
		}
	}

	if machineName == "" {
		fmt.Fprintf(os.Stderr, "FAIL: machine_name attribute missing; found %d attrs:\n", len(attrs))
		for _, a := range attrs {
			v, _ := a.ReadValue()
			fmt.Fprintf(os.Stderr, "  %s = %v\n", a.Name, v)
		}
		os.Exit(1)
	}

	fmt.Printf("PASS: machine_name = %q  (%d root attrs total)\n", machineName, len(attrs))
}
