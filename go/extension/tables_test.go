package extension_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"machine-config-go/extension"
)

// Same tree and digest as nodejs/tests/extension_tables.test.ts and extension-contract's tests.
const tableDigest = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858"

func pointsTable() *extension.Table {
	return &extension.Table{
		Fields: []extension.TableField{
			{Name: "input_value", Dtype: extension.F64},
			{Name: "output_value", Dtype: extension.F64},
		},
		Rows: 2,
		Columns: map[string]any{
			"input_value":  []float64{4, 20},
			"output_value": []float64{-1, 6},
		},
		Attrs: map[string]extension.Attr{},
	}
}

func tableTree() *extension.Tree {
	t := extension.NewTree()
	t.Attrs["Schema"] = extension.Attr{Dtype: extension.Utf8, Value: "1.0"}
	t.Tables["Constants"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "name", Dtype: extension.Utf8, Size: 64},
			{Name: "value", Dtype: extension.F64},
		},
		Rows:    2,
		Columns: map[string]any{"name": []string{"a", "b"}, "value": []float64{0.4375, -2.75}},
		Attrs:   map[string]extension.Attr{"Units": {Dtype: extension.Utf8, Value: "V"}},
	}
	t.Tables["Mixed"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "id", Dtype: extension.U32},
			{Name: "count", Dtype: extension.I64},
			{Name: "ratio", Dtype: extension.F32},
			{Name: "flag", Dtype: extension.U8},
			{Name: "label", Dtype: extension.Utf8, Size: 8},
		},
		Rows: 3,
		Columns: map[string]any{
			"id":    []uint64{1, 2, 4294967295},
			"count": []int64{-9007199254740993, 0, 7},
			"ratio": []float64{0.5, -1.5, 2},
			"flag":  []uint64{0, 1, 1},
			"label": []string{"x", "\u00b5m", ""},
		},
		Attrs: map[string]extension.Attr{},
	}
	t.Tables["Empty"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "input_value", Dtype: extension.F64},
			{Name: "output_value", Dtype: extension.F64},
		},
		Rows:    0,
		Columns: map[string]any{"input_value": []float64{}, "output_value": []float64{}},
		Attrs:   map[string]extension.Attr{},
	}
	child := extension.NewTree()
	child.Tables["Points"] = pointsTable()
	t.Children["Train_1"] = child
	return t
}

func TestContractStaysOnThe01Line(t *testing.T) {
	if !strings.HasPrefix(extension.ContractVersion, "0.1.") {
		t.Fatalf("ContractVersion = %s", extension.ContractVersion)
	}
}

func TestTableDigestIsPinned(t *testing.T) {
	if got := mustDigest(t, tableTree()); got != tableDigest {
		t.Fatalf("digest = %s, want %s", got, tableDigest)
	}
}

func TestEmptyTablesMapDoesNotChangeTheDigest(t *testing.T) {
	tree := extension.NewTree()
	tree.Attrs["A"] = extension.Attr{Dtype: extension.U32, Value: uint64(1)}
	plain := mustDigest(t, tree)
	bare := &extension.Tree{Attrs: tree.Attrs, Children: tree.Children, Datasets: tree.Datasets}
	if got := mustDigest(t, bare); got != plain {
		t.Fatalf("nil tables changed the digest: %s != %s", got, plain)
	}
	if _, ok := tree.ToJSON()["tables"]; ok {
		t.Fatal("tables must be omitted from the JSON when empty")
	}
}

func TestGoldenJSONFixturesKeepTheirDigests(t *testing.T) {
	read := func(name string) *extension.Tree {
		raw, err := os.ReadFile(filepath.Join(filepath.Dir(fixturePath(t, "x")), name))
		if err != nil {
			t.Fatal(err)
		}
		tree, err := extension.FromJSON(raw)
		if err != nil {
			t.Fatal(err)
		}
		return tree
	}
	if got := mustDigest(t, read("extension_sample_tree.json")); got != pinnedDigest {
		t.Fatalf("legacy digest = %s", got)
	}
	if got := mustDigest(t, read("extension_table_sample_tree.json")); got != tableDigest {
		t.Fatalf("table digest = %s, want %s", got, tableDigest)
	}
}

