package machineconfig_test

import (
	"os"
	"path/filepath"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/extension"
	"machine-config-go/internal/extio"
	"machine-config-go/internal/h5c"
)

const extPinnedDigest = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"

func extSampleTree(t *testing.T) *extension.Tree {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join(fixturesDir(t), "extension_sample_tree.json"))
	if err != nil {
		t.Fatal(err)
	}
	tree, err := extension.FromJSON(raw)
	if err != nil {
		t.Fatal(err)
	}
	return tree
}

func extDigest(t *testing.T, tree *extension.Tree) string {
	t.Helper()
	d, err := extension.Digest(tree)
	if err != nil {
		t.Fatal(err)
	}
	return d
}

func parseFixture(t *testing.T, name string) *machineconfig.MachineConfig {
	t.Helper()
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), name)).Parse()
	if err != nil {
		t.Fatal(err)
	}
	return cfg
}

func TestOpcuaGroupIsAnOpaqueExtension(t *testing.T) {
	if got := parseFixture(t, "reference_config.h5").Extensions; len(got) != 1 || got["ClearBox"] == nil {
		t.Fatalf("reference config must carry exactly the ClearBox extension, got %v", got)
	}
	cfg := parseFixture(t, "reference_config_opcua.h5")
	if len(cfg.Extensions) != 2 || cfg.Extensions["ClearBox"] == nil || cfg.Extensions["TM_OPCUA"] == nil {
		t.Fatalf("expected exactly ClearBox and TM_OPCUA, got %v", cfg.Extensions)
	}
	tree := cfg.Extensions["TM_OPCUA"]
	for _, child := range []string{"Client", "Pipe", "Triggers"} {
		if tree.Children[child] == nil {
			t.Errorf("missing child group %q", child)
		}
	}
	url := tree.Children["Client"].Attrs["Server_URL"]
	if url.Dtype != extension.Utf8 || url.Value != "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer" {
		t.Errorf("Server_URL = %+v", url)
	}
	if pi := tree.Children["Client"].Attrs["Publish_Interval"]; pi.Value == nil {
		t.Error("Publish_Interval missing")
	}
	trig := tree.Children["Triggers"]
	if trig.Children["Laser Emission Interlock"] == nil || trig.Children["Chamber Oxygen Level"] == nil {
		t.Error("trigger groups must be children of Triggers")
	}
}

func TestReadsExtensionFixturesWrittenByOtherLanguages(t *testing.T) {
	for _, name := range []string{"extension_sample_node_v1_1.h5", "extension_sample_rust_v1_1.h5"} {
		cfg := parseFixture(t, name)
		tree := cfg.Extensions["Private_Ext"]
		if tree == nil {
			t.Fatalf("%s: missing Private_Ext", name)
		}
		if got := extDigest(t, tree); got != extPinnedDigest {
			t.Errorf("%s: digest %s, want %s", name, got, extPinnedDigest)
		}
	}
}

func TestV1_1ExtensionRoundTripPreservesDigest(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	if _, ok := cfg.Extensions["ClearBox"]; !ok {
		t.Fatal("ClearBox is an ordinary extension in File_Version 1.1")
	}
	cfg.Extensions = map[string]*extension.Tree{"Private_Ext": extSampleTree(t)}

	rt := writeAndRead(t, cfg)
	if got := extDigest(t, rt.Extensions["Private_Ext"]); got != extPinnedDigest {
		t.Fatalf("digest after round trip = %s, want %s", got, extPinnedDigest)
	}
}

func TestWriterRejectsInvalidExtensionBeforeCreatingTheFile(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	bad := extension.NewTree()
	bad.Attrs["A"] = extension.Attr{Dtype: extension.U8, Value: uint64(999)}
	cfg.Extensions = map[string]*extension.Tree{"Bad": bad}

	out := filepath.Join(t.TempDir(), "never.h5")
	if err := machineconfig.NewWriter().Write(cfg, out); err == nil {
		t.Fatal("expected an error")
	}
	if _, err := os.Stat(out); err == nil {
		t.Fatal("no file may be created for a rejected config")
	}
}

