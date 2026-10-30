// Proto — CGo libhdf5 read of root machine_name (dense attrs must work).
//
// Run inside Docker (recommended on Windows):
//   docker run --rm -v "${PWD}:/work" -w /work/go golang:1.24-bookworm \
//     bash -c 'apt-get update -qq && apt-get install -y -qq libhdf5-dev >/dev/null && go run ./proto/proto_cgo_read_root_attrs/'
package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

	"machine-config-go/internal/h5c"
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
	f, err := h5c.Open(path)
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	root, err := f.Root()
	if err != nil {
		log.Fatal(err)
	}
	defer root.Close()

	if !root.HasAttr("machine_name") {
		fmt.Fprintln(os.Stderr, "FAIL: machine_name attribute missing")
		os.Exit(1)
	}
	name, err := root.ReadStringAttr("machine_name")
	if err != nil {
		log.Fatal(err)
	}
	if name == "" {
		fmt.Fprintln(os.Stderr, "FAIL: machine_name empty")
		os.Exit(1)
	}
	fmt.Printf("PASS: machine_name = %q\n", name)
}
