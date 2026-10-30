// File_Version 1.1 HDF5 writer — exact inverse of hdf5.go.
//
// Deliberately independent of capabilities/v1_0/hdf5/writer.go — see
// hdf5.go's package doc for the full rationale.
package hdf5

import (
	"sort"
	"fmt"

	"machine-config-go/capabilities/v1_1/layout"
	"machine-config-go/extension"
	"machine-config-go/internal/extio"
	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

// Write serialises cfg to a File_Version 1.1 HDF5 file at path.
//
// ClearBox is an ordinary extension tree here: the shared Output_Path/Software_Trigger_Delay live
// on its root and one subgroup per train beneath it, so every extension is written the same way.
func Write(cfg *MachineConfig, path string) error {
	if err := validateExtensions(cfg); err != nil {
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
		train := &cfg.OpticalTrains[i]
		tid := layout.TrainID(i)
		trainGrp, err := trainsGrp.CreateGroup(tid)
		if err != nil {
			return err
		}
		err = writeTrain(trainGrp, train)
		trainGrp.Close()
		if err != nil {
			return err
		}
	}

	if len(cfg.Extensions) > 0 {
		extGrp, err := root.CreateGroup(layout.RootExtensions)
		if err != nil {
			return err
		}
		defer extGrp.Close()
		for _, name := range sortedExtensionNames(cfg.Extensions) {
			if err := extio.WriteTree(extGrp, name, cfg.Extensions[name]); err != nil {
				return err
			}
		}
	}
	return nil
}

func sortedExtensionNames(m map[string]*extension.Tree) []string {
	names := make([]string, 0, len(m))
	for n := range m {
		names = append(names, n)
	}
	sort.Strings(names)
	return names
}

// validateExtensions runs before the output file is created, so a rejected config never leaves a
// partial file behind.
func validateExtensions(cfg *MachineConfig) error {
	for _, name := range sortedExtensionNames(cfg.Extensions) {
		if err := extension.Validate(cfg.Extensions[name], name); err != nil {
			return err
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Write helpers — exact inverses of the reader helpers
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

// wbIfTrue writes an int64 attribute (1) only when v is true — matches
// v1.0's Scanner Invert_* convention exactly (Change 5 doesn't touch this).
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

// writeAxis: Change 5 — Tuning_Parameters/Tuning_Type are never written in
// v1.1, regardless of what's on the model.
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
	return wf(g, "Smoothing_Parameters", ax.SmoothingParameters)
}

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
// Machine/ — unchanged from v1.0's own shape (no Change touches it)
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
// Machine/Optical_Trains/<tid>/ — Change 1: no Optional_Components/ClearBox
// written here anymore (handled by Write's own loop, under Extensions/).
// ---------------------------------------------------------------------------

func writeTrain(g *h5c.Group, t *OpticalTrain) error {
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

	if t.ScanFieldCorrectionFile != nil {
		if err := writeSFCF(g, t.ScanFieldCorrectionFile); err != nil {
			return err
		}
	}

	return nil
}

// ---------------------------------------------------------------------------
// Scanner/ — unchanged from v1.0 except AxisConfig (writeAxis, Change 5)
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
// Power_Characterization/ — shared shape for Changes 3 and 4. Always writes
// a complete, present group — schema-complete over omitted, even when pc is
// nil or its lists are empty (per the manifest's derivation rules).
// ---------------------------------------------------------------------------

func writePowerCharacterization(g *h5c.Group, pc *PowerCharacterization) error {
	var algType, algEq, inputType, unitsQty *string
	var constants []EquationConstant
	var points []CalibrationPoint
	if pc != nil {
		algType = pc.AlgorithmType
		algEq = pc.AlgorithmEquation
		inputType = pc.InputType
		unitsQty = pc.UnitsDerivedQuantity
		constants = pc.DerivationEquationConstants
		points = pc.CharacterizationPoints
	}
	if err := ws(g, "Algorithm_Type", strOrEmpty(algType)); err != nil {
		return err
	}
	if err := ws(g, "Algorithm_Equation", strOrEmpty(algEq)); err != nil {
		return err
	}
	if err := ws(g, "Input_Type", strOrEmpty(inputType)); err != nil {
		return err
	}
	if err := ws(g, "Units_Derived_Quantity", strOrEmpty(unitsQty)); err != nil {
		return err
	}

	constRows := make([]h5c.EquationConstantRow, len(constants))
	for i, c := range constants {
		constRows[i] = h5c.EquationConstantRow{Name: c.Name, Value: c.Value}
	}
	if err := g.CreateEquationConstantsDataset(layout.DSDerivationEquationConstants, constRows); err != nil {
		return err
	}

	pointRows := make([]h5c.CalibrationPointRow, len(points))
	for i, p := range points {
		pointRows[i] = h5c.CalibrationPointRow{InputValue: p.InputValue, OutputValue: p.OutputValue}
	}
	return g.CreateCalibrationPointsDataset(layout.DSCharacterizationPoints, pointRows)
}

// ---------------------------------------------------------------------------
// Light_Source/ — Change 4: Watts_To_Volts_Algorithm/Params are never
// written in v1.1 (superseded by Power_Characterization).
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

	pc := ls.PowerCharacterization
	pcGrp, err := g.CreateGroup(layout.GroupPowerCharacterization)
	if err != nil {
		return err
	}
	defer pcGrp.Close()
	return writePowerCharacterization(pcGrp, pc)
}

// ---------------------------------------------------------------------------
// Collimator/, Scanner_Card/ — unchanged from v1.0
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
// scan_field_correction_file dataset — unchanged from v1.0
// ---------------------------------------------------------------------------

func writeSFCF(g *h5c.Group, sfcf *ScanFieldCorrectionFile) error {
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
// Extensions/TM_OPCUA/ — Change 2: relocate + rename only; internal
// Client/Pipe/Triggers shape unchanged from v1.0.
// ---------------------------------------------------------------------------

