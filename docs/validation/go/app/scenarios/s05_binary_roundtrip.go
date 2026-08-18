package scenarios

// S-05: Full binary round-trip with correction hash verification
//
// ID:       S-05
// Action:   Read (with binary) -> write to temp -> read temp -> compare
//           SHA-256 of correction_data / inverse_correction_data.
// Rationale: silent precision loss in binary data is undetectable without a
//           hash comparison.

import (
	"fmt"
	"os"
	"path/filepath"

	mc "machine-config-go"
)

func RunS05BinaryRoundtrip(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	reader := mc.NewReader(path)

	// ParseWithOptions(IncludeBinary: true), not Parse(): Write()ing a config
	// read via plain Parse() would omit the correction grid entirely, since
	// the scalars-only path never loads it.
	cfg, err := reader.ParseWithOptions(mc.ParseOptions{IncludeBinary: true})
	if err != nil {
		return false, fmt.Sprintf("ParseWithOptions failed: %v", err)
	}

	cdBefore, err := reader.GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetCorrectionData failed: %v", err)
	}
	icdBefore, err := reader.GetInverseCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetInverseCorrectionData failed: %v", err)
	}

	tmpPath, err := tempH5("mcl_go_s05")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}

	reader2 := mc.NewReader(tmpPath)
	cdAfter, err := reader2.GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("readback GetCorrectionData failed: %v", err)
	}
	icdAfter, err := reader2.GetInverseCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("readback GetInverseCorrectionData failed: %v", err)
	}

	if !bitwiseEqual(cdBefore.Data, cdAfter.Data) {
		return false, fmt.Sprintf("correction_data mismatch: %s... -> %s...",
			sha256Hex(cdBefore.Data)[:16], sha256Hex(cdAfter.Data)[:16])
	}
	if !bitwiseEqual(icdBefore.Data, icdAfter.Data) {
		return false, "inverse_correction_data mismatch after roundtrip"
	}

	return true, fmt.Sprintf("correction_data preserved: SHA-256=%s", sha256Hex(cdBefore.Data))
}
