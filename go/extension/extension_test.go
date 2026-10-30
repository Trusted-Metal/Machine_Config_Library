package extension_test

import (
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"machine-config-go/extension"
)

const pinnedDigest = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"

func sampleTree() *extension.Tree {
	t := extension.NewTree()
	t.Attrs = map[string]extension.Attr{
		"Schema":     {Dtype: extension.Utf8, Value: "1.2"},
		"Flag":       {Dtype: extension.U32, Value: uint64(1)},
		"Float_Flag": {Dtype: extension.F64, Value: 1.0},
		"Small":      {Dtype: extension.I8, Value: int64(-5)},
		"Wide":       {Dtype: extension.I64, Value: int64(-9007199254740993)},
		"Huge":       {Dtype: extension.U64, Value: uint64(math.MaxUint64)},
		"Ratio":      {Dtype: extension.F32, Value: 0.5},
		"Pair":       {Dtype: extension.F64, Value: []float64{1.5, -2.5}},
		"Ids":        {Dtype: extension.U32, Value: []uint64{1, 2, 3}},
	}
	cond := extension.NewTree()
	cond.Attrs["Value"] = extension.Attr{Dtype: extension.Utf8, Value: "running"}
	job := extension.NewTree()
	job.Attrs["Label"] = extension.Attr{Dtype: extension.Utf8, Value: "Job Start"}
	job.Children["Condition 1"] = cond
	t.Children["Job Start"] = job
	t.Datasets["Grid"] = &extension.Dataset{
		Dtype: extension.F64,
		Shape: []uint64{2, 3},
		Data:  []float64{1, 2, 3, 4, 5, 6.5},
		Attrs: map[string]extension.Attr{"Units": {Dtype: extension.Utf8, Value: "mm"}},
	}
	return t
}

func fixturePath(t *testing.T, name string) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "fixtures", name))
}

func mustDigest(t *testing.T, tree *extension.Tree) string {
	t.Helper()
	d, err := extension.Digest(tree)
	if err != nil {
		t.Fatal(err)
	}
	return d
}

func TestPinnedConformanceDigest(t *testing.T) {
	if got := mustDigest(t, sampleTree()); got != pinnedDigest {
		t.Fatalf("digest = %s, want %s", got, pinnedDigest)
	}
}

func TestDigestTreatsNegativeZeroAndNaNCanonically(t *testing.T) {
	mk := func(v float64) string {
		tree := extension.NewTree()
		tree.Attrs["X"] = extension.Attr{Dtype: extension.F64, Value: v}
		return mustDigest(t, tree)
	}
	if mk(math.Copysign(0, -1)) != mk(0) {
		t.Error("-0 and +0 must hash identically")
	}
	if mk(math.NaN()) != mk(math.Float64frombits(0x7FF8000000000001)) {
		t.Error("all NaN payloads must hash identically")
	}
	if mk(1) == mk(2) {
		t.Error("different values must hash differently")
	}
}

func TestGoldenJSONFixtureHasThePinnedDigest(t *testing.T) {
	raw, err := os.ReadFile(fixturePath(t, "extension_sample_tree.json"))
	if err != nil {
		t.Fatal(err)
	}
	tree, err := extension.FromJSON(raw)
	if err != nil {
		t.Fatal(err)
	}
	if got := mustDigest(t, tree); got != pinnedDigest {
		t.Fatalf("digest = %s, want %s", got, pinnedDigest)
	}
}

func TestJSONRoundTripIsLossless(t *testing.T) {
	tree := sampleTree()
	tree.Attrs["Weird"] = extension.Attr{Dtype: extension.F64, Value: []float64{math.NaN(), math.Inf(1), math.Inf(-1)}}
	b, err := json.Marshal(tree)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(b), `"-9007199254740993"`) || !strings.Contains(string(b), `"18446744073709551615"`) {
		t.Errorf("64-bit integers must be decimal strings: %s", b)
	}
	back, err := extension.FromJSON(b)
	if err != nil {
		t.Fatal(err)
	}
	if mustDigest(t, back) != mustDigest(t, tree) {
		t.Error("digest changed across a JSON round trip")
	}
}

func TestJSONRejectsOutOfRangeAndUnknownDtypes(t *testing.T) {
	cases := map[string]string{
		"out of range":  `{"attrs":{"X":{"dtype":"u8","value":256}}}`,
		"unknown dtype": `{"attrs":{"X":{"dtype":"u128","value":1}}}`,
		"i64 as number": `{"attrs":{"X":{"dtype":"i64","value":1}}}`,
	}
	for name, in := range cases {
		if _, err := extension.FromJSON([]byte(in)); err == nil {
			t.Errorf("%s: expected an error", name)
		}
	}
}

func TestValidationRejectsInvalidTreesWithAPath(t *testing.T) {
	bad := map[string]*extension.Tree{}

	outOfRange := extension.NewTree()
	outOfRange.Attrs["A"] = extension.Attr{Dtype: extension.U8, Value: uint64(300)}
	bad["out of range"] = outOfRange

	wrongType := extension.NewTree()
	wrongType.Attrs["A"] = extension.Attr{Dtype: extension.I32, Value: "x"}
	bad["wrong type"] = wrongType

	unknown := extension.NewTree()
	unknown.Attrs["A"] = extension.Attr{Dtype: "bogus", Value: int64(1)}
	bad["unknown dtype"] = unknown

	emptyName := extension.NewTree()
	emptyName.Attrs[""] = extension.Attr{Dtype: extension.U8, Value: uint64(1)}
	bad["empty name"] = emptyName

	slashName := extension.NewTree()
	slashName.Children["a/b"] = extension.NewTree()
	bad["slash name"] = slashName

	clash := extension.NewTree()
	clash.Children["X"] = extension.NewTree()
	clash.Datasets["X"] = &extension.Dataset{Dtype: extension.F64, Shape: []uint64{1}, Data: []float64{1}}
	bad["dataset/child clash"] = clash

	badShape := extension.NewTree()
	badShape.Datasets["D"] = &extension.Dataset{Dtype: extension.F64, Shape: []uint64{2, 2}, Data: []float64{1, 2, 3}}
	bad["shape mismatch"] = badShape

	for name, tree := range bad {
		err := extension.Validate(tree, "Ext")
		if err == nil {
			t.Errorf("%s: expected a validation error", name)
			continue
		}
		var ce *extension.Error
		if !asError(err, &ce) || !strings.HasPrefix(ce.Path, "Ext") {
			t.Errorf("%s: error should carry a path under Ext, got %v", name, err)
		}
	}

	nested := extension.NewTree()
	inner := extension.NewTree()
	inner.Attrs["A"] = extension.Attr{Dtype: extension.U8, Value: uint64(0)}
	inner.Attrs["B"] = extension.Attr{Dtype: extension.I8, Value: int64(-200)}
	nested.Children["Outer"] = inner
	err := extension.Validate(nested, "Ext")
	var ce *extension.Error
	if !asError(err, &ce) || ce.Path != "Ext/Outer/@B" {
		t.Errorf("nested path = %v, want Ext/Outer/@B", err)
	}
}

func asError(err error, target **extension.Error) bool {
	e, ok := err.(*extension.Error)
	if ok {
		*target = e
	}
	return ok
}
