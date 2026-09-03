package main

import (
	"fmt"
	"os"

	"mcl_go_validation/scenarios"
)

type runFn func(fixturesDir, realDir string) (bool, string)

type scenario struct {
	id  string
	run runFn
}

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "Usage: mcl_go_validation <fixtures_dir> <real_dir>")
		os.Exit(1)
	}
	fixturesDir := os.Args[1]
	realDir := os.Args[2]

	list := []scenario{
		{"S-01", scenarios.RunS01ReadScalars},
		{"S-02", scenarios.RunS02ReadBinary},
		{"S-03", scenarios.RunS03ReadReal},
		{"S-04", scenarios.RunS04WriteModify},
		{"S-05", scenarios.RunS05BinaryRoundtrip},
		{"S-06", scenarios.RunS06Builder},
		{"S-07", scenarios.RunS07Opcua},
		{"S-08", scenarios.RunS08DrasticChange},
		{"S-09", scenarios.RunS09TypeExports},
		{"S-10", scenarios.RunS10FacadeExportSurface},
		{"AV-01", scenarios.RunAv01UnknownVersion},
		{"AV-02", scenarios.RunAv02MissingVersion},
		{"AV-03", scenarios.RunAv03FutureVersion},
		{"AV-04", scenarios.RunAv04MissingGroup},
		{"AV-05", scenarios.RunAv05CorruptScalar},
		{"AV-06", scenarios.RunAv06WhitespaceVersion},
		{"AV-07", scenarios.RunAv07EmptyVersion},
		{"AV-08", scenarios.RunAv08VersionFidelity},
		{"AV-12", scenarios.RunAv12OpcuaFacadeOk},
		{"AV-13", scenarios.RunAv13OpcuaFacadeValidation},
	}

	passed, failed := 0, 0
	for _, sc := range list {
		ok, detail := sc.run(fixturesDir, realDir)
		if ok {
			fmt.Printf("[PASS] %s: %s\n", sc.id, detail)
			passed++
		} else {
			fmt.Printf("[FAIL] %s: %s\n", sc.id, detail)
			failed++
		}
	}

	fmt.Printf("\n%d scenarios: %d passed, %d failed\n", passed+failed, passed, failed)
	if failed != 0 {
		os.Exit(1)
	}
}
