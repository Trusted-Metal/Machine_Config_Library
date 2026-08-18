// Package mockv1_1 implements a mock v1.1 adapter — test artifact only, used
// to exercise the change-category architecture (Addition/Removal/Name/Path/
// Name+Path), not a planned schema change. See
// docs/migrations/mock_v1_0_to_v1_1.md for the full manifest.
//
// Design: the reader/writer delegate to the real, public v1.0 adapter
// (machine-config-go/capabilities/v1_0/hdf5) for the whole file, then patch
// exactly the 10 documented differences, rather than reimplementing or
// reaching into v1.0's private parsing internals. "Unchanged" subcomponents
// (light_source, collimator, scanner_card, clearbox, sfcf, opcua) are never
// re-tested here — they're already covered by the existing suite exercising
// the real v1.0 adapter. Mirrors rust/tests/mock_v1_1/mod.rs and
// nodejs/tests/mockV1_1.ts's design exactly.
//
// Lives under go/internal/ (not go/*_test.go directly) so it can be a normal
// importable package with non-test helper functions, while staying entirely
// invisible to real consumers of "machine-config-go" — the root package
// never imports it, mirroring the isolation Rust gets from tests/ being a
// separate integration-test crate.
//
// There is no dispatch-table injection here (unlike Python's _ADAPTERS or
// Node's _READERS/_WRITERS): Go's public dispatcher is a hardcoded switch in
// reader.go/writer.go, not a registry, so the AV-09-11 tests call
// MockV1_1Reader/MockV1_1Writer directly rather than through
// MachineConfigReader/MachineConfigWriter (see VALIDATION_PLAN.md §9.4).
package mockv1_1

import (
	mc "machine-config-go"
	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
	"machine-config-go/capabilities/v1_0/layout"
	"machine-config-go/internal/h5c"
	"strings"
)

// On-disk constants that differ from v1.0.
const (
	FileVersion       = "1.1-mock"
	AttrFacilityID    = "Facility_ID"
	AttrConfigAuthor  = "Config_Author"
	AttrMachineLabel  = "Machine_Label"
	AttrFocalDistance = "Focal_Distance"
	AttrBPWidth       = "Width"
	AttrBPHeight      = "Height"
)

func readOptStr(g *h5c.Group, key string) *string {
	if !g.HasAttr(key) {
		return nil
	}
	s, err := g.ReadStringAttr(key)
	if err != nil {
		return nil
	}
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	return &s
}

func readOptF64(g *h5c.Group, key string) *float64 {
	if !g.HasAttr(key) {
		return nil
	}
	v, err := g.ReadFloat64Attr(key)
	if err != nil {
		return nil
	}
	return &v
}

// renameAttrStr reads oldKey as a string, writes it to newKey, deletes
// oldKey. No-op (beyond the delete attempt) if oldKey is absent.
func renameAttrStr(g *h5c.Group, oldKey, newKey string) error {
	if g.HasAttr(oldKey) {
		if v, err := g.ReadStringAttr(oldKey); err == nil {
			if err := g.WriteStringAttr(newKey, v); err != nil {
				return err
			}
		}
	}
	return g.DeleteAttr(oldKey)
}

// moveAttrF64 reads oldKey (float64) from "from", writes it to newKey on
// "to", deletes oldKey from "from". from and to may be the same group (a
// same-group rename) or different groups (a move).
func moveAttrF64(from, to *h5c.Group, oldKey, newKey string) error {
	if from.HasAttr(oldKey) {
		if v, err := from.ReadFloat64Attr(oldKey); err == nil {
			if err := to.WriteFloat64Attr(newKey, v); err != nil {
				return err
			}
		}
	}
	return from.DeleteAttr(oldKey)
}

// MockV1_1Reader delegates to the real v1.0 parser, then patches the delta.
type MockV1_1Reader struct {
	path string
}

func NewReader(path string) *MockV1_1Reader {
	return &MockV1_1Reader{path: path}
}

