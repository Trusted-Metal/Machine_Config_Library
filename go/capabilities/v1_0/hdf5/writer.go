// File_Version 1.0 HDF5 writer — exact inverse of hdf5.go.
// Authoritative field-by-field reference: rust/src/capabilities/v1_0/writer.rs
package hdf5

import (
	"fmt"

	"machine-config-go/capabilities/v1_0/layout"
	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

// Write serialises cfg to a File_Version 1.0 HDF5 file at path.
func Write(cfg *MachineConfig, path string) error {
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
		err = writeTrain(trainGrp, &cfg.OpticalTrains[i])
		trainGrp.Close()
		if err != nil {
			return err
		}
	}

	if cfg.Opcua != nil {
		opcuaGrp, err := root.CreateGroup(layout.RootOPCUA)
		if err != nil {
			return err
		}
		err = writeOpcua(opcuaGrp, cfg.Opcua)
		opcuaGrp.Close()
		if err != nil {
			return err
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

// strOrDefault dereferences *string or returns def when nil.
func strOrDefault(s *string, def string) string {
	if s == nil {
		return def
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
	if err := ws(g, "Build_Plate_X_Dimension_unit", strOrDefault(ma.BuildPlateXUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Y_Dimension", ma.BuildPlateY); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Y_Dimension_unit", strOrDefault(ma.BuildPlateYUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Z_Dimension", ma.BuildPlateZ); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Z_Dimension_unit", strOrDefault(ma.BuildPlateZUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Build_Plate_Corner_Radius", ma.BuildPlateRadius); err != nil {
		return err
	}
	if err := ws(g, "Build_Plate_Corner_Radius_unit", strOrDefault(ma.BuildPlateRadiusUnit, "mm")); err != nil {
		return err
	}
	if err := ws(g, "Gas_Flow_Direction", strOrEmpty(ma.GasFlowDirection)); err != nil {
		return err
	}
	return ws(g, "Recoat_Direction", strOrEmpty(ma.RecoatDirection))
}

// ---------------------------------------------------------------------------
// Machine/Optical_Trains/<tid>/
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
	if err := ws(g, "Beam_Waist_Major_unit", strOrDefault(t.BeamWaistMajorUnit, "μm")); err != nil {
		return err
	}
	if err := wf(g, "Beam_Waist_Minor", t.BeamWaistMinor); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Minor_unit", strOrDefault(t.BeamWaistMinorUnit, "μm")); err != nil {
		return err
	}
	if err := wf(g, "Beam_Waist_Offset_Z", t.BeamWaistOffsetZ); err != nil {
		return err
	}
	if err := ws(g, "Beam_Waist_Offset_Z_unit", strOrDefault(t.BeamWaistOffsetZUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Build_Plane_Offset_Major", t.BuildPlaneOffsetMajor); err != nil {
		return err
	}
	if err := ws(g, "Build_Plane_Offset_Major_unit", strOrDefault(t.BuildPlaneOffsetMajorUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Build_Plane_Offset_Minor", t.BuildPlaneOffsetMinor); err != nil {
		return err
	}
	if err := ws(g, "Build_Plane_Offset_Minor_unit", strOrDefault(t.BuildPlaneOffsetMinorUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Collimator_Focal_Length", t.CollimatorFocalLength); err != nil {
		return err
	}
	if err := ws(g, "Collimator_Focal_Length_unit", strOrDefault(t.CollimatorFocalLengthUnit, "mm")); err != nil {
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
	if err := ws(g, "Major_Axis_Angle_unit", strOrDefault(t.MajorAxisAngleUnit, "degrees")); err != nil {
		return err
	}
	if err := wf(g, "Rayleigh_Length_Major", t.RayleighLengthMajor); err != nil {
		return err
	}
	if err := ws(g, "Rayleigh_Length_Major_unit", strOrDefault(t.RayleighLengthMajorUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Rayleigh_Length_Minor", t.RayleighLengthMinor); err != nil {
		return err
	}
	if err := ws(g, "Rayleigh_Length_Minor_unit", strOrDefault(t.RayleighLengthMinorUnit, "mm")); err != nil {
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
	if err := ws(g, "Thermal_Lensing_Focal_Plane_Shift_unit", strOrDefault(t.ThermalLensingFocalPlaneShiftUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Thermal_Lensing_Threshold", t.ThermalLensingThreshold); err != nil {
		return err
	}
	if err := ws(g, "Thermal_Lensing_Threshold_unit", strOrDefault(t.ThermalLensingThresholdUnit, "mm")); err != nil {
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

	if t.OptionalComponents.Clearbox != nil {
		optGrp, err := g.CreateGroup(layout.GroupOptionalComponents)
		if err != nil {
			return err
		}
		defer optGrp.Close()
		cbGrp, err := optGrp.CreateGroup(layout.GroupClearBox)
		if err != nil {
			return err
		}
		defer cbGrp.Close()
		if err := writeClearBox(cbGrp, t.OptionalComponents.Clearbox); err != nil {
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
	if err := ws(g, "Working_Distance_unit", strOrDefault(s.WorkingDistanceUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_X", s.ScanFieldX); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_X_unit", strOrDefault(s.ScanFieldXUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_Y", s.ScanFieldY); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_Y_unit", strOrDefault(s.ScanFieldYUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Field_Size_Z", s.ScanFieldZ); err != nil {
		return err
	}
	if err := ws(g, "Scan_Field_Size_Z_unit", strOrDefault(s.ScanFieldZUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_X", s.ScanHeadOffsetX); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_X_unit", strOrDefault(s.ScanHeadOffsetXUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_Y", s.ScanHeadOffsetY); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_Y_unit", strOrDefault(s.ScanHeadOffsetYUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Offset_Z", s.ScanHeadOffsetZ); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Offset_Z_unit", strOrDefault(s.ScanHeadOffsetZUnit, "mm")); err != nil {
		return err
	}
	if err := wf(g, "Scan_Head_Rotation", s.ScanHeadRotation); err != nil {
		return err
	}
	if err := ws(g, "Scan_Head_Rotation_unit", strOrDefault(s.ScanHeadRotationUnit, "degrees")); err != nil {
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
	if err := ws(g, "Light_Wavelength_unit", strOrDefault(ls.WavelengthUnit, "nm")); err != nil {
		return err
	}
	if err := wf(g, "Power_Max_Nominal", ls.PowerMaxNominal); err != nil {
		return err
	}
	if err := ws(g, "Power_Max_Nominal_unit", strOrDefault(ls.PowerMaxNominalUnit, "W")); err != nil {
		return err
	}
	if err := wf(g, "Power_Max_Actual", ls.PowerMaxActual); err != nil {
		return err
	}
	if err := ws(g, "Power_Max_Actual_unit", strOrDefault(ls.PowerMaxActualUnit, "W")); err != nil {
		return err
	}
	if err := wf(g, "Power_Min_Actual", ls.PowerMinActual); err != nil {
		return err
	}
	if err := ws(g, "Power_Min_Actual_unit", strOrDefault(ls.PowerMinActualUnit, "W")); err != nil {
		return err
	}
	if err := wf(g, "Power_Min_Nominal", ls.PowerMinNominal); err != nil {
		return err
	}
	if err := ws(g, "Power_Min_Nominal_unit", strOrDefault(ls.PowerMinNominalUnit, "W")); err != nil {
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
	if err := ws(g, "Power_Bit_Resolution_unit", strOrDefault(ls.PowerBitResolutionUnit, "bits")); err != nil {
		return err
	}
	if err := ws(g, "Watts_To_Volts_Algorithm", strOrEmpty(ls.WattsToVoltsAlgorithm)); err != nil {
		return err
	}
	return ws(g, "Watts_To_Volts_Params", strOrEmpty(ls.WattsToVoltsParams))
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
	return ws(g, "Focal_Length_unit", strOrDefault(c.FocalLengthUnit, "mm"))
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
	return ws(g, "Sample_Period_unit", strOrDefault(sc.SamplePeriodUnit, "μs"))
}

// ---------------------------------------------------------------------------
// Optional_Components/ClearBox/
// ---------------------------------------------------------------------------

func writeClearBox(g *h5c.Group, cb *ClearBox) error {
	if err := ws(g, "Ip_Address", cb.IPAddress); err != nil {
		return err
	}
	if err := ws(g, "Serial_Number", strOrEmpty(cb.SerialNumber)); err != nil {
		return err
	}
	if err := wi(g, "Data_Port", cb.DataPort); err != nil {
		return err
	}
	if err := wi(g, "Server_Port", cb.ServerPort); err != nil {
		return err
	}
	if err := wi(g, "Actual_Timing_Offset", cb.ActualTimingOffset); err != nil {
		return err
	}
	if err := wi(g, "Commanded_Timing_Offset", cb.CommandedTimingOffset); err != nil {
		return err
	}
	if err := ws(g, "Manufacturer", strOrEmpty(cb.Manufacturer)); err != nil {
		return err
	}
	if err := ws(g, "Model", strOrEmpty(cb.Model)); err != nil {
		return err
	}
	if err := ws(g, "Output_Path", strOrEmpty(cb.OutputPath)); err != nil {
		return err
	}
	if err := ws(g, "Selected_Camera", strOrEmpty(cb.SelectedCamera)); err != nil {
		return err
	}
	if err := ws(g, "Custom_Video_Format", strOrEmpty(cb.CustomVideoFormat)); err != nil {
		return err
	}
	if err := ws(g, "Video_Output", strOrEmpty(cb.VideoOutput)); err != nil {
		return err
	}
	if err := wb(g, "Show_Console", cb.ShowConsole); err != nil {
		return err
	}
	if err := wi(g, "Software_Trigger_Delay", cb.SoftwareTriggerDelay); err != nil {
		return err
	}
	if err := ws(g, "Volts_To_Watts_Algorithm", strOrEmpty(cb.VoltsToWattsAlgorithm)); err != nil {
		return err
	}
	if err := ws(g, "Volts_To_Watts_Params", strOrEmpty(cb.VoltsToWattsParams)); err != nil {
		return err
	}
	if err := ws(g, "Correction_Grid_Domain_Shape", strOrEmpty(cb.CorrectionGridDomainShape)); err != nil {
		return err
	}
	if err := ws(g, "Inverse_Grid_Domain_Shape", strOrEmpty(cb.InverseGridDomainShape)); err != nil {
		return err
	}

	cdFlat := FlatFromNestedGrid(cb.CorrectionData)
	cdDS, err := g.CreateFloat64DatasetOpen(layout.DSCorrectionData, []uint64{257, 257, 2}, cdFlat)
	if err != nil {
		return err
	}
	if err := wsDS(cdDS, "dimensions", "H,W,D"); err != nil {
		cdDS.Close()
		return err
	}
	if err := wsDS(cdDS, "dtype", "float64"); err != nil {
		cdDS.Close()
		return err
	}
	if err := wsDS(cdDS, "shape", "257x257x2"); err != nil {
		cdDS.Close()
		return err
	}
	cdDS.Close()

	icdFlat := FlatFromNestedGrid(cb.InverseCorrectionData)
	icdDS, err := g.CreateFloat64DatasetOpen(layout.DSInverseCorrectionData, []uint64{257, 257, 2}, icdFlat)
	if err != nil {
		return err
	}
	if err := wsDS(icdDS, "dimensions", "H,W,D"); err != nil {
		icdDS.Close()
		return err
	}
	if err := wsDS(icdDS, "dtype", "float64"); err != nil {
		icdDS.Close()
		return err
	}
	if err := wsDS(icdDS, "shape", "257x257x2"); err != nil {
		icdDS.Close()
		return err
	}
	icdDS.Close()

	// Only create the Synchronous_Sensors group at all when the map is
	// non-empty, so a ClearBox with zero sensors is byte-identical on disk
	// to before this field existed — no empty placeholder group. Matches
	// Rust's/Python's/Node's choice, deliberately different from
	// OPCUA/Triggers (always created, even with zero triggers).
	if len(cb.SynchronousSensors) > 0 {
		sensorsGrp, err := g.CreateGroup("Synchronous_Sensors")
		if err != nil {
			return err
		}
		defer sensorsGrp.Close()
		for name, sensor := range cb.SynchronousSensors {
			sGrp, err := sensorsGrp.CreateGroup(name)
			if err != nil {
				return err
			}
			if err := writeSynchronousSensor(sGrp, &sensor); err != nil {
				sGrp.Close()
				return err
			}
			sGrp.Close()
		}
	}

	return nil
}

// writeSynchronousSensor writes one Synchronous Sensor's 18 scalar
// attributes plus its two compound datasets. A zero-length row slice
// produces a valid zero-row dataset (see h5c.CreateEquationConstantsDataset/
// CreateCalibrationPointsDataset), not an absent dataset.
func writeSynchronousSensor(g *h5c.Group, s *SynchronousSensor) error {
	if err := wb(g, "Enabled", s.Enabled); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Name", strOrEmpty(s.SensorName)); err != nil {
		return err
	}
	if err := wf(g, "Sensor_Output_Range_Low", s.SensorOutputRangeLow); err != nil {
		return err
	}
	if err := wf(g, "Sensor_Output_Range_High", s.SensorOutputRangeHigh); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Output_Space", strOrEmpty(s.SensorOutputSpace)); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Model", strOrEmpty(s.SensorModel)); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Manufacturer", strOrEmpty(s.SensorManufacturer)); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Scope", strOrEmpty(s.SensorScope)); err != nil {
		return err
	}
	if err := ws(g, "Units_Derived_Quantity", strOrEmpty(s.UnitsDerivedQuantity)); err != nil {
		return err
	}
	if err := wi(g, "Port_ID", s.PortID); err != nil {
		return err
	}
	if err := ws(g, "Sensor_Type", strOrEmpty(s.SensorType)); err != nil {
		return err
	}
	if err := ws(g, "Input_Type", strOrEmpty(s.InputType)); err != nil {
		return err
	}
	if err := ws(g, "Algorithm_Type", strOrEmpty(s.AlgorithmType)); err != nil {
		return err
	}
	if err := ws(g, "Algorithm_Equation", strOrEmpty(s.AlgorithmEquation)); err != nil {
		return err
	}
	if err := ws(g, "Calibration_Source", strOrEmpty(s.CalibrationSource)); err != nil {
		return err
	}
	if err := wb(g, "Calibration_Verified", s.CalibrationVerified); err != nil {
		return err
	}
	if err := wf(g, "Sample_Period", s.SamplePeriod); err != nil {
		return err
	}
	if err := ws(g, "Metadata", strOrEmpty(s.Metadata)); err != nil {
		return err
	}

	constRows := make([]h5c.EquationConstantRow, len(s.DerivationEquationConstants))
	for i, c := range s.DerivationEquationConstants {
		constRows[i] = h5c.EquationConstantRow{Name: c.Name, Value: c.Value}
	}
	if err := g.CreateEquationConstantsDataset("Derivation_Equation_Constants", constRows); err != nil {
		return err
	}

	pointRows := make([]h5c.CalibrationPointRow, len(s.CalibrationPoints))
	for i, p := range s.CalibrationPoints {
		pointRows[i] = h5c.CalibrationPointRow{InputValue: p.InputValue, OutputValue: p.OutputValue}
	}
	if err := g.CreateCalibrationPointsDataset("Calibration_Points", pointRows); err != nil {
		return err
	}

	return nil
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

func writeOpcua(g *h5c.Group, opcua *OpcuaConfig) error {
	clientGrp, err := g.CreateGroup("Client")
	if err != nil {
		return err
	}
	defer clientGrp.Close()
	c := &opcua.Client
	if err := ws(clientGrp, "Server_URL", c.ServerURL); err != nil {
		return err
	}
	if err := ws(clientGrp, "Auth_Mode", c.AuthMode); err != nil {
		return err
	}
	if err := ws(clientGrp, "Security_Mode", c.SecurityMode); err != nil {
		return err
	}
	if err := ws(clientGrp, "Security_Policy", c.SecurityPolicy); err != nil {
		return err
	}
	if err := clientGrp.WriteInt64Attr("BFS_Max_Depth", int64(c.BfsMaxDepth)); err != nil {
		return err
	}
	if err := clientGrp.WriteInt64Attr("Publish_Interval", int64(c.PublishInterval)); err != nil {
		return err
	}
	if err := clientGrp.WriteInt64Attr("Sampling_Interval", int64(c.SamplingInterval)); err != nil {
		return err
	}
	if err := clientGrp.WriteInt64Attr("Session_Timeout", int64(c.SessionTimeout)); err != nil {
		return err
	}
	if err := wi(clientGrp, "Keep_Alive_Count", c.KeepAliveCount); err != nil {
		return err
	}
	if err := wi(clientGrp, "Lifetime_Count", c.LifetimeCount); err != nil {
		return err
	}
	if err := ws(clientGrp, "Machine_Profile", strOrEmpty(c.MachineProfile)); err != nil {
		return err
	}
	if err := ws(clientGrp, "Queue_Policy", strOrEmpty(c.QueuePolicy)); err != nil {
		return err
	}
	if err := wi(clientGrp, "Queue_Size_Data_Change", c.QueueSizeDataChange); err != nil {
		return err
	}
	if err := wi(clientGrp, "Queue_Size_Events", c.QueueSizeEvents); err != nil {
		return err
	}
	if err := wi(clientGrp, "Reconnect_Interval", c.ReconnectInterval); err != nil {
		return err
	}
	if err := ws(clientGrp, "Root_Node", strOrEmpty(c.RootNode)); err != nil {
		return err
	}
	if err := wi(clientGrp, "Sync_Loop_Interval_Initial", c.SyncLoopIntervalInitial); err != nil {
		return err
	}
	if err := wi(clientGrp, "Sync_Loop_Interval_Settled", c.SyncLoopIntervalSettled); err != nil {
		return err
	}
	for k, v := range c.Extra {
		if err := writeExtra(clientGrp, k, v); err != nil {
			return err
		}
	}

	pipeGrp, err := g.CreateGroup("Pipe")
	if err != nil {
		return err
	}
	defer pipeGrp.Close()
	p := &opcua.Pipe
	var pipeEnabled int64
	if p.PipeEnabled {
		pipeEnabled = 1
	}
	if err := pipeGrp.WriteInt64Attr("Pipe_Enabled", pipeEnabled); err != nil {
		return err
	}
	if err := pipeGrp.WriteInt64Attr("Buffer_Size", int64(p.BufferSize)); err != nil {
		return err
	}
	if err := wb(pipeGrp, "Configure_Client", p.ConfigureClient); err != nil {
		return err
	}
	if err := wi(pipeGrp, "Inbound_Rate_Limit", p.InboundRateLimit); err != nil {
		return err
	}
	if err := wi(pipeGrp, "Max_Inbound_Message_Size", p.MaxInboundMessageSize); err != nil {
		return err
	}
	if err := ws(pipeGrp, "Min_Integrity_Level", strOrEmpty(p.MinIntegrityLevel)); err != nil {
		return err
	}
	if err := ws(pipeGrp, "Pipe_Name", strOrEmpty(p.PipeName)); err != nil {
		return err
	}
	if err := ws(pipeGrp, "User_Access_Level", strOrEmpty(p.UserAccessLevel)); err != nil {
		return err
	}
	for k, v := range p.Extra {
		if err := writeExtra(pipeGrp, k, v); err != nil {
			return err
		}
	}

	triggersGrp, err := g.CreateGroup("Triggers")
	if err != nil {
		return err
	}
	defer triggersGrp.Close()

	// Triggers_Enabled is stored as float64, not int — same as Rust.
	if opcua.TriggersEnabled != nil {
		var te float64
		if *opcua.TriggersEnabled {
			te = 1.0
		}
		if err := triggersGrp.WriteFloat64Attr("Triggers_Enabled", te); err != nil {
			return err
		}
	}
	if err := wi(triggersGrp, "Trigger_Stop_Ceiling_Layers", opcua.TriggerStopCeilingLayers); err != nil {
		return err
	}

	for name, trigger := range opcua.Triggers {
		tGrp, err := triggersGrp.CreateGroup(name)
		if err != nil {
			return err
		}
		if err := ws(tGrp, "ID", strOrEmpty(trigger.ID)); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Signal", strOrEmpty(trigger.Signal)); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Subsystem", strOrEmpty(trigger.Subsystem)); err != nil {
			tGrp.Close()
			return err
		}
		if err := wb(tGrp, "Rule_Enabled", trigger.RuleEnabled); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Start_Value", strOrEmpty(trigger.StartValue)); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Stop_Value", strOrEmpty(trigger.StopValue)); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Case_Sensitivity", strOrEmpty(trigger.CaseSensitivity)); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Component", strOrEmpty(trigger.Component)); err != nil {
			tGrp.Close()
			return err
		}
		if err := wi(tGrp, "Cooldown_Period", trigger.CooldownPeriod); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Event", strOrEmpty(trigger.Event)); err != nil {
			tGrp.Close()
			return err
		}
		if err := wi(tGrp, "Max_Fires_Per_Job", trigger.MaxFiresPerJob); err != nil {
			tGrp.Close()
			return err
		}
		if err := ws(tGrp, "Trigger_Label", strOrEmpty(trigger.TriggerLabel)); err != nil {
			tGrp.Close()
			return err
		}
		for k, v := range trigger.Extra {
			if err := writeExtra(tGrp, k, v); err != nil {
				tGrp.Close()
				return err
			}
		}
		tGrp.Close()
	}
	return nil
}
