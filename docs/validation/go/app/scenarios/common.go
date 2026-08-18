package scenarios

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"strings"
)

// bitwiseEqual is NaN-aware equality for correction-grid data — mirrors
// Python's np.array_equal(a, b, equal_nan=True). Plain == on []float64 would
// treat any NaN as unequal to itself, which is wrong here: NaN marks an
// intentional out-of-field cell in the correction grid.
func bitwiseEqual(a, b []float64) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if math.Float64bits(a[i]) != math.Float64bits(b[i]) {
			return false
		}
	}
	return true
}

// sha256Hex hashes flat little-endian float64 bytes — same convention as
// Python/Rust/Node.js/C++ (and go/cmd/machine-config-cli's correction-hash).
func sha256Hex(data []float64) string {
	h := sha256.New()
	buf := make([]byte, 8)
	for _, v := range data {
		binary.LittleEndian.PutUint64(buf, math.Float64bits(v))
		h.Write(buf)
	}
	return hex.EncodeToString(h.Sum(nil))
}

// avFixture resolves a shared, language-agnostic AV fixture at
// docs/validation/fixtures/<name>, relative to the repo root one level up
// from fixturesDir — same convention already used by Rust/Python/Node's AV
// scenario files.
func avFixture(fixturesDir, name string) string {
	// filepath.Join(fixturesDir, "..", ...) instead of filepath.Dir(fixturesDir):
	// when fixturesDir carries a trailing separator (as it does when invoked from
	// go.yml/CI with "$PWD/fixtures/"), filepath.Dir on Windows returns fixturesDir
	// unchanged instead of its parent — Split() sees an empty final element after
	// the trailing separator, so there's nothing to "drop". Join's own Clean()
	// resolves ".." correctly regardless of a trailing separator in the input.
	return filepath.Join(fixturesDir, "..", "docs", "validation", "fixtures", name)
}

// findRealFixture locates the real AconityMIDI fixture under realDir.
func findRealFixture(realDir string) (string, error) {
	entries, err := os.ReadDir(realDir)
	if err != nil {
		return "", fmt.Errorf("ReadDir(%s) failed: %w", realDir, err)
	}
	for _, e := range entries {
		name := e.Name()
		if strings.HasSuffix(name, ".h5") && strings.Contains(name, "AconityMIDI") && strings.Contains(name, "OG_178") {
			return filepath.Join(realDir, name), nil
		}
	}
	return "", fmt.Errorf("real AconityMIDI file not found in %s", realDir)
}

// fmtF64Ptr renders a *float64 for diagnostic messages (nil-safe).
func fmtF64Ptr(p *float64) string {
	if p == nil {
		return "<nil>"
	}
	return fmt.Sprintf("%v", *p)
}

// tempH5 creates an empty temp file with a .h5 suffix and returns its path.
// The caller is responsible for removing it.
func tempH5(prefix string) (string, error) {
	tmp, err := os.CreateTemp("", prefix+"_*.h5")
	if err != nil {
		return "", err
	}
	path := tmp.Name()
	tmp.Close()
	return path, nil
}