func TestWriterAcceptsAClearBoxExtensionTree(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	tree := extension.NewTree()
	tree.Attrs["Output_Path"] = extension.Attr{Dtype: extension.Utf8, Value: "x"}
	cfg.Extensions = map[string]*extension.Tree{"ClearBox": tree}
	rt := writeAndRead(t, cfg)
	if got := cbText(rt.Extensions["ClearBox"], "Output_Path"); got != "x" {
		t.Fatalf("Output_Path = %q, want x", got)
	}
}

func TestV1_0OpcuaGroupRoundTripsAsTmOpcua(t *testing.T) {
	cfg := parseFixture(t, "reference_config_opcua.h5")
	want := extDigest(t, cfg.Extensions["TM_OPCUA"])
	rt := writeAndRead(t, cfg)
	if got := extDigest(t, rt.Extensions["TM_OPCUA"]); got != want {
		t.Fatalf("digest changed across a v1.0 round trip: %s != %s", got, want)
	}
}

func TestV1_0CannotHoldOtherExtensions(t *testing.T) {
	cfg := parseFixture(t, "reference_config.h5")
	cfg.Extensions = map[string]*extension.Tree{"Private_Ext": extSampleTree(t)}
	if err := machineconfig.NewWriter().Write(cfg, filepath.Join(t.TempDir(), "never.h5")); err == nil {
		t.Fatal("expected an error: File_Version 1.0 has no Extensions group")
	}
}

func TestV1_0OpcuaGroupMigratesToAV1_1Extension(t *testing.T) {
	cfg := parseFixture(t, "reference_config_opcua.h5")
	want := extDigest(t, cfg.Extensions["TM_OPCUA"])
	cfg.Meta.FileVersion = "1.1"
	rt := writeAndRead(t, cfg)
	if got := extDigest(t, rt.Extensions["TM_OPCUA"]); got != want {
		t.Fatalf("digest changed across migration: %s != %s", got, want)
	}
}

func TestHashUnchangedWithoutExtensionsAndChangesWithThem(t *testing.T) {
	cfg := parseFixture(t, "reference_config_v1_1.h5")
	base, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}

	cfg.Extensions["Private_Ext"] = extSampleTree(t)
	withExt, _ := machineconfig.ComputeConfigurationHash(cfg)
	if withExt == base {
		t.Error("adding an extension must change the hash")
	}

	cfg.Extensions["Private_Ext"].Attrs["Flag"] = extension.Attr{Dtype: extension.U32, Value: uint64(2)}
	edited, _ := machineconfig.ComputeConfigurationHash(cfg)
	if edited == withExt {
		t.Error("editing an extension must change the hash")
	}

	delete(cfg.Extensions, "Private_Ext")
	cleared, _ := machineconfig.ComputeConfigurationHash(cfg)
	if cleared != base {
		t.Error("a config without Private_Ext must hash as it did before")
	}
}

func TestExtensionIOHandlesAllDtypesAndShapes(t *testing.T) {
	tree := extension.NewTree()
	tree.Attrs["EmptyList"] = extension.Attr{Dtype: extension.I32, Value: []int64{}}
	tree.Attrs["Strings"] = extension.Attr{Dtype: extension.Utf8, Value: []string{"a", "bé", ""}}
	for dtype, v := range map[string]any{
		extension.I8: int64(-8), extension.I16: int64(-16), extension.I32: int64(-32),
		extension.U8: uint64(8), extension.U16: uint64(16), extension.U32: uint64(32),
		extension.F32: 0.25,
	} {
		tree.Attrs["S_"+dtype] = extension.Attr{Dtype: dtype, Value: v}
	}
	tree.Datasets["Scalar"] = &extension.Dataset{Dtype: extension.I16, Shape: nil, Data: []int64{-3}}
	tree.Datasets["Cube"] = &extension.Dataset{
		Dtype: extension.U8, Shape: []uint64{2, 2, 2}, Data: []uint64{1, 2, 3, 4, 5, 6, 7, 8},
	}
	want := extDigest(t, tree)

	path := filepath.Join(t.TempDir(), "io.h5")
	f, err := h5c.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, _ := f.Root()
	if err := extio.WriteTree(root, "T", tree); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := h5c.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	g, err := f2.Group("T")
	if err != nil {
		t.Fatal(err)
	}
	defer g.Close()
	back, err := extio.ReadTree(g, "T")
	if err != nil {
		t.Fatal(err)
	}
	if got := extDigest(t, back); got != want {
		t.Fatalf("digest after HDF5 round trip = %s, want %s", got, want)
	}
}