func (r *MockV1_1Reader) Parse() (*mc.MachineConfig, error) {
	// Step 1: the real v1.0 parser correctly reads every subcomponent this
	// mock doesn't change. Fields it can't find (renamed, moved, or removed)
	// come back nil/empty — never a panic, since none of the 10 changes
	// remove or rename a *group*, only attributes within one.
	base, err := v1_0hdf5.Parse(r.path, false)
	if err != nil {
		return nil, err
	}

	// Step 2: patch exactly the 10 documented differences by reading their
	// real, mock-v1.1 locations directly.
	f, err := h5c.Open(r.path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()

	machineGrp, err := f.Group(layout.RootMachine)
	if err != nil {
		return nil, err
	}
	defer machineGrp.Close()

	var dimsGrp *h5c.Group
	if machineGrp.LinkExists("Dimensions") {
		dimsGrp, err = machineGrp.OpenGroup("Dimensions")
		if err != nil {
			return nil, err
		}
		defer dimsGrp.Close()
	}

	facilityID := readOptStr(root, AttrFacilityID)
	configAuthor := readOptStr(root, AttrConfigAuthor)
	machineLabel := readOptStr(machineGrp, AttrMachineLabel)

	var buildPlateX, buildPlateY, buildPlateZ, buildPlateRadius *float64
	if dimsGrp != nil {
		buildPlateX = readOptF64(dimsGrp, AttrBPWidth)
		buildPlateY = readOptF64(dimsGrp, AttrBPHeight)
		buildPlateZ = readOptF64(dimsGrp, "Build_Plate_Z_Dimension")
		buildPlateRadius = readOptF64(dimsGrp, "Build_Plate_Corner_Radius")
	}

	// The base parser doesn't know Facility_ID/Config_Author are typed
	// fields, so it swept them into meta.Extra as unknown attrs. Strip them
	// out there before setting the typed fields, or they'd exist in both
	// places — and get written twice, colliding, on the next write. Mirrors
	// the fix already applied in Rust's/Node's mocks for the same bug class.
	delete(base.Meta.Extra, AttrFacilityID)
	delete(base.Meta.Extra, AttrConfigAuthor)
	base.Meta.FacilityID = facilityID
	base.Meta.ConfigAuthor = configAuthor

	if machineLabel != nil {
		base.Machine.MachineName = *machineLabel
	} else {
		base.Machine.MachineName = ""
	}
	base.Machine.BuildPlateX = buildPlateX
	base.Machine.BuildPlateY = buildPlateY
	base.Machine.BuildPlateZ = buildPlateZ
	base.Machine.BuildPlateRadius = buildPlateRadius
	// GasFlowDirection / RecoatDirection: already nil from the base parse
	// (the attrs are genuinely absent) — no patch needed.

	for i := range base.OpticalTrains {
		train := &base.OpticalTrains[i]
		sg, err := f.Group(layout.TrainPathByID(train.TrainID) + "/Scanner")
		if err != nil {
			continue
		}
		train.Scanner.WorkingDistance = readOptF64(sg, AttrFocalDistance)
		sg.Close()
	}

	return base, nil
}

// MockV1_1Writer writes a real v1.0-shaped file, then patches the delta.
type MockV1_1Writer struct {
	config *mc.MachineConfig
}

func NewWriter(config *mc.MachineConfig) *MockV1_1Writer {
	return &MockV1_1Writer{config: config}
}

func (w *MockV1_1Writer) Write(path string) error {
	// Step 1: the real v1.0 writer correctly writes every subcomponent this
	// mock doesn't change, plus File_Version itself (already "1.1-mock" on
	// the input config — the writer just persists whatever string is there).
	if err := v1_0hdf5.Write(w.config, path); err != nil {
		return err
	}

	// Step 2: patch exactly the 10 documented differences in place.
	f, err := h5c.OpenRW(path)
	if err != nil {
		return err
	}
	defer f.Close()

	root, err := f.Root()
	if err != nil {
		return err
	}
	defer root.Close()

	// ADDITION (x2)
	facilityID := ""
	if w.config.Meta.FacilityID != nil {
		facilityID = *w.config.Meta.FacilityID
	}
	if err := root.WriteStringAttr(AttrFacilityID, facilityID); err != nil {
		return err
	}
	configAuthor := ""
	if w.config.Meta.ConfigAuthor != nil {
		configAuthor = *w.config.Meta.ConfigAuthor
	}
	if err := root.WriteStringAttr(AttrConfigAuthor, configAuthor); err != nil {
		return err
	}

	machineGrp, err := f.Group(layout.RootMachine)
	if err != nil {
		return err
	}
	defer machineGrp.Close()

	// REMOVAL (x2)
	if err := machineGrp.DeleteAttr("Gas_Flow_Direction"); err != nil {
		return err
	}
	if err := machineGrp.DeleteAttr("Recoat_Direction"); err != nil {
		return err
	}

	// NAME (Machine_Name -> Machine_Label)
	if err := renameAttrStr(machineGrp, "Machine_Name", AttrMachineLabel); err != nil {
		return err
	}

	// PATH / NAME+PATH: build-plate values move into Machine/Dimensions/.
	// Unit attrs are unaffected — they stay on Machine/ per the manifest.
	dimsGrp, err := machineGrp.CreateGroup("Dimensions")
	if err != nil {
		return err
	}
	defer dimsGrp.Close()
	if err := moveAttrF64(machineGrp, dimsGrp, "Build_Plate_X_Dimension", AttrBPWidth); err != nil {
		return err
	}
	if err := moveAttrF64(machineGrp, dimsGrp, "Build_Plate_Y_Dimension", AttrBPHeight); err != nil {
		return err
	}
	if err := moveAttrF64(machineGrp, dimsGrp, "Build_Plate_Z_Dimension", "Build_Plate_Z_Dimension"); err != nil {
		return err
	}
	if err := moveAttrF64(machineGrp, dimsGrp, "Build_Plate_Corner_Radius", "Build_Plate_Corner_Radius"); err != nil {
		return err
	}

	// NAME (Working_Distance -> Focal_Distance), once per optical train.
	// Uses layout.TrainID(i) directly (the same numbering the real writer
	// just used to create these groups) rather than enumerating group names,
	// since h5c.Group.SubGroupNames() returns all links, not just subgroups.
	for i := range w.config.OpticalTrains {
		tid := layout.TrainID(i)
		sg, err := machineGrp.OpenGroup("Optical_Trains/" + tid + "/Scanner")
		if err != nil {
			continue
		}
		_ = moveAttrF64(sg, sg, "Working_Distance", AttrFocalDistance)
		sg.Close()
	}

	return nil
}

// MakeMockConfig mirrors Python's _make_config() / Node's makeMockConfig /
// Rust's make_mock_config.
func MakeMockConfig(machineName, facilityID, configAuthor *string) *mc.MachineConfig {
	builder := mc.NewMockConfigBuilder()
	builder.NLasers = 1
	builder.BuildPlateX = 250.0
	builder.BuildPlateY = 175.0 // distinct from X so name+path assertions are unambiguous
	if machineName != nil {
		builder.MachineName = *machineName
	} else {
		builder.MachineName = "MigrationTestMachine"
	}

	cfg := builder.Build()
	cfg.Meta.FileVersion = FileVersion
	cfg.Meta.FacilityID = facilityID
	cfg.Meta.ConfigAuthor = configAuthor
	cfg.Machine.GasFlowDirection = nil // absent in v1.1-mock by design
	cfg.Machine.RecoatDirection = nil
	// MockConfigBuilder never sets this (always nil) — give it a real value
	// so the PATH category (Build_Plate_Corner_Radius) has something
	// non-trivial to verify preservation of, mirroring Rust's make_mock_config.
	cfg.Machine.BuildPlateRadius = mc.Float64Ptr(10.0)

	return cfg
}
