package machineconfig_test

import (
	"os"
	"path/filepath"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/extension"
)

// Same digest as nodejs/tests/extension_tables.test.ts and extension-contract's tests.
const extTableDigest = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858"

func extTableTree(t *testing.T) *extension.Tree {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join(fixturesDir(t), "extension_table_sample_tree.json"))
	if err != nil {
		t.Fatal(err)
	}
	tree, err := extension.FromJSON(raw)
	if err != nil {
		t.Fatal(err)
	}
	return tree
}

func TestReadsTableFixturesWrittenByOtherLanguages(t *testing.T) {
	for _, name := range []string{"extension_table_sample_node_v1_1.h5", "extension_table_sample_rust_v1_1.h5"} {
		tree := parseFixture(t, name).Extensions["Private_Ext"]
		if tree == nil {
			t.Fatalf("%s: missing Private_Ext", name)
		}
		if got := extDigest(t, tree); got != extTableDigest {
			t.Errorf("%s: digest %s, want %s", name, got, extTableDigest)
		}
	}
}

func TestV1_1TablesRoundTripPreservesTheDigest(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	cfg.Extensions = map[string]*extension.Tree{"Private_Ext": extTableTree(t)}

	got := writeAndRead(t, cfg).Extensions["Private_Ext"]
	if d := extDigest(t, got); d != extTableDigest {
		t.Fatalf("digest after round trip = %s, want %s", d, extTableDigest)
	}
	mixed := got.Tables["Mixed"]
	if mixed.Fields[4].Dtype != extension.Utf8 || mixed.Fields[4].Size != 8 {
		t.Errorf("label field = %+v", mixed.Fields[4])
	}
	if labels := mixed.Columns["label"].([]string); labels[1] != "\u00b5m" {
		t.Errorf("labels = %q", labels)
	}
	if got.Tables["Empty"].Rows != 0 {
		t.Error("empty table must keep zero rows")
	}
	if got.Tables["Constants"].Attrs["Units"].Value != "V" {
		t.Error("table attributes must survive")
	}
}

func TestATreeWithoutTablesHasNoneAfterARoundTrip(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	tree := extension.NewTree()
	tree.Attrs["A"] = extension.Attr{Dtype: extension.U32, Value: uint64(1)}
	cfg.Extensions = map[string]*extension.Tree{"Private_Ext": tree}
	if n := len(writeAndRead(t, cfg).Extensions["Private_Ext"].Tables); n != 0 {
		t.Fatalf("got %d tables", n)
	}
}

func TestWriterRejectsAnInvalidTableBeforeCreatingTheFile(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	bad := extTableTree(t)
	bad.Tables["Constants"].Columns["name"] = []string{"only-one"}
	cfg.Extensions = map[string]*extension.Tree{"Bad": bad}

	out := filepath.Join(t.TempDir(), "never.h5")
	if err := machineconfig.NewWriter().Write(cfg, out); err == nil {
		t.Fatal("expected an error")
	}
	if _, err := os.Stat(out); err == nil {
		t.Fatal("no file may be created for a rejected config")
	}
}