func TestTableJSONRoundTripsLosslessly(t *testing.T) {
	raw, err := json.Marshal(tableTree())
	if err != nil {
		t.Fatal(err)
	}
	var generic map[string]any
	if err := json.Unmarshal(raw, &generic); err != nil {
		t.Fatal(err)
	}
	mixed := generic["tables"].(map[string]any)["Mixed"].(map[string]any)
	counts := mixed["columns"].(map[string]any)["count"].([]any)
	if counts[0] != "-9007199254740993" {
		t.Fatalf("i64 column must be decimal strings, got %v", counts)
	}
	back, err := extension.FromJSON(raw)
	if err != nil {
		t.Fatal(err)
	}
	if got := mustDigest(t, back); got != tableDigest {
		t.Fatalf("digest after JSON round trip = %s", got)
	}
}

func TestDigestIsSensitiveToFieldOrderSizeValuesAndRows(t *testing.T) {
	base := mustDigest(t, tableTree())
	check := func(name string, mutate func(*extension.Tree)) {
		t.Helper()
		tree := tableTree()
		mutate(tree)
		if mustDigest(t, tree) == base {
			t.Errorf("%s did not change the digest", name)
		}
	}
	check("field order", func(tr *extension.Tree) {
		f := tr.Tables["Constants"].Fields
		f[0], f[1] = f[1], f[0]
	})
	check("field size", func(tr *extension.Tree) { tr.Tables["Constants"].Fields[0].Size = 32 })
	check("value", func(tr *extension.Tree) { tr.Tables["Constants"].Columns["value"].([]float64)[1] = -2.5 })
	check("row count", func(tr *extension.Tree) {
		p := tr.Children["Train_1"].Tables["Points"]
		p.Rows = 1
		p.Columns = map[string]any{"input_value": []float64{4}, "output_value": []float64{-1}}
	})
}

func TestValidationRejectsMalformedTablesWithAPath(t *testing.T) {
	cases := []struct {
		name, want string
		mutate     func(*extension.Table)
	}{
		{"no fields", "at least one field", func(tb *extension.Table) { tb.Fields = nil }},
		{"duplicate", "duplicate field", func(tb *extension.Table) { tb.Fields[1].Name = "input_value" }},
		{"rows", "does not match 3 rows", func(tb *extension.Table) { tb.Rows = 3 }},
		{"missing column", "missing column", func(tb *extension.Table) { delete(tb.Columns, "output_value") }},
		{"extra column", "no matching field", func(tb *extension.Table) { tb.Columns["extra"] = []float64{0, 0} }},
		{"size on numeric", "only valid for utf8", func(tb *extension.Table) { tb.Fields[0].Size = 8 }},
		{"utf8 without size", "positive integer size", func(tb *extension.Table) {
			tb.Fields[0] = extension.TableField{Name: "input_value", Dtype: extension.Utf8}
			tb.Columns["input_value"] = []string{"a", "b"}
		}},
		{"too long", "do not fit the 2-byte field", func(tb *extension.Table) {
			tb.Fields[0] = extension.TableField{Name: "input_value", Dtype: extension.Utf8, Size: 2}
			tb.Columns["input_value"] = []string{"abc", "b"}
		}},
		{"wrong shape", "field dtype is f64", func(tb *extension.Table) { tb.Columns["input_value"] = []string{"a", "b"} }},
	}
	for _, c := range cases {
		tb := pointsTable()
		c.mutate(tb)
		tree := extension.NewTree()
		tree.Tables["T"] = tb
		err := extension.Validate(tree, "Ext")
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: error = %v, want it to contain %q", c.name, err, c.want)
		}
	}

	tb := pointsTable()
	tb.Columns["output_value"] = []float64{0}
	tree := extension.NewTree()
	tree.Tables["T"] = tb
	var e *extension.Error
	if err := extension.Validate(tree, "Ext"); err == nil {
		t.Fatal("expected an error")
	} else if ee, ok := err.(*extension.Error); ok {
		e = ee
	}
	if e == nil || e.Path != "Ext/T/output_value" {
		t.Fatalf("path = %+v", e)
	}

	clash := extension.NewTree()
	clash.Children["T"] = extension.NewTree()
	clash.Tables["T"] = pointsTable()
	if err := extension.Validate(clash, "Ext"); err == nil || !strings.Contains(err.Error(), "cannot share a name") {
		t.Fatalf("clash error = %v", err)
	}
}
