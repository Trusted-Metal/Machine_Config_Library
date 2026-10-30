package models

import (
	"fmt"
	"sort"
	"strings"

	"machine-config-go/extension"
)

// File_Version 1.0 keeps ClearBox per optical train under Optional_Components/ClearBox/; core
// exposes it as the opaque "ClearBox" extension tree, always in the File_Version 1.1 layout, so an
// extension library never has to know which file version a tree came from:
//
//	ClearBox/                      Output_Path, Software_Trigger_Delay   (one shared value each)
//	  <train_id>/                  per-train attributes, correction datasets, Synchronous_Sensors/
//	    Power_Characterization/    structured replacement for Volts_To_Watts_Algorithm/Params
//
// Everything else in a v1.0 ClearBox group is carried verbatim, including the attributes v1.1
// dropped (Selected_Camera, Show_Console, ...), so a v1.0 file round-trips without losing them. The
// two shared values are taken from the first train that has a ClearBox.
//
// Mirrors rust/src/clearbox_legacy.rs.

// ClearBoxExtensionName is the extension name core exposes ClearBox under.
const ClearBoxExtensionName = "ClearBox"

const (
	cbPowerCharacterization = "Power_Characterization"
	cbFlatAlgorithm         = "Volts_To_Watts_Algorithm"
	cbFlatParams            = "Volts_To_Watts_Params"
	cbNameBytes             = 64
)

var (
	cbSharedAttrs   = []string{"Output_Path", "Software_Trigger_Delay"}
	cbV11OnlyAttrs  = []string{"Firmware_Version"}
	cbMissingString = extension.Attr{Dtype: extension.Utf8, Value: ""}
)

// ClearBoxTrain is one optical train's verbatim File_Version 1.0 ClearBox group.
type ClearBoxTrain struct {
	TrainID string
	Tree    *extension.Tree
}

// CloneTree returns a deep copy of t.
func CloneTree(t *extension.Tree) *extension.Tree {
	out := extension.NewTree()
	for k, a := range t.Attrs {
		out.Attrs[k] = cloneAttr(a)
	}
	for k, c := range t.Children {
		out.Children[k] = CloneTree(c)
	}
	for k, d := range t.Datasets {
		nd := &extension.Dataset{Dtype: d.Dtype, Shape: append([]uint64(nil), d.Shape...), Attrs: map[string]extension.Attr{}}
		switch v := d.Data.(type) {
		case []int64:
			nd.Data = append([]int64(nil), v...)
		case []uint64:
			nd.Data = append([]uint64(nil), v...)
		case []float64:
			nd.Data = append([]float64(nil), v...)
		}
		for ak, a := range d.Attrs {
			nd.Attrs[ak] = cloneAttr(a)
		}
		out.Datasets[k] = nd
	}
	for k, tb := range t.Tables {
		nt := &extension.Table{
			Fields:  append([]extension.TableField(nil), tb.Fields...),
			Rows:    tb.Rows,
			Columns: map[string]any{},
			Attrs:   map[string]extension.Attr{},
		}
		for ck, col := range tb.Columns {
			switch v := col.(type) {
			case []string:
				nt.Columns[ck] = append([]string(nil), v...)
			case []int64:
				nt.Columns[ck] = append([]int64(nil), v...)
			case []uint64:
				nt.Columns[ck] = append([]uint64(nil), v...)
			case []float64:
				nt.Columns[ck] = append([]float64(nil), v...)
			}
		}
		for ak, a := range tb.Attrs {
			nt.Attrs[ak] = cloneAttr(a)
		}
		out.Tables[k] = nt
	}
	return out
}

func cloneAttr(a extension.Attr) extension.Attr {
	out := extension.Attr{Dtype: a.Dtype}
	switch v := a.Value.(type) {
	case []string:
		out.Value = append([]string(nil), v...)
	case []int64:
		out.Value = append([]int64(nil), v...)
	case []uint64:
		out.Value = append([]uint64(nil), v...)
	case []float64:
		out.Value = append([]float64(nil), v...)
	default:
		out.Value = a.Value
	}
	return out
}

// ClearBoxText returns the trimmed, non-empty string attribute name of tree, or nil.
func ClearBoxText(tree *extension.Tree, name string) *string {
	a, ok := tree.Attrs[name]
	if !ok {
		return nil
	}
	s, ok := a.Value.(string)
	if !ok {
		return nil
	}
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	return &s
}

func strAttr(v *string) extension.Attr {
	if v == nil {
		return cbMissingString
	}
	return extension.Attr{Dtype: extension.Utf8, Value: *v}
}

func powerCharacterizationToTree(pc *PowerCharacterization) *extension.Tree {
	tree := extension.NewTree()
	var at, ae, it, ud *string
	var constants []EquationConstant
	var points []CalibrationPoint
	if pc != nil {
		at, ae, it, ud = pc.AlgorithmType, pc.AlgorithmEquation, pc.InputType, pc.UnitsDerivedQuantity
		constants, points = pc.DerivationEquationConstants, pc.CharacterizationPoints
	}
	tree.Attrs["Algorithm_Type"] = strAttr(at)
	tree.Attrs["Algorithm_Equation"] = strAttr(ae)
	tree.Attrs["Input_Type"] = strAttr(it)
	tree.Attrs["Units_Derived_Quantity"] = strAttr(ud)

	names := make([]string, len(constants))
	values := make([]float64, len(constants))
	for i, c := range constants {
		names[i], values[i] = c.Name, c.Value
	}
	tree.Tables["Derivation_Equation_Constants"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "name", Dtype: extension.Utf8, Size: cbNameBytes},
			{Name: "value", Dtype: extension.F64},
		},
		Rows:    uint64(len(constants)),
		Columns: map[string]any{"name": names, "value": values},
		Attrs:   map[string]extension.Attr{},
	}
	in := make([]float64, len(points))
	out := make([]float64, len(points))
	for i, p := range points {
		in[i], out[i] = p.InputValue, p.OutputValue
	}
	tree.Tables["Characterization_Points"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "input_value", Dtype: extension.F64},
			{Name: "output_value", Dtype: extension.F64},
		},
		Rows:    uint64(len(points)),
		Columns: map[string]any{"input_value": in, "output_value": out},
		Attrs:   map[string]extension.Attr{},
	}
	return tree
}

