// File_Version 1.0 HDF5 writer — exact inverse of hdf5.go.
// Authoritative field-by-field reference: rust/src/capabilities/v1_0/writer.rs
package hdf5

import (
	"fmt"

	"machine-config-go/capabilities/v1_0/layout"
	"machine-config-go/extension"
	"machine-config-go/internal/extio"
	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

// Write serialises cfg to a File_Version 1.0 HDF5 file at path.
func Write(cfg *MachineConfig, path string) error {
	if err := validateExtensions(cfg); err != nil {
		return err
	}
	clearBox, err := clearBoxGroups(cfg)
	if err != nil {
		return err
	}
	f, err := h5c.Create(path)
	if err != nil {
		return fmt.Errorf("create %q: %w", path, err)
	}
	defer f.Close()

	root, err := f.Root()
	if err != nil {
		return err
	}
	defer root.Close()

	if err := writeRootAttrs(root, &cfg.Meta); err != nil {
		return err
	}

	machineGrp, err := root.CreateGroup(layout.RootMachine)
	if err != nil {
		return err
	}
	defer machineGrp.Close()

	if err := writeMachine(machineGrp, &cfg.Machine); err != nil {
		return err
	}

	trainsGrp, err := machineGrp.CreateGroup("Optical_Trains")
	if err != nil {
		return err
	}
	defer trainsGrp.Close()

	for i := range cfg.OpticalTrains {
		trainGrp, err := trainsGrp.CreateGroup(layout.TrainID(i))
		if err != nil {
			return err
		}
		err = writeTrain(trainGrp, &cfg.OpticalTrains[i], clearBox[layout.TrainID(i)])
		trainGrp.Close()
		if err != nil {
			return err
		}
	}

	if tree, ok := cfg.Extensions[layout.LegacyOpcuaExtensionName]; ok {
		if err := extio.WriteTree(root, layout.RootOPCUA, tree); err != nil {
			return err
		}
	}
	return nil
}

// validateExtensions runs before the output file is created, so a rejected config never leaves a
// File_Version 1.0 can only hold the legacy OPCUA/ extension and ClearBox (split per train).
func validateExtensions(cfg *MachineConfig) error {
	for name, tree := range cfg.Extensions {
		if err := extension.Validate(tree, name); err != nil {
			return err
		}
		if name != layout.LegacyOpcuaExtensionName && name != ClearBoxExtensionName {
			return fmt.Errorf("File_Version 1.0 has no Extensions group; cannot write extension %q", name)
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Write helpers — exact inverses of the reader helpers
//
// ws / wf / wi / wb  — operate on a *h5c.Group
// wsDS / wiDS        — operate on a *h5c.Dataset (for SFCF / ClearBox attrs)
// ---------------------------------------------------------------------------

func ws(g *h5c.Group, key, val string) error {
	return g.WriteStringAttr(key, val)
}

// wf writes a float64 attr, or "" when nil.
func wf(g *h5c.Group, key string, v *float64) error {
	if v == nil {
		return g.WriteStringAttr(key, "")
	}
	return g.WriteFloat64Attr(key, *v)
}

// wi writes an int64 attr from *int, or "" when nil.
func wi(g *h5c.Group, key string, v *int) error {
	if v == nil {
		return g.WriteStringAttr(key, "")
	}
	return g.WriteInt64Attr(key, int64(*v))
}

// wb writes 1/0 as int64, or "" when nil.
func wb(g *h5c.Group, key string, v *bool) error {
	if v == nil {
		return g.WriteStringAttr(key, "")
	}
	if *v {
		return g.WriteInt64Attr(key, 1)
	}
	return g.WriteInt64Attr(key, 0)
}

// wbIfTrue writes an int64 attribute (1) only when v is true; writes
// nothing at all when v is false — unlike wb, there is no "absent"
// placeholder written for the false case. Used for Scanner's four
// Invert_* fields, which never appear in any output unless true
// (user-confirmed, 2026-08-21): a write->read round-trip is deliberately
// lossy for an explicit false, which becomes indistinguishable from
// "never set".
func wbIfTrue(g *h5c.Group, key string, v bool) error {
	if !v {
		return nil
	}
	return g.WriteInt64Attr(key, 1)
}

func wsDS(d *h5c.Dataset, key, val string) error {
	return d.WriteStringAttr(key, val)
}

func wiDS(d *h5c.Dataset, key string, val int64) error {
	return d.WriteInt64Attr(key, val)
}

// writeExtra writes an extra map value preserving the natural HDF5 type.
func writeExtra(g *h5c.Group, key string, val any) error {
	switch v := val.(type) {
	case string:
		return g.WriteStringAttr(key, v)
	case float64:
		return g.WriteFloat64Attr(key, v)
	case int64:
		return g.WriteInt64Attr(key, v)
	case int:
		return g.WriteInt64Attr(key, int64(v))
	case bool:
		if v {
			return g.WriteInt64Attr(key, 1)
		}
		return g.WriteInt64Attr(key, 0)
	default:
		return g.WriteStringAttr(key, fmt.Sprintf("%v", v))
	}
}

// writeAxis writes all 11 AxisConfig fields; takes a value (not pointer).
func writeAxis(g *h5c.Group, ax AxisConfig) error {
	if err := wi(g, "Actual_Bit_Resolution", ax.ActualBitResolution); err != nil {
		return err
	}
	if err := ws(g, "Actual_Bit_Resolution_unit", strOrEmpty(ax.ActualBitResolutionUnit)); err != nil {
		return err
	}
	if err := wi(g, "Commanded_Bit_Resolution", ax.CommandedBitResolution); err != nil {
		return err
	}
	if err := ws(g, "Commanded_Bit_Resolution_unit", strOrEmpty(ax.CommandedBitResolutionUnit)); err != nil {
		return err
	}
	if err := ws(g, "Control_Type", strOrEmpty(ax.ControlType)); err != nil {
		return err
	}
	if err := wf(g, "Range_Of_Motion", ax.RangeOfMotion); err != nil {
		return err
	}
	if err := ws(g, "Range_Of_Motion_unit", strOrEmpty(ax.RangeOfMotionUnit)); err != nil {
		return err
	}
	if err := ws(g, "Smoothing_Kernel", strOrEmpty(ax.SmoothingKernel)); err != nil {
		return err
	}
	if err := wf(g, "Smoothing_Parameters", ax.SmoothingParameters); err != nil {
		return err
	}
	if err := ws(g, "Tuning_Parameters", strOrEmpty(ax.TuningParameters)); err != nil {
		return err
	}
	return ws(g, "Tuning_Type", strOrEmpty(ax.TuningType))
}

// strOrEmpty dereferences *string or returns "".
func strOrEmpty(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}

// ---------------------------------------------------------------------------
// Root group → MachineConfigMeta
// ---------------------------------------------------------------------------

func writeRootAttrs(g *h5c.Group, m *MachineConfigMeta) error {
	if err := ws(g, "machine_name", m.MachineName); err != nil {
		return err
	}
	if err := ws(g, "manufacturer", m.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "model", m.Model); err != nil {
		return err
	}
	if err := ws(g, "serial_number", m.SerialNumber); err != nil {
		return err
	}
	if err := ws(g, "File_Version", m.FileVersion); err != nil {
		return err
	}
	if err := ws(g, "Export_Date", m.ExportDate); err != nil {
		return err
	}
	if err := ws(g, "Configuration_Hash", m.ConfigurationHash); err != nil {
		return err
	}
	for k, v := range m.Extra {
		if err := writeExtra(g, k, v); err != nil {
			return err
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Machine/
// ---------------------------------------------------------------------------

func writeMachine(g *h5c.Group, ma *Machine) error {
	if err := ws(g, "ID", strOrEmpty(ma.ID)); err != nil {
		return err
	}
	if err := ws(g, "Machine_Name", ma.MachineName); err != nil {
		return err
	}
	if err := ws(g, "Manufacturer", ma.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "Model", ma.Model); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", ma.SerialNumber); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_X_Dimension", ma.BuildPlateX); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_X_Dimension_unit", strOrEmpty(ma.BuildPlateXUnit)); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Y_Dimension", ma.BuildPlateY); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Y_Dimension_unit", strOrEmpty(ma.BuildPlateYUnit)); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Z_Dimension", ma.BuildPlateZ); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Z_Dimension_unit", strOrEmpty(ma.BuildPlateZUnit)); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Corner_Radius", ma.BuildPlateRadius); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Corner_Radius_unit", strOrEmpty(ma.BuildPlateRadiusUnit)); err != nil {
		return err
	}
	if err := ws(g, "Gas_Flow_Direction", strOrEmpty(ma.GasFlowDirection)); err != nil {
		return err
	}
	if err := ws(g, "Recoat_Direction", strOrEmpty(ma.RecoatDirection)); err != nil {
		return err
	}
	return ws(g, "Recoater_Blade_Type", strOrEmpty(ma.RecoaterBladeType))
}

// ---------------------------------------------------------------------------
// Machine/Optical_Trains/<tid>/
// ---------------------------------------------------------------------------

func writeTrain(g *h5c.Group, t *OpticalTrain, clearBox *extension.Tree) error {
	if err := ws(g, "ID", strOrEmpty(t.ID)); err != nil {
		return err
	}
	if err := ws(g, "Beam_Profile_Type", strOrEmpty(t.BeamProfileType)); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Definition", strOrEmpty(t.BeamWaistDefinition)); err != nil {
		return err
	}
	if err := wf(g, "Beam_Waist_Major", t.BeamWaistMajor); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Major_unit", strOrEmpty(t.BeamWaistMajorUnit)); err != nil {
		return err
	}
	if err := wf(g, "Beam_Waist_Minor", t.BeamWaistMinor); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Minor_unit", strOrEmpty(t.BeamWaistMinorUnit)); err != nil {
		return err
	}
	if err := wf(g, "Beam_Waist_Offset_Z", t.BeamWaistOffsetZ); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Offset_Z_unit", strOrEmpty(t.BeamWaistOffsetZUnit)); err != nil {
		return err
	}
	if err := wf(g, "Build_Plane_Offset_Major", t.BuildPlaneOffsetMajor); err != nil {
		return err
	}
	if err := ws(g, "Build_Plane_Offset_Major_unit", strOrEmpty(t.BuildPlaneOffsetMajorUnit)); err != nil {
		return err
	}
	if err := wf(g, "Build_Plane_Offset_Minor", t.BuildPlaneOffsetMinor); err != nil {
		return err
	}
	if err := ws(g, "Build_Plane_Offset_Minor_unit", strOrEmpty(t.BuildPlaneOffsetMinorUnit)); err != nil {
		return err
	}
	if err := wf(g, "Collimator_Focal_Length", t.CollimatorFocalLength); err != nil {
		return err
	}
	if err := ws(g, "Collimator_Focal_Length_unit", strOrEmpty(t.CollimatorFocalLengthUnit)); err != nil {
		return err
	}
	if err := wf(g, "M2_Major", t.M2Major); err != nil {
		return err
	}
	if err := wf(g, "M2_Minor", t.M2Minor); err != nil {
		return err
	}
	if err := wf(g, "Major_Axis_Angle", t.MajorAxisAngle); err != nil {
		return err
	}
	if err := ws(g, "Major_Axis_Angle_unit", strOrEmpty(t.MajorAxisAngleUnit)); err != nil {
		return err
	}
	if err := wf(g, "Rayleigh_Length_Major", t.RayleighLengthMajor); err != nil {
		return err
	}
	if err := ws(g, "Rayleigh_Length_Major_unit", strOrEmpty(t.RayleighLengthMajorUnit)); err != nil {
		return err
	}
	if err := wf(g, "Rayleigh_Length_Minor", t.RayleighLengthMinor); err != nil {
		return err
	}
	if err := ws(g, "Rayleigh_Length_Minor_unit", strOrEmpty(t.RayleighLengthMinorUnit)); err != nil {
		return err
	}
	if err := ws(g, "Scanner_Number", strOrEmpty(t.ScannerNumber)); err != nil {
		return err
	}
	if err := wb(g, "Thermal_Lensing_Test_Passed", t.ThermalLensingPassed); err != nil {
		return err
	}
	if err := wf(g, "Thermal_Lensing_Focal_Plane_Shift", t.ThermalLensingFocalPlaneShift); err != nil {
		return err
	}
	if err := ws(g, "Thermal_Lensing_Focal_Plane_Shift_unit", strOrEmpty(t.ThermalLensingFocalPlaneShiftUnit)); err != nil {
		return err
	}
	if err := wf(g, "Thermal_Lensing_Threshold", t.ThermalLensingThreshold); err != nil {
		return err
	}
	if err := ws(g, "Thermal_Lensing_Threshold_unit", strOrEmpty(t.ThermalLensingThresholdUnit)); err != nil {
		return err
	}

	scannerGrp, err := g.CreateGroup(layout.GroupScanner)
	if err != nil {
		return err
	}
	defer scannerGrp.Close()
	if err := writeScanner(scannerGrp, &t.Scanner); err != nil {
		return err
	}

	lsGrp, err := g.CreateGroup(layout.GroupLightSource)
	if err != nil {
		return err
	}
	defer lsGrp.Close()
	if err := writeLightSource(lsGrp, &t.LightSource); err != nil {
		return err
	}

	colGrp, err := g.CreateGroup(layout.GroupCollimator)
	if err != nil {
		return err
	}
	defer colGrp.Close()
	if err := writeCollimator(colGrp, &t.Collimator); err != nil {
		return err
	}

	scGrp, err := g.CreateGroup(layout.GroupScannerCard)
	if err != nil {
		return err
	}
	defer scGrp.Close()
	if err := writeScannerCard(scGrp, &t.ScannerCard); err != nil {
		return err
	}

	if clearBox != nil {
		if err := writeClearBoxGroup(g, clearBox); err != nil {
			return err
		}
	}

	if t.ScanFieldCorrectionFile != nil {
		if err := writeSFCF(g, t.ScanFieldCorrectionFile); err != nil {
			return err
		}
	}

	return nil
}

