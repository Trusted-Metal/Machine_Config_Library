// AV-09-11 architectural guard: no version's adapter — real or mock — may
// import or call into another version's adapter code. Reasoning: if v1.1
// depended on v1.0's code, v1.0 could never be changed or removed later
// without checking v1.1, and every later version would compound the
// problem. See go/internal/mockv1_1's package doc for the full rationale
// and the migration that first violated (then fixed) this rule for the
// mock v1.1 adapter.
//
// This test statically inspects the *import declarations* of every .go file
// under each known version-adapter directory (parsed via go/parser in
// ImportsOnly mode, so prose in doc comments that happens to mention another
// version's package path — as mockv1_1's own doc comments do, by design —
// is never mistaken for an actual import). Any import path naming a
// DIFFERENT version token (vX_Y) than the file's own directory is a
// violation.
//
// Currently only capabilities/v1_0 and internal/mockv1_1 exist, so this can
// only pass trivially today. It starts doing real enforcement the moment a
// real capabilities/v1_1 (or any further version) is added — that's the
// point: a future v1.1 importing v1_0hdf5 (as this file's own history did,
// before the fix) fails this test immediately.
package machineconfig_test

import (
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"testing"
)

// versionToken matches a File_Version-shaped directory/import-path segment:
// "v" + digits + "_" + digits (e.g. "v1_0", "v1_1", "v2_3").
var versionToken = regexp.MustCompile(`v\d+_\d+`)

// versionAdapterDirs lists directories (relative to go/) that hold a single
// File_Version's adapter code, real or mock. Missing directories are
// skipped (e.g. capabilities/v1_1 doesn't exist yet) rather than failing,
// so this test starts enforcing automatically as soon as they're added.
var versionAdapterDirs = []string{
	"capabilities/v1_0",
	"capabilities/v1_1",
	"internal/mockv1_1",
}

func TestVersionAdapterIsolation(t *testing.T) {
	_, thisFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	goDir := filepath.Dir(thisFile)

	checked := 0
	for _, rel := range versionAdapterDirs {
		dir := filepath.Join(goDir, filepath.FromSlash(rel))
		info, err := os.Stat(dir)
		if err != nil || !info.IsDir() {
			continue // not present yet -- nothing to enforce
		}

		own := versionToken.FindString(rel)
		if own == "" {
			t.Fatalf("versionAdapterDirs entry %q names no version token (vX_Y) -- fix the test's own list", rel)
		}

		err = filepath.WalkDir(dir, func(path string, d os.DirEntry, walkErr error) error {
			if walkErr != nil {
				return walkErr
			}
			if d.IsDir() || !strings.HasSuffix(path, ".go") {
				return nil
			}
			checked++

			fset := token.NewFileSet()
			f, perr := parser.ParseFile(fset, path, nil, parser.ImportsOnly)
			if perr != nil {
				return perr
			}
			for _, imp := range f.Imports {
				importPath := strings.Trim(imp.Path.Value, `"`)
				for _, tok := range versionToken.FindAllString(importPath, -1) {
					if tok != own {
						t.Errorf(
							"%s: imports %q (names version %q) but lives under %q (version %q) -- "+
								"no version's adapter may import another version's adapter code",
							path, importPath, tok, rel, own,
						)
					}
				}
			}
			return nil
		})
		if err != nil {
			t.Fatal(err)
		}
	}

	if checked == 0 {
		t.Fatal("no .go files were checked -- versionAdapterDirs paths are likely wrong")
	}
}