// PowerCharacterizationFromTree reads a ClearBox Power_Characterization tree back into the model.
func PowerCharacterizationFromTree(tree *extension.Tree) *PowerCharacterization {
	constants := []EquationConstant{}
	if t, ok := tree.Tables["Derivation_Equation_Constants"]; ok {
		names, _ := t.Columns["name"].([]string)
		values, _ := t.Columns["value"].([]float64)
		for i := 0; i < len(names) && i < len(values); i++ {
			constants = append(constants, EquationConstant{Name: names[i], Value: values[i]})
		}
	}
	points := []CalibrationPoint{}
	if t, ok := tree.Tables["Characterization_Points"]; ok {
		in, _ := t.Columns["input_value"].([]float64)
		out, _ := t.Columns["output_value"].([]float64)
		for i := 0; i < len(in) && i < len(out); i++ {
			points = append(points, CalibrationPoint{InputValue: in[i], OutputValue: out[i]})
		}
	}
	return &PowerCharacterization{
		AlgorithmType:               ClearBoxText(tree, "Algorithm_Type"),
		AlgorithmEquation:           ClearBoxText(tree, "Algorithm_Equation"),
		InputType:                   ClearBoxText(tree, "Input_Type"),
		UnitsDerivedQuantity:        ClearBoxText(tree, "Units_Derived_Quantity"),
		DerivationEquationConstants: constants,
		CharacterizationPoints:      points,
	}
}

// ClearBoxFromV10Trains builds the ClearBox tree from each train's verbatim v1.0 group, in train
// order. It returns nil when no train has a ClearBox.
func ClearBoxFromV10Trains(trains []ClearBoxTrain) (*extension.Tree, error) {
	if len(trains) == 0 {
		return nil, nil
	}
	root := extension.NewTree()
	first := trains[0].Tree
	for _, name := range cbSharedAttrs {
		if a, ok := first.Attrs[name]; ok {
			root.Attrs[name] = a
		}
	}
	for _, t := range trains {
		for _, name := range cbSharedAttrs {
			delete(t.Tree.Attrs, name)
		}
		pc, err := ForwardPowerCharacterizationCoefficients(
			ClearBoxText(t.Tree, cbFlatAlgorithm), ClearBoxText(t.Tree, cbFlatParams))
		if err != nil {
			return nil, fmt.Errorf("%s: %w", t.TrainID, err)
		}
		delete(t.Tree.Attrs, cbFlatAlgorithm)
		delete(t.Tree.Attrs, cbFlatParams)
		if _, clash := t.Tree.Children[cbPowerCharacterization]; clash {
			return nil, fmt.Errorf("%s: a File_Version 1.0 ClearBox group cannot contain %s", t.TrainID, cbPowerCharacterization)
		}
		t.Tree.Children[cbPowerCharacterization] = powerCharacterizationToTree(pc)
		root.Children[t.TrainID] = t.Tree
	}
	return root, nil
}

// ClearBoxToV10Trains splits the ClearBox tree back into one verbatim v1.0 group per train id.
//
// It errors rather than dropping anything v1.0 has no place for: attributes or datasets directly on
// the extension root other than the two shared values.
func ClearBoxToV10Trains(tree *extension.Tree) (map[string]*extension.Tree, error) {
	lossy := func(what string) error {
		return fmt.Errorf("ClearBox: %s has no File_Version 1.0 location", what)
	}
	var rootAttrs []string
	for name := range tree.Attrs {
		shared := false
		for _, s := range cbSharedAttrs {
			if name == s {
				shared = true
			}
		}
		if !shared {
			rootAttrs = append(rootAttrs, name)
		}
	}
	sort.Strings(rootAttrs)
	if len(rootAttrs) > 0 {
		return nil, lossy(fmt.Sprintf("attribute %q", rootAttrs[0]))
	}
	for name := range tree.Datasets {
		return nil, lossy(fmt.Sprintf("dataset %q", name))
	}
	for name := range tree.Tables {
		return nil, lossy(fmt.Sprintf("dataset %q", name))
	}

	out := map[string]*extension.Tree{}
	for trainID, child := range tree.Children {
		group := CloneTree(child)
		for _, name := range cbSharedAttrs {
			if _, own := group.Attrs[name]; !own {
				if a, ok := tree.Attrs[name]; ok {
					group.Attrs[name] = cloneAttr(a)
				}
			}
		}
		for _, name := range cbV11OnlyAttrs {
			delete(group.Attrs, name)
		}
		var pc *PowerCharacterization
		if pcTree, ok := group.Children[cbPowerCharacterization]; ok {
			pc = PowerCharacterizationFromTree(pcTree)
			delete(group.Children, cbPowerCharacterization)
		}
		algorithm, params := BackwardFlatFieldsCoefficients(pc)
		group.Attrs[cbFlatAlgorithm] = strAttr(algorithm)
		group.Attrs[cbFlatParams] = strAttr(params)
		out[trainID] = group
	}
	return out, nil
}