// ---------------------------------------------------------------------------
// Scanner/
// ---------------------------------------------------------------------------

func writeScanner(g *h5c.Group, s *Scanner) error {
	if err := ws(g, "Manufacturer", s.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "Model", s.Model); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", s.SerialNumber); err != nil {
		return err
	}
	if err := wf(g, "Working_Distance", s.WorkingDistance); err != nil {
		return err
	}
	if err := ws(g, "Working_Distance_unit", strOrEmpty(s.WorkingDistanceUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_X", s.ScanFieldX); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_X_unit", strOrEmpty(s.ScanFieldXUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_Y", s.ScanFieldY); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_Y_unit", strOrEmpty(s.ScanFieldYUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_Z", s.ScanFieldZ); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_Z_unit", strOrEmpty(s.ScanFieldZUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_X", s.ScanHeadOffsetX); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_X_unit", strOrEmpty(s.ScanHeadOffsetXUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_Y", s.ScanHeadOffsetY); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_Y_unit", strOrEmpty(s.ScanHeadOffsetYUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_Z", s.ScanHeadOffsetZ); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_Z_unit", strOrEmpty(s.ScanHeadOffsetZUnit)); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Rotation", s.ScanHeadRotation); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Rotation_unit", strOrEmpty(s.ScanHeadRotationUnit)); err != nil {
		return err
	}
	if err := ws(g, "Axis_Configuration", strOrEmpty(s.AxisConfiguration)); err != nil {
		return err
	}
	if err := wbIfTrue(g, "Invert_Actual_X", s.InvertActualX); err != nil {
		return err
	}
	if err := wbIfTrue(g, "Invert_Actual_Y", s.InvertActualY); err != nil {
		return err
	}
	if err := wbIfTrue(g, "Invert_Commanded_X", s.InvertCommandedX); err != nil {
		return err
	}
	if err := wbIfTrue(g, "Invert_Commanded_Y", s.InvertCommandedY); err != nil {
		return err
	}

	// X_Axis and Y_Axis are value types — always written.
	xGrp, err := g.CreateGroup("X_Axis")
	if err != nil {
		return err
	}
	defer xGrp.Close()
	if err := writeAxis(xGrp, s.XAxis); err != nil {
		return err
	}

	yGrp, err := g.CreateGroup("Y_Axis")
	if err != nil {
		return err
	}
	defer yGrp.Close()
	if err := writeAxis(yGrp, s.YAxis); err != nil {
		return err
	}

	// Z_Axis and Focus are pointer types — written only when non-nil.
	if s.ZAxis != nil {
		zGrp, err := g.CreateGroup("Z_Axis")
		if err != nil {
			return err
		}
		defer zGrp.Close()
		if err := writeAxis(zGrp, *s.ZAxis); err != nil {
			return err
		}
	}

	if s.Focus != nil {
		fGrp, err := g.CreateGroup("Focus")
		if err != nil {
			return err
		}
		defer fGrp.Close()
		if err := writeAxis(fGrp, *s.Focus); err != nil {
			return err
		}
	}

	return nil
}

// ---------------------------------------------------------------------------
// Light_Source/
// ---------------------------------------------------------------------------

func writeLightSource(g *h5c.Group, ls *LightSource) error {
	if err := ws(g, "Manufacturer", ls.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "Model", ls.Model); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", ls.SerialNumber); err != nil {
		return err
	}
	if err := wf(g, "Light_Wavelength", ls.Wavelength); err != nil {
		return err
	}
	if err := ws(g, "Light_Wavelength_unit", strOrEmpty(ls.WavelengthUnit)); err != nil {
		return err
	}
	if err := wf(g, "Power_Max_Nominal", ls.PowerMaxNominal); err != nil {
		return err
	}
	if err := ws(g, "Power_Max_Nominal_unit", strOrEmpty(ls.PowerMaxNominalUnit)); err != nil {
		return err
	}
	if err := wf(g, "Power_Max_Actual", ls.PowerMaxActual); err != nil {
		return err
	}
	if err := ws(g, "Power_Max_Actual_unit", strOrEmpty(ls.PowerMaxActualUnit)); err != nil {
		return err
	}
	if err := wf(g, "Power_Min_Actual", ls.PowerMinActual); err != nil {
		return err
	}
	if err := ws(g, "Power_Min_Actual_unit", strOrEmpty(ls.PowerMinActualUnit)); err != nil {
		return err
	}
	if err := wf(g, "Power_Min_Nominal", ls.PowerMinNominal); err != nil {
		return err
	}
	if err := ws(g, "Power_Min_Nominal_unit", strOrEmpty(ls.PowerMinNominalUnit)); err != nil {
		return err
	}
	// Power_Bit_Resolution is always stored as a string in real HDF5 files.
	var pbr string
	if ls.PowerBitResolution != nil {
		pbr = fmt.Sprintf("%g", *ls.PowerBitResolution)
	}
	if err := ws(g, "Power_Bit_Resolution", pbr); err != nil {
		return err
	}
	if err := ws(g, "Power_Bit_Resolution_unit", strOrEmpty(ls.PowerBitResolutionUnit)); err != nil {
		return err
	}
	algorithm, params := BackwardFlatFieldsPoints(ls.PowerCharacterization)
	if err := ws(g, "Watts_To_Volts_Algorithm", strOrEmpty(algorithm)); err != nil {
		return err
	}
	return ws(g, "Watts_To_Volts_Params", strOrEmpty(params))
}

// ---------------------------------------------------------------------------
// Collimator/
// ---------------------------------------------------------------------------

func writeCollimator(g *h5c.Group, c *Collimator) error {
	if err := ws(g, "Manufacturer", c.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "Model", c.Model); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", c.SerialNumber); err != nil {
		return err
	}
	if err := wf(g, "Focal_Length", c.FocalLength); err != nil {
		return err
	}
	return ws(g, "Focal_Length_unit", strOrEmpty(c.FocalLengthUnit))
}

// ---------------------------------------------------------------------------
// Scanner_Card/
// ---------------------------------------------------------------------------

func writeScannerCard(g *h5c.Group, sc *ScannerCard) error {
	if err := ws(g, "Manufacturer", sc.Manufacturer); err != nil {
		return err
	}
	if err := ws(g, "Model", sc.Model); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", sc.SerialNumber); err != nil {
		return err
	}
	if err := ws(g, "Communication_Protocol", strOrEmpty(sc.CommunicationProtocol)); err != nil {
		return err
	}
	if err := wf(g, "Sample_Period", sc.SamplePeriod); err != nil {
		return err
	}
	return ws(g, "Sample_Period_unit", strOrEmpty(sc.SamplePeriodUnit))
}

// ---------------------------------------------------------------------------
// Optional_Components/ClearBox/ — the opaque ClearBox extension tree, split per train
// ---------------------------------------------------------------------------

// clearBoxGroups returns each optical train's verbatim File_Version 1.0 ClearBox group, keyed by
// train id, or nil when cfg has no ClearBox. It runs before the output file is created so a tree
// File_Version 1.0 cannot hold never leaves a partial file behind.
func clearBoxGroups(cfg *MachineConfig) (map[string]*extension.Tree, error) {
	tree, ok := cfg.Extensions[ClearBoxExtensionName]
	if !ok {
		return nil, nil
	}
	groups, err := ClearBoxToV10Trains(tree)
	if err != nil {
		return nil, err
	}
	for id := range groups {
		matched := false
		for i := range cfg.OpticalTrains {
			if layout.TrainID(i) == id {
				matched = true
			}
		}
		if !matched {
			return nil, fmt.Errorf("ClearBox: %s has no matching optical train", id)
		}
	}
	return groups, nil
}

func writeClearBoxGroup(g *h5c.Group, tree *extension.Tree) error {
	optGrp, err := g.CreateGroup(layout.GroupOptionalComponents)
	if err != nil {
		return err
	}
	defer optGrp.Close()
	return extio.WriteTree(optGrp, layout.GroupClearBox, tree)
}

// ---------------------------------------------------------------------------
// scan_field_correction_file dataset
// ---------------------------------------------------------------------------

func writeSFCF(g *h5c.Group, sfcf *ScanFieldCorrectionFile) error {
	// Mirror Rust: fill with zeros (size file_size, min 1) when raw bytes absent.
	data := sfcf.RawBytes
	if data == nil {
		sz := sfcf.FileSize
		if sz < 1 {
			sz = 1
		}
		data = make([]byte, sz)
	}

	ds, err := g.CreateUint8DatasetOpen(layout.DSScanFieldCorrectionFile, data)
	if err != nil {
		return err
	}
	if err := wsDS(ds, "document_name", sfcf.DocumentName); err != nil {
		ds.Close()
		return err
	}
	if err := wsDS(ds, "document_id", sfcf.DocumentID); err != nil {
		ds.Close()
		return err
	}
	if err := wiDS(ds, "file_size", int64(sfcf.FileSize)); err != nil {
		ds.Close()
		return err
	}
	if err := wsDS(ds, "valid_as_of_date", sfcf.ValidAsOfDate); err != nil {
		ds.Close()
		return err
	}
	if err := wsDS(ds, "document_created_at", strOrEmpty(sfcf.DocumentCreatedAt)); err != nil {
		ds.Close()
		return err
	}
	if err := wsDS(ds, "document_type", strOrEmpty(sfcf.DocumentType)); err != nil {
		ds.Close()
		return err
	}
	if err := wsDS(ds, "original_uri", strOrEmpty(sfcf.OriginalURI)); err != nil {
		ds.Close()
		return err
	}
	ds.Close()
	return nil
}

// ---------------------------------------------------------------------------
// OPCUA/
// ---------------------------------------------------------------------------

