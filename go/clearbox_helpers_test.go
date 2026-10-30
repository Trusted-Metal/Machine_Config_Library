package machineconfig_test

import (
	"fmt"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/extension"
)

// Test helpers for reading ClearBox, which core exposes as an opaque extension tree in its
// File_Version 1.1 layout for both file versions (see internal/models/clearbox_legacy.go).

func cbTrainID(index int) string { return fmt.Sprintf("Optical_Train_%02d", index+1) }

func clearboxRoot(t *testing.T, cfg *machineconfig.MachineConfig) *extension.Tree {
	t.Helper()
	tree := cfg.Extensions["ClearBox"]
	if tree == nil {
		t.Fatal("config has no ClearBox extension")
	}
	return tree
}

func clearboxTrain(t *testing.T, cfg *machineconfig.MachineConfig, index int) *extension.Tree {
	t.Helper()
	child := clearboxRoot(t, cfg).Children[cbTrainID(index)]
	if child == nil {
		t.Fatalf("ClearBox has no %s", cbTrainID(index))
	}
	return child
}

func cbPowerCharacterization(t *testing.T, cfg *machineconfig.MachineConfig, index int) *extension.Tree {
	t.Helper()
	pc := clearboxTrain(t, cfg, index).Children["Power_Characterization"]
	if pc == nil {
		t.Fatalf("%s has no Power_Characterization", cbTrainID(index))
	}
	return pc
}

// cbText returns the string attribute, or "" when absent ("" is also the contract's "absent").
func cbText(tree *extension.Tree, name string) string {
	if a, ok := tree.Attrs[name]; ok {
		if s, ok := a.Value.(string); ok {
			return s
		}
	}
	return ""
}

// cbInt returns an integer attribute (signed or unsigned) and whether it exists.
func cbInt(tree *extension.Tree, name string) (int64, bool) {
	a, ok := tree.Attrs[name]
	if !ok {
		return 0, false
	}
	switch v := a.Value.(type) {
	case int64:
		return v, true
	case uint64:
		return int64(v), true
	}
	return 0, false
}

func cbFloat(tree *extension.Tree, name string) (float64, bool) {
	if a, ok := tree.Attrs[name]; ok {
		if v, ok := a.Value.(float64); ok {
			return v, true
		}
	}
	return 0, false
}

// cbGrid returns one train's forward/inverse correction grid as a flat float64 slice plus its shape.
func cbGrid(t *testing.T, cfg *machineconfig.MachineConfig, index int, name string) ([]float64, []uint64) {
	t.Helper()
	ds := clearboxTrain(t, cfg, index).Datasets[name]
	if ds == nil {
		t.Fatalf("%s has no %s", cbTrainID(index), name)
	}
	data, ok := ds.Data.([]float64)
	if !ok {
		t.Fatalf("%s is not float64", name)
	}
	return data, ds.Shape
}

type cbConstant struct {
	Name  string
	Value float64
}

func cbConstants(pc *extension.Tree) []cbConstant {
	tb := pc.Tables["Derivation_Equation_Constants"]
	if tb == nil {
		return nil
	}
	names, _ := tb.Columns["name"].([]string)
	values, _ := tb.Columns["value"].([]float64)
	out := make([]cbConstant, len(names))
	for i := range names {
		out[i] = cbConstant{Name: names[i], Value: values[i]}
	}
	return out
}

func cbPointCount(pc *extension.Tree) int {
	if tb := pc.Tables["Characterization_Points"]; tb != nil {
		return int(tb.Rows)
	}
	return 0
}

// cbMakePowerCharacterization builds a Power_Characterization tree with the given constants, no points.
func cbMakePowerCharacterization(algorithm string, constants []cbConstant) *extension.Tree {
	tree := extension.NewTree()
	for _, name := range []string{"Algorithm_Equation", "Input_Type", "Units_Derived_Quantity"} {
		tree.Attrs[name] = extension.Attr{Dtype: extension.Utf8, Value: ""}
	}
	tree.Attrs["Algorithm_Type"] = extension.Attr{Dtype: extension.Utf8, Value: algorithm}
	names := make([]string, len(constants))
	values := make([]float64, len(constants))
	for i, c := range constants {
		names[i], values[i] = c.Name, c.Value
	}
	tree.Tables["Derivation_Equation_Constants"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "name", Dtype: extension.Utf8, Size: 64},
			{Name: "value", Dtype: extension.F64},
		},
		Rows:    uint64(len(constants)),
		Columns: map[string]any{"name": names, "value": values},
		Attrs:   map[string]extension.Attr{},
	}
	tree.Tables["Characterization_Points"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "input_value", Dtype: extension.F64},
			{Name: "output_value", Dtype: extension.F64},
		},
		Rows:    0,
		Columns: map[string]any{"input_value": []float64{}, "output_value": []float64{}},
		Attrs:   map[string]extension.Attr{},
	}
	return tree
}
