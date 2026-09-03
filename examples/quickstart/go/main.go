// Machine Config Library — Go Quickstart (read-only)
//
// Requires CGo + libhdf5. From repo root (Docker recommended on Windows):
//
//   docker run --rm -v "${PWD}:/work" -w /work/go golang:1.24-bookworm \
//     bash -c 'apt-get update -qq && apt-get install -y -qq libhdf5-dev pkg-config >/dev/null && \
//              CGO_ENABLED=1 go run ../examples/quickstart/go/'
//
// Opens examples/dummy_2train.h5 via the stable model facade.
// Save is not demonstrated yet (writer is Phase 5.7).

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"machine-config-go/capabilities"
	machineconfig "machine-config-go"
)

func dummyPath() (string, error) {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		return "", fmt.Errorf("runtime.Caller failed")
	}
	// examples/quickstart/go/main.go → repo root
	root := filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", ".."))
	p := filepath.Join(root, "examples", "dummy_2train.h5")
	if _, err := os.Stat(p); err != nil {
		return "", fmt.Errorf("dummy file not found: %s (run: python examples/generate_dummy.py)", p)
	}
	return p, nil
}

func main() {
	path, err := dummyPath()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	f, capErr := capabilities.OpenMachineConfig(path)
	if capErr != nil {
		fmt.Fprintf(os.Stderr, "open failed: %s\n", capErr)
		os.Exit(1)
	}
	defer f.Close()

	fmt.Println("=== Machine Config Quickstart ===")
	fmt.Println()
	fmt.Printf("File version   : %s\n", f.FileVersion())
	meta, capErr := f.GetMeta()
	if capErr != nil {
		fmt.Fprintln(os.Stderr, capErr)
		os.Exit(1)
	}
	fmt.Printf("Machine name   : %s\n", meta.MachineName)
	n, capErr := f.OpticalTrainCount()
	if capErr != nil {
		fmt.Fprintln(os.Stderr, capErr)
		os.Exit(1)
	}
	fmt.Printf("Optical trains : %d\n", n)

	for i := 0; i < n; i++ {
		sc, capErr := f.GetScanner(i)
		if capErr != nil {
			fmt.Fprintln(os.Stderr, capErr)
			os.Exit(1)
		}
		wd := "<nil>"
		if sc.WorkingDistance != nil {
			wd = fmt.Sprintf("%v", *sc.WorkingDistance)
		}
		fmt.Printf("  Train %d  wd=%s %s  offset x=%v, y=%v\n",
			i, wd, ptrStr(sc.WorkingDistanceUnit),
			ptrF(sc.ScanHeadOffsetX), ptrF(sc.ScanHeadOffsetY))
		if _, e := f.GetClearbox(i); e != nil {
			fmt.Printf("           clearbox: %s\n", e.Code)
		} else {
			fmt.Println("           clearbox: present")
		}
	}

	cd, err := machineconfig.NewReader(path).GetCorrectionData(0)
	if err != nil {
		fmt.Fprintf(os.Stderr, "correction: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("Correction grid: %v   (train 0)\n", cd.Shape)

	if n != 2 {
		fmt.Println()
		fmt.Println("FAIL")
		fmt.Printf("  expected 2 optical trains, got %d\n", n)
		os.Exit(1)
	}
	fmt.Println()
	fmt.Println("PASS")
}

func ptrStr(p *string) string {
	if p == nil {
		return ""
	}
	return *p
}

func ptrF(p *float64) string {
	if p == nil {
		return "null"
	}
	return fmt.Sprintf("%v", *p)
}
