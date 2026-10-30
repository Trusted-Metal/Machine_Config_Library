//go:build ignore

// Proto 5 — Dense attribute storage audit.
//
// Walks the full reference_config.h5 hierarchy via scigolib and reports
// the attribute count scigolib *actually* reads vs. what h5py reports.
// Groups with >8 attrs are stored in HDF5 dense format; scigolib returns
// 0 for those, confirming the read limitation scope.
//
// Run from repo root:  go run ./go/proto/proto5_dense_attr_audit/
// Run from go/ dir:    go run ./proto/proto5_dense_attr_audit/
package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/scigolib/hdf5"
)

// expected is the attribute count h5py sees for each path (from Python audit).
// Walk gives groups a trailing "/" and a leading "/"; datasets have no trailing "/".
var expected = map[string]int{
	"/":                                                                              9,
	"/Machine/":                                                                      15,
	"/Machine/Optical_Trains/Optical_Train_01/":                                     29,
	"/Machine/Optical_Trains/Optical_Train_02/":                                     29,
	"/Machine/Optical_Trains/Optical_Train_01/Scanner/":                             20,
	"/Machine/Optical_Trains/Optical_Train_02/Scanner/":                             20,
	"/Machine/Optical_Trains/Optical_Train_01/Optional_Components/ClearBox/":        18,
	"/Machine/Optical_Trains/Optical_Train_02/Optional_Components/ClearBox/":        18,
	"/Machine/Optical_Trains/Optical_Train_01/Light_Source/":                        17,
	"/Machine/Optical_Trains/Optical_Train_02/Light_Source/":                        17,
	"/Machine/Optical_Trains/Optical_Train_01/Scanner/X_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_01/Scanner/Y_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_01/Scanner/Z_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_02/Scanner/X_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_02/Scanner/Y_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_02/Scanner/Z_Axis/":                      11,
	"/Machine/Optical_Trains/Optical_Train_01/scan_field_correction_file":           7,
	"/Machine/Optical_Trains/Optical_Train_02/scan_field_correction_file":           7,
	"/Machine/Optical_Trains/Optical_Train_01/Scanner_Card/":                        6,
	"/Machine/Optical_Trains/Optical_Train_02/Scanner_Card/":                        6,
	"/Machine/Optical_Trains/Optical_Train_01/Collimator/":                          5,
	"/Machine/Optical_Trains/Optical_Train_02/Collimator/":                          5,
}

func mustFindFixture(name string) string {
	cwd, _ := os.Getwd()
	candidates := []string{
		filepath.Join(cwd, "fixtures", name),
		filepath.Join(cwd, "..", "fixtures", name),
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			return c
		}
	}
	log.Fatalf("cannot find fixture %q", name)
	return ""
}

type result struct {
	path   string
	expect int
	got    int
}

func main() {
	path := mustFindFixture("reference_config.h5")
	f, err := hdf5.Open(path)
	if err != nil {
		log.Fatalf("open: %v", err)
	}
	defer f.Close()

	results := make([]result, 0, len(expected))

	f.Walk(func(p string, obj hdf5.Object) {
		exp, known := expected[p]
		if !known {
			return
		}
		var got int
		var attrErr error
		switch v := obj.(type) {
		case *hdf5.Group:
			attrs, err := v.Attributes()
			attrErr = err
			got = len(attrs)
		case *hdf5.Dataset:
			attrs, err := v.Attributes()
			attrErr = err
			got = len(attrs)
		}
		if attrErr != nil {
			fmt.Printf("  error at %s: %v\n", p, attrErr)
		} else {
			fmt.Printf("Error showing as nil at %s\n", p)
		}
		results = append(results, result{p, exp, got})
	})

	pass, fail := 0, 0
	fmt.Printf("\n%-65s  %6s  %6s  %s\n", "PATH", "EXPECT", "GOT", "STATUS")
	fmt.Printf("%s\n", "-------------------------------------------------------------------  ------  ------  ------")
	for _, r := range results {
		status := "OK"
		if r.got != r.expect {
			status = "FAIL (dense storage unreadable)"
			fail++
		} else {
			pass++
		}
		fmt.Printf("%-65s  %6d  %6d  %s\n", r.path, r.expect, r.got, status)
	}

	fmt.Printf("\n%d/%d groups readable, %d blocked by dense attribute storage\n", pass, pass+fail, fail)
	if fail > 0 {
		os.Exit(1)
	}
}
