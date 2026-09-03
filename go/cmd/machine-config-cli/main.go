// CLI entry point: export-json, correction-hash, write-hdf5, copy-hdf5 subcommands.
package main

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"strconv"

	machineconfig "machine-config-go"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("usage: machine-config-cli <export-json|correction-hash|write-hdf5|copy-hdf5> [args]")
	}
	switch args[0] {
	case "export-json":
		return cmdExportJSON(args[1:])
	case "correction-hash":
		return cmdCorrectionHash(args[1:])
	case "write-hdf5":
		return cmdWriteHDF5(args[1:])
	case "copy-hdf5":
		return cmdCopyHDF5(args[1:])
	default:
		return fmt.Errorf("unknown command %q (available: export-json, correction-hash, write-hdf5, copy-hdf5)", args[0])
	}
}

func cmdExportJSON(args []string) error {
	if len(args) < 1 {
		return fmt.Errorf("export-json: usage: export-json <path.h5>")
	}
	cfg, err := machineconfig.NewReader(args[0]).Parse()
	if err != nil {
		return err
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetEscapeHTML(false)
	return enc.Encode(cfg)
}

func cmdCorrectionHash(args []string) error {
	var (
		path    string
		train   = 0
		inverse = false
	)
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--train":
			i++
			if i >= len(args) {
				return fmt.Errorf("--train requires a value")
			}
			n, err := strconv.Atoi(args[i])
			if err != nil {
				return fmt.Errorf("--train: %w", err)
			}
			train = n
		case "--inverse":
			inverse = true
		default:
			if path != "" {
				return fmt.Errorf("unexpected argument %q", args[i])
			}
			path = args[i]
		}
	}
	if path == "" {
		return fmt.Errorf("correction-hash: usage: correction-hash <path.h5> [--train N] [--inverse]")
	}
	reader := machineconfig.NewReader(path)
	var cd *machineconfig.CorrectionData
	var err error
	if inverse {
		cd, err = reader.GetInverseCorrectionData(train)
	} else {
		cd, err = reader.GetCorrectionData(train)
	}
	if err != nil {
		return err
	}
	// Hash as flat little-endian float64 bytes — same convention as Python/Rust/Node.js/C++.
	h := sha256.New()
	buf := make([]byte, 8)
	for _, v := range cd.Data {
		binary.LittleEndian.PutUint64(buf, math.Float64bits(v))
		h.Write(buf)
	}
	fmt.Printf("%x\n", h.Sum(nil))
	return nil
}

func cmdWriteHDF5(args []string) error {
	if len(args) < 2 {
		return fmt.Errorf("write-hdf5: usage: write-hdf5 <input.json|- > <output.h5>")
	}
	var data []byte
	var err error
	if args[0] == "-" {
		data, err = os.ReadFile("/dev/stdin")
		if err != nil {
			// fallback for platforms without /dev/stdin
			data, err = func() ([]byte, error) {
				buf := make([]byte, 0, 1<<20)
				tmp := make([]byte, 4096)
				for {
					n, readErr := os.Stdin.Read(tmp)
					buf = append(buf, tmp[:n]...)
					if readErr != nil {
						break
					}
				}
				return buf, nil
			}()
		}
	} else {
		data, err = os.ReadFile(args[0])
	}
	if err != nil {
		return fmt.Errorf("write-hdf5: read input: %w", err)
	}
	var cfg machineconfig.MachineConfig
	if err := json.Unmarshal(data, &cfg); err != nil {
		return fmt.Errorf("write-hdf5: parse JSON: %w", err)
	}
	return machineconfig.NewWriter().Write(&cfg, args[1])
}

func cmdCopyHDF5(args []string) error {
	if len(args) < 2 {
		return fmt.Errorf("copy-hdf5: usage: copy-hdf5 <src.h5> <dst.h5>")
	}
	cfg, err := machineconfig.NewReader(args[0]).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: true})
	if err != nil {
		return fmt.Errorf("copy-hdf5: read: %w", err)
	}
	return machineconfig.NewWriter().Write(cfg, args[1])
}
