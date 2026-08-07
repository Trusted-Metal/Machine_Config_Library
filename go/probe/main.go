package main

import (
	"fmt"
	"log"
	"os"

	"github.com/scigolib/hdf5"
)

func main() {
	path := "../fixtures/reference_config.h5"
	if len(os.Args) > 1 {
		path = os.Args[1]
	}
	f, err := hdf5.Open(path)
	if err != nil {
		log.Fatalf("open: %v", err)
	}
	defer f.Close()

	fmt.Printf("superblock version: %d\n", f.SuperblockVersion())
	root := f.Root()
	fmt.Printf("root name: %s\n", root.Name())
	fmt.Printf("root children count: %d\n", len(root.Children()))
	attrs, err := root.Attributes()
	fmt.Printf("root.Attributes() err=%v  len=%d\n", err, len(attrs))

	fmt.Println("\nWalking file (groups with attribute counts):")
	f.Walk(func(path string, obj hdf5.Object) {
		switch g := obj.(type) {
		case *hdf5.Group:
			attrs, err := g.Attributes()
			fmt.Printf("  G %-70s  attrs=%d err=%v\n", path, len(attrs), err)
			for _, a := range attrs {
				v, _ := a.ReadValue()
				fmt.Printf("      attr: %-30s = %v\n", a.Name, v)
			}
		case *hdf5.Dataset:
			fmt.Printf("  D %s\n", path)
		}
	})
}
