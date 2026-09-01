// Package hdf5 is the File_Version 1.1 HDF5 reader (and, in writer.go /
// migrate.go, writer and v1.0<->v1.1 migration logic).
//
// Deliberately independent of capabilities/v1_0/hdf5 — every helper used
// here is defined fresh in this package (helpers.go), even where the
// on-disk shape happens to match v1.0 today (Collimator, ScannerCard,
// ScanFieldCorrectionFile, OPCUA's inner Client/Pipe/Triggers shape,
// SynchronousSensor). v1.0 could change or be removed later without
// affecting v1.1. See go/version_adapter_isolation_test.go for the
// automated guard and docs/migrations/v1_0_to_v1_1.md for the full manifest
// this implements.
package hdf5

import (
	"fmt"
	"strings"

	"machine-config-go/capabilities/v1_1/layout"
	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

const schemaVersion = "v1"

// Adapter is the File_Version 1.1 HDF5 reader.
type Adapter struct {
	path string
}

// Parse reads path as File_Version 1.1.
func Parse(path string, includeBinary bool) (*MachineConfig, error) {
	r := &Adapter{path: path}
	return r.parseWithOptions(includeBinary)
}

// GetCorrectionData loads the ClearBox Correction_Data grid for train index.
func GetCorrectionData(path string, trainIndex int) (*CorrectionData, error) {
	r := &Adapter{path: path}
	return r.GetCorrectionData(trainIndex)
}

// GetInverseCorrectionData loads the ClearBox Inverse_Correction_Data grid for train index.
func GetInverseCorrectionData(path string, trainIndex int) (*CorrectionData, error) {
	r := &Adapter{path: path}
	return r.GetInverseCorrectionData(trainIndex)
}

func (r *Adapter) parseWithOptions(includeBinary bool) (*MachineConfig, error) {
	f, err := h5c.Open(r.path)
	if err != nil {
		return nil, fmt.Errorf("open %q: %w", r.path, err)
	}
	defer f.Close()
	return r.parse(f, includeBinary)
}

func (r *Adapter) parse(f *h5c.File, includeBinary bool) (*MachineConfig, error) {
	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()

	fv := readRequiredStr(root, "File_Version")
	if fv == "" {
		fv = layout.FileVersion
	}

	metaKnownKeys := map[string]bool{
		"File_Version": true, "machine_name": true, "manufacturer": true,
		"model": true, "serial_number": true, "Export_Date": true,
		"Configuration_Hash": true,
	}
	meta := MachineConfigMeta{
		SchemaVersion:     schemaVersion,
		MachineName:       readRequiredStr(root, "machine_name"),
		Manufacturer:      readRequiredStr(root, "manufacturer"),
		Model:             readRequiredStr(root, "model"),
		SerialNumber:      readRequiredStr(root, "serial_number"),
		FileVersion:       fv,
		ExportDate:        readRequiredStr(root, "Export_Date"),
		ConfigurationHash: readRequiredStr(root, "Configuration_Hash"),
		Extra:             readGroupExtras(root, metaKnownKeys),
	}

	machineGrp, err := f.Group(layout.RootMachine)
	if err != nil {
		return nil, err
	}
	defer machineGrp.Close()

	machine, err := parseMachine(machineGrp)
	if err != nil {
		return nil, err
	}

	// Change 1 (Consolidate): read the one shared Output_Path/
	// Software_Trigger_Delay value up front, if ClearBox exists at all.
	hasClearboxRoot := root.LinkExists(layout.GroupClearBoxRoot)
	var sharedOutputPath *string
	var sharedSoftwareTriggerDelay *int
	if hasClearboxRoot {
		cbRoot, err := f.Group(layout.GroupClearBoxRoot)
		if err != nil {
			return nil, err
		}
		sharedOutputPath = readStrAttr(cbRoot, "Output_Path")
		sharedSoftwareTriggerDelay, err = readIntAttr(cbRoot, "Software_Trigger_Delay")
		cbRoot.Close()
		if err != nil {
			return nil, err
		}
	}

	trainsGrp, err := f.Group(layout.RootOpticalTrains)
	if err != nil {
		return nil, err
	}
	defer trainsGrp.Close()

	var trains []OpticalTrain
	for i := 1; ; i++ {
		tid := layout.TrainID(i - 1)
		if !trainsGrp.LinkExists(tid) {
			break
		}
		train, err := parseTrain(f, root, tid, hasClearboxRoot, sharedOutputPath, sharedSoftwareTriggerDelay, includeBinary)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", tid, err)
		}
		trains = append(trains, train)
	}

	opcua, err := parseOpcua(f, root)
	if err != nil {
		return nil, err
	}

	return &MachineConfig{
		Meta:          meta,
		Machine:       machine,
		OpticalTrains: trains,
		Opcua:         opcua,
	}, nil
}

func parseMachine(g *h5c.Group) (Machine, error) {
	bx, err := readFloatAttr(g, "Build_Plate_X_Dimension")
	if err != nil {
		return Machine{}, err
	}
	by, err := readFloatAttr(g, "Build_Plate_Y_Dimension")
	if err != nil {
		return Machine{}, err
	}
	bz, err := readFloatAttr(g, "Build_Plate_Z_Dimension")
	if err != nil {
		return Machine{}, err
	}
	br, err := readFloatAttr(g, "Build_Plate_Corner_Radius")
	if err != nil {
		return Machine{}, err
	}
	bxu, _ := readStrLocked(g, "Build_Plate_X_Dimension_unit", "mm")
	byu, _ := readStrLocked(g, "Build_Plate_Y_Dimension_unit", "mm")
	bzu, _ := readStrLocked(g, "Build_Plate_Z_Dimension_unit", "mm")
	bru, _ := readStrLocked(g, "Build_Plate_Corner_Radius_unit", "mm")
	return Machine{
		ID:                   readStrAttr(g, "ID"),
		MachineName:          readRequiredStr(g, "Machine_Name"),
		Manufacturer:         readRequiredStr(g, "Manufacturer"),
		Model:                readRequiredStr(g, "Model"),
		SerialNumber:         readRequiredStr(g, "Serial_Number"),
		BuildPlateX:          bx,
		BuildPlateXUnit:      bxu,
		BuildPlateY:          by,
		BuildPlateYUnit:      byu,
		BuildPlateZ:          bz,
		BuildPlateZUnit:      bzu,
		BuildPlateRadius:     br,
		BuildPlateRadiusUnit: bru,
		GasFlowDirection:     readStrAttr(g, "Gas_Flow_Direction"),
		RecoatDirection:      readStrAttr(g, "Recoat_Direction"),
	}, nil
}

func parseTrain(
	f *h5c.File,
	root *h5c.Group,
	trainID string,
	hasClearboxRoot bool,
	sharedOutputPath *string,
	sharedSoftwareTriggerDelay *int,
	includeBinary bool,
) (OpticalTrain, error) {
	base := layout.TrainPathByID(trainID)
	g, err := f.Group(base)
	if err != nil {
		return OpticalTrain{}, err
	}
	defer g.Close()

	mustGroup := func(name string) (*h5c.Group, error) {
		if !g.LinkExists(name) {
			return nil, fmt.Errorf("required subgroup %q missing", name)
		}
		return g.OpenGroup(name)
	}

	sg, err := mustGroup(layout.GroupScanner)
	if err != nil {
		return OpticalTrain{}, err
	}
	defer sg.Close()
	scanner, err := parseScanner(sg)
	if err != nil {
		return OpticalTrain{}, err
	}

	lg, err := mustGroup(layout.GroupLightSource)
	if err != nil {
		return OpticalTrain{}, err
	}
	defer lg.Close()
	light, err := parseLightSource(lg)
	if err != nil {
		return OpticalTrain{}, err
	}

	cg, err := mustGroup(layout.GroupCollimator)
	if err != nil {
		return OpticalTrain{}, err
	}
	defer cg.Close()
	collimator, err := parseCollimator(cg)
	if err != nil {
		return OpticalTrain{}, err
	}

	scg, err := mustGroup(layout.GroupScannerCard)
	if err != nil {
		return OpticalTrain{}, err
	}
	defer scg.Close()
	card, err := parseScannerCard(scg)
	if err != nil {
		return OpticalTrain{}, err
	}

	// Change 1: ClearBox lives at Extensions/ClearBox/<train_id>/ now, not
	// under this train's own group — only present if the shared root exists
	// AND this specific train has its own subgroup there.
	var clearbox *ClearBox
	if hasClearboxRoot && root.LinkExists(layout.ClearboxPathByID(trainID)) {
		cb, err := parseClearBox(f, trainID, sharedOutputPath, sharedSoftwareTriggerDelay, includeBinary)
		if err != nil {
			return OpticalTrain{}, err
		}
		clearbox = &cb
	}

	var sfcf *ScanFieldCorrectionFile
	if g.LinkExists(layout.DSScanFieldCorrectionFile) {
		sfcf, err = parseSFCF(f, base+"/"+layout.DSScanFieldCorrectionFile, includeBinary)
		if err != nil {
			return OpticalTrain{}, err
		}
	}

	tlp, err := readBoolFromIntAttr(g, "Thermal_Lensing_Test_Passed")
	if err != nil {
		return OpticalTrain{}, err
	}
	bwMajor, err := readFloatAttr(g, "Beam_Waist_Major")
	if err != nil {
		return OpticalTrain{}, err
	}
	bwMinor, err := readFloatAttr(g, "Beam_Waist_Minor")
	if err != nil {
		return OpticalTrain{}, err
	}
	cfl, err := readFloatAttr(g, "Collimator_Focal_Length")
	if err != nil {
		return OpticalTrain{}, err
	}
	tlShift, err := readFloatAttr(g, "Thermal_Lensing_Focal_Plane_Shift")
	if err != nil {
		return OpticalTrain{}, err
	}
	tlThresh, err := readFloatAttr(g, "Thermal_Lensing_Threshold")
	if err != nil {
		return OpticalTrain{}, err
	}

	return OpticalTrain{
		TrainID:                           trainID,
		ID:                                readStrAttr(g, "ID"),
		BeamProfileType:                   readStrAttr(g, "Beam_Profile_Type"),
		BeamWaistDefinition:               readStrAttr(g, "Beam_Waist_Definition"),
		BeamWaistMajor:                    bwMajor,
		BeamWaistMajorUnit:                readStrAttr(g, "Beam_Waist_Major_unit"),
		BeamWaistMinor:                    bwMinor,
		BeamWaistMinorUnit:                readStrAttr(g, "Beam_Waist_Minor_unit"),
		BeamWaistOffsetZ:                  mustFloat(g, "Beam_Waist_Offset_Z"),
		BeamWaistOffsetZUnit:              readStrAttr(g, "Beam_Waist_Offset_Z_unit"),
		M2Major:                           mustFloat(g, "M2_Major"),
		M2Minor:                           mustFloat(g, "M2_Minor"),
		RayleighLengthMajor:               mustFloat(g, "Rayleigh_Length_Major"),
		RayleighLengthMajorUnit:           readStrAttr(g, "Rayleigh_Length_Major_unit"),
		RayleighLengthMinor:               mustFloat(g, "Rayleigh_Length_Minor"),
		RayleighLengthMinorUnit:           readStrAttr(g, "Rayleigh_Length_Minor_unit"),
		BuildPlaneOffsetMajor:             mustFloat(g, "Build_Plane_Offset_Major"),
		BuildPlaneOffsetMajorUnit:         readStrAttr(g, "Build_Plane_Offset_Major_unit"),
		BuildPlaneOffsetMinor:             mustFloat(g, "Build_Plane_Offset_Minor"),
		BuildPlaneOffsetMinorUnit:         readStrAttr(g, "Build_Plane_Offset_Minor_unit"),
		CollimatorFocalLength:             cfl,
		CollimatorFocalLengthUnit:         readStrAttr(g, "Collimator_Focal_Length_unit"),
		MajorAxisAngle:                    mustFloat(g, "Major_Axis_Angle"),
		MajorAxisAngleUnit:                readStrAttr(g, "Major_Axis_Angle_unit"),
		ScannerNumber:                     readStrAttr(g, "Scanner_Number"),
		ThermalLensingPassed:              tlp,
		ThermalLensingFocalPlaneShift:     tlShift,
		ThermalLensingFocalPlaneShiftUnit: readStrAttr(g, "Thermal_Lensing_Focal_Plane_Shift_unit"),
		ThermalLensingThreshold:           tlThresh,
		ThermalLensingThresholdUnit:       readStrAttr(g, "Thermal_Lensing_Threshold_unit"),
		Scanner:                           scanner,
		LightSource:                       light,
		Collimator:                        collimator,
		ScannerCard:                       card,
		OptionalComponents:                OptionalComponents{Clearbox: clearbox},
		ScanFieldCorrectionFile:           sfcf,
	}, nil
}

func mustFloat(g *h5c.Group, key string) *float64 {
	v, _ := readFloatAttr(g, key)
	return v
}

func parseScanner(g *h5c.Group) (Scanner, error) {
	xag, err := g.OpenGroup("X_Axis")
	if err != nil {
		return Scanner{}, err
	}
	defer xag.Close()
	yag, err := g.OpenGroup("Y_Axis")
	if err != nil {
		return Scanner{}, err
	}
	defer yag.Close()
	xa, err := parseAxis(xag)
	if err != nil {
		return Scanner{}, err
	}
	ya, err := parseAxis(yag)
	if err != nil {
		return Scanner{}, err
	}
	var za *AxisConfig
	if g.LinkExists("Z_Axis") {
		zag, err := g.OpenGroup("Z_Axis")
		if err != nil {
			return Scanner{}, err
		}
		a, err := parseAxis(zag)
		zag.Close()
		if err != nil {
			return Scanner{}, err
		}
		za = &a
	}
	var focus *AxisConfig
	if g.LinkExists("Focus") {
		fg, err := g.OpenGroup("Focus")
		if err != nil {
			return Scanner{}, err
		}
		a, err := parseAxis(fg)
		fg.Close()
		if err != nil {
			return Scanner{}, err
		}
		focus = &a
	}
	wd, err := readFloatAttr(g, "Working_Distance")
	if err != nil {
		return Scanner{}, err
	}
	wdu, _ := readStrLocked(g, "Working_Distance_unit", "mm")
	// Plain bool, not *bool — defaults to false whether the attribute is
	// absent or explicitly 0 (matches v1.0's convention exactly).
	iax, err := readBoolFromIntAttr(g, "Invert_Actual_X")
	if err != nil {
		return Scanner{}, err
	}
	iay, err := readBoolFromIntAttr(g, "Invert_Actual_Y")
	if err != nil {
		return Scanner{}, err
	}
	icx, err := readBoolFromIntAttr(g, "Invert_Commanded_X")
	if err != nil {
		return Scanner{}, err
	}
	icy, err := readBoolFromIntAttr(g, "Invert_Commanded_Y")
	if err != nil {
		return Scanner{}, err
	}
	return Scanner{
		Manufacturer:         readRequiredStr(g, "Manufacturer"),
		Model:                readRequiredStr(g, "Model"),
		SerialNumber:         readRequiredStr(g, "Serial_Number"),
		WorkingDistance:      wd,
		WorkingDistanceUnit:  wdu,
		ScanFieldX:           mustFloat(g, "Scan_Field_Size_X"),
		ScanFieldXUnit:       readStrAttr(g, "Scan_Field_Size_X_unit"),
		ScanFieldY:           mustFloat(g, "Scan_Field_Size_Y"),
		ScanFieldYUnit:       readStrAttr(g, "Scan_Field_Size_Y_unit"),
		ScanFieldZ:           mustFloat(g, "Scan_Field_Size_Z"),
		ScanFieldZUnit:       readStrAttr(g, "Scan_Field_Size_Z_unit"),
		ScanHeadOffsetX:      mustFloat(g, "Scan_Head_Offset_X"),
		ScanHeadOffsetXUnit:  readStrAttr(g, "Scan_Head_Offset_X_unit"),
		ScanHeadOffsetY:      mustFloat(g, "Scan_Head_Offset_Y"),
		ScanHeadOffsetYUnit:  readStrAttr(g, "Scan_Head_Offset_Y_unit"),
		ScanHeadOffsetZ:      mustFloat(g, "Scan_Head_Offset_Z"),
		ScanHeadOffsetZUnit:  readStrAttr(g, "Scan_Head_Offset_Z_unit"),
		ScanHeadRotation:     mustFloat(g, "Scan_Head_Rotation"),
		ScanHeadRotationUnit: readStrAttr(g, "Scan_Head_Rotation_unit"),
		AxisConfiguration:    readStrAttr(g, "Axis_Configuration"),
		XAxis:                xa,
		YAxis:                ya,
		ZAxis:                za,
		Focus:                focus,
		InvertActualX:        iax != nil && *iax,
		InvertActualY:        iay != nil && *iay,
		InvertCommandedX:     icx != nil && *icx,
		InvertCommandedY:     icy != nil && *icy,
	}, nil
}

// parseAxis: Change 5 — Tuning_Parameters/Tuning_Type have no on-disk
// source in v1.1; always nil here, regardless of what (if anything) is in
// the group.
func parseAxis(g *h5c.Group) (AxisConfig, error) {
	abr, err := readIntAttr(g, "Actual_Bit_Resolution")
	if err != nil {
		return AxisConfig{}, err
	}
	cbr, err := readIntAttr(g, "Commanded_Bit_Resolution")
	if err != nil {
		return AxisConfig{}, err
	}
	rom, err := readFloatAttr(g, "Range_Of_Motion")
	if err != nil {
		return AxisConfig{}, err
	}
	sp, err := readFloatAttr(g, "Smoothing_Parameters")
	if err != nil {
		return AxisConfig{}, err
	}
	return AxisConfig{
		ActualBitResolution:        abr,
		ActualBitResolutionUnit:    readStrAttr(g, "Actual_Bit_Resolution_unit"),
		CommandedBitResolution:     cbr,
		CommandedBitResolutionUnit: readStrAttr(g, "Commanded_Bit_Resolution_unit"),
		ControlType:                readStrAttr(g, "Control_Type"),
		RangeOfMotion:              rom,
		RangeOfMotionUnit:          readStrAttr(g, "Range_Of_Motion_unit"),
		SmoothingKernel:            readStrAttr(g, "Smoothing_Kernel"),
		SmoothingParameters:        sp,
		TuningParameters:           nil,
		TuningType:                 nil,
	}, nil
}

// parsePowerCharacterization reads one Power_Characterization group (shared
// shape for both ClearBox's and Light_Source's instance — Changes 3/4).
func parsePowerCharacterization(g *h5c.Group) (PowerCharacterization, error) {
	constants := []EquationConstant{}
	if rows, err := g.ReadEquationConstantsDataset(layout.DSDerivationEquationConstants); err == nil {
		for _, r := range rows {
			constants = append(constants, EquationConstant{Name: r.Name, Value: r.Value})
		}
	}
	points := []CalibrationPoint{}
	if rows, err := g.ReadCalibrationPointsDataset(layout.DSCharacterizationPoints); err == nil {
		for _, r := range rows {
			points = append(points, CalibrationPoint{InputValue: r.InputValue, OutputValue: r.OutputValue})
		}
	}
	return PowerCharacterization{
		AlgorithmType:               readStrAttr(g, "Algorithm_Type"),
		AlgorithmEquation:           readStrAttr(g, "Algorithm_Equation"),
		InputType:                   readStrAttr(g, "Input_Type"),
		UnitsDerivedQuantity:        readStrAttr(g, "Units_Derived_Quantity"),
		DerivationEquationConstants: constants,
		CharacterizationPoints:      points,
	}, nil
}

// parseLightSource: Change 4 — Watts_To_Volts_Algorithm/Params have no
// on-disk source in v1.1 (superseded by Power_Characterization).
func parseLightSource(g *h5c.Group) (LightSource, error) {
	var pc *PowerCharacterization
	if g.LinkExists(layout.GroupPowerCharacterization) {
		pcg, err := g.OpenGroup(layout.GroupPowerCharacterization)
		if err != nil {
			return LightSource{}, err
		}
		parsed, err := parsePowerCharacterization(pcg)
		pcg.Close()
		if err != nil {
			return LightSource{}, err
		}
		pc = &parsed
	}
	return LightSource{
		Manufacturer:           readRequiredStr(g, "Manufacturer"),
		Model:                  readRequiredStr(g, "Model"),
		SerialNumber:           readRequiredStr(g, "Serial_Number"),
		Wavelength:             mustFloat(g, "Light_Wavelength"),
		WavelengthUnit:         readStrAttr(g, "Light_Wavelength_unit"),
		PowerMaxNominal:        mustFloat(g, "Power_Max_Nominal"),
		PowerMaxNominalUnit:    readStrAttr(g, "Power_Max_Nominal_unit"),
		PowerMaxActual:         mustFloat(g, "Power_Max_Actual"),
		PowerMaxActualUnit:     readStrAttr(g, "Power_Max_Actual_unit"),
		PowerMinActual:         mustFloat(g, "Power_Min_Actual"),
		PowerMinActualUnit:     readStrAttr(g, "Power_Min_Actual_unit"),
		PowerMinNominal:        mustFloat(g, "Power_Min_Nominal"),
		PowerMinNominalUnit:    readStrAttr(g, "Power_Min_Nominal_unit"),
		PowerBitResolution:     mustFloat(g, "Power_Bit_Resolution"),
		PowerBitResolutionUnit: readStrAttr(g, "Power_Bit_Resolution_unit"),
		WattsToVoltsAlgorithm:  nil,
		WattsToVoltsParams:     nil,
		PowerCharacterization:  pc,
	}, nil
}

func parseCollimator(g *h5c.Group) (Collimator, error) {
	fl, err := readFloatAttr(g, "Focal_Length")
	if err != nil {
		return Collimator{}, err
	}
	flu, _ := readStrLocked(g, "Focal_Length_unit", "mm")
	return Collimator{
		Manufacturer:    readRequiredStr(g, "Manufacturer"),
		Model:           readRequiredStr(g, "Model"),
		SerialNumber:    readRequiredStr(g, "Serial_Number"),
		FocalLength:     fl,
		FocalLengthUnit: flu,
	}, nil
}

func parseScannerCard(g *h5c.Group) (ScannerCard, error) {
	sp, err := readFloatAttr(g, "Sample_Period")
	if err != nil {
		return ScannerCard{}, err
	}
	return ScannerCard{
		Manufacturer:          readRequiredStr(g, "Manufacturer"),
		Model:                 readRequiredStr(g, "Model"),
		SerialNumber:          readRequiredStr(g, "Serial_Number"),
		CommunicationProtocol: readStrAttr(g, "Communication_Protocol"),
		SamplePeriod:          sp,
		SamplePeriodUnit:      readStrAttr(g, "Sample_Period_unit"),
	}, nil
}

// parseClearBox reads one Extensions/ClearBox/<train_id>/ subgroup. Change
// 1: several v1.0 attrs no longer exist on disk (always nil here);
// Firmware_Version is new; Output_Path/Software_Trigger_Delay come from the
// shared Consolidate value read once by the caller, not from this group.
func parseClearBox(f *h5c.File, trainID string, sharedOutputPath *string, sharedSoftwareTriggerDelay *int, includeBinary bool) (ClearBox, error) {
	path := layout.ClearboxPathByID(trainID)
	g, err := f.Group(path)
	if err != nil {
		return ClearBox{}, err
	}
	defer g.Close()

	dp, err := readIntAttr(g, "Data_Port")
	if err != nil {
		return ClearBox{}, err
	}
	sp, err := readIntAttr(g, "Server_Port")
	if err != nil {
		return ClearBox{}, err
	}
	ato, err := readIntAttr(g, "Actual_Timing_Offset")
	if err != nil {
		return ClearBox{}, err
	}
	cto, err := readIntAttr(g, "Commanded_Timing_Offset")
	if err != nil {
		return ClearBox{}, err
	}

	cb := ClearBox{
		IPAddress:                 readRequiredStr(g, "Ip_Address"),
		SerialNumber:              readStrAttr(g, "Serial_Number"),
		DataPort:                  dp,
		ServerPort:                sp,
		ActualTimingOffset:        ato,
		CommandedTimingOffset:     cto,
		Manufacturer:              readStrAttr(g, "Manufacturer"),
		Model:                     readStrAttr(g, "Model"),
		OutputPath:                sharedOutputPath,
		SelectedCamera:            nil,
		CustomVideoFormat:         nil,
		VideoOutput:               nil,
		ShowConsole:               nil,
		SoftwareTriggerDelay:      sharedSoftwareTriggerDelay,
		VoltsToWattsAlgorithm:     nil,
		VoltsToWattsParams:        nil,
		CorrectionGridDomainShape: nil,
		InverseGridDomainShape:    nil,
		SynchronousSensors:        parseSynchronousSensors(g),
		FirmwareVersion:           readStrAttr(g, "Firmware_Version"),
	}

	if g.LinkExists(layout.GroupPowerCharacterization) {
		pcg, err := g.OpenGroup(layout.GroupPowerCharacterization)
		if err != nil {
			return ClearBox{}, err
		}
		parsed, err := parsePowerCharacterization(pcg)
		pcg.Close()
		if err != nil {
			return ClearBox{}, err
		}
		cb.PowerCharacterization = &parsed
	}

	if includeBinary {
		if grid, err := readFloatGrid(f, path+"/"+layout.DSCorrectionData); err == nil {
			cb.CorrectionData = grid
		}
		if grid, err := readFloatGrid(f, path+"/"+layout.DSInverseCorrectionData); err == nil {
			cb.InverseCorrectionData = grid
		}
	}
	return cb, nil
}

// parseSynchronousSensors enumerates Synchronous_Sensors/<name> sub-groups
// — unchanged shape from v1.0 (Change 1's field-by-field table lists it as
// a plain Path relocation, since it moves along with the rest of ClearBox).
func parseSynchronousSensors(g *h5c.Group) map[string]SynchronousSensor {
	sensors := map[string]SynchronousSensor{}
	if !g.LinkExists("Synchronous_Sensors") {
		return sensors
	}
	sensorsGrp, err := g.OpenGroup("Synchronous_Sensors")
	if err != nil {
		return sensors
	}
	defer sensorsGrp.Close()
	for _, name := range sensorsGrp.SubGroupNames() {
		sg, err := sensorsGrp.OpenGroup(name)
		if err != nil {
			continue
		}
		sensors[name] = parseSynchronousSensor(sg)
		sg.Close()
	}
	return sensors
}

func parseSynchronousSensor(g *h5c.Group) SynchronousSensor {
	enabled, _ := readBoolFromIntAttr(g, "Enabled")
	rangeLow, _ := readFloatAttr(g, "Sensor_Output_Range_Low")
	rangeHigh, _ := readFloatAttr(g, "Sensor_Output_Range_High")
	portID, _ := readIntAttr(g, "Port_ID")
	calibVerified, _ := readBoolFromIntAttr(g, "Calibration_Verified")
	samplePeriod, _ := readFloatAttr(g, "Sample_Period")

	constants := []EquationConstant{}
	if rows, err := g.ReadEquationConstantsDataset("Derivation_Equation_Constants"); err == nil {
		for _, r := range rows {
			constants = append(constants, EquationConstant{Name: r.Name, Value: r.Value})
		}
	}
	points := []CalibrationPoint{}
	if rows, err := g.ReadCalibrationPointsDataset("Calibration_Points"); err == nil {
		for _, r := range rows {
			points = append(points, CalibrationPoint{InputValue: r.InputValue, OutputValue: r.OutputValue})
		}
	}

	return SynchronousSensor{
		Enabled:                     enabled,
		SensorName:                  readStrAttr(g, "Sensor_Name"),
		SensorOutputRangeLow:        rangeLow,
		SensorOutputRangeHigh:       rangeHigh,
		SensorOutputSpace:           readStrAttr(g, "Sensor_Output_Space"),
		SensorModel:                 readStrAttr(g, "Sensor_Model"),
		SensorManufacturer:          readStrAttr(g, "Sensor_Manufacturer"),
		SensorScope:                 readStrAttr(g, "Sensor_Scope"),
		UnitsDerivedQuantity:        readStrAttr(g, "Units_Derived_Quantity"),
		PortID:                      portID,
		SensorType:                  readStrAttr(g, "Sensor_Type"),
		InputType:                   readStrAttr(g, "Input_Type"),
		AlgorithmType:               readStrAttr(g, "Algorithm_Type"),
		AlgorithmEquation:           readStrAttr(g, "Algorithm_Equation"),
		CalibrationSource:           readStrAttr(g, "Calibration_Source"),
		CalibrationVerified:         calibVerified,
		SamplePeriod:                samplePeriod,
		Metadata:                    readStrAttr(g, "Metadata"),
		DerivationEquationConstants: constants,
		CalibrationPoints:           points,
	}
}

func readFloatGrid(f *h5c.File, path string) (*[][][]*float64, error) {
	ds, err := f.OpenDataset(path)
	if err != nil {
		return nil, err
	}
	defer ds.Close()
	dims, err := ds.Dims()
	if err != nil || len(dims) != 3 {
		return nil, fmt.Errorf("expected 3D dataset at %s", path)
	}
	n := dims[0] * dims[1] * dims[2]
	flat := make([]float64, n)
	if err := ds.ReadFloat64(flat); err != nil {
		return nil, err
	}
	out := NestedGridFromFlat(flat, [3]int{int(dims[0]), int(dims[1]), int(dims[2])})
	return &out, nil
}

func parseSFCF(f *h5c.File, path string, includeBinary bool) (*ScanFieldCorrectionFile, error) {
	ds, err := f.OpenDataset(path)
	if err != nil {
		return nil, nil
	}
	defer ds.Close()

	readDSStr := func(key string) string {
		if !ds.HasAttr(key) {
			return ""
		}
		s, err := ds.ReadStringAttr(key)
		if err != nil {
			return ""
		}
		return strings.TrimSpace(strings.TrimRight(s, "\x00"))
	}
	readDSStrPtr := func(key string) *string {
		s := readDSStr(key)
		if s == "" {
			return nil
		}
		return &s
	}

	sfcf := &ScanFieldCorrectionFile{
		DocumentName:      readDSStr("document_name"),
		DocumentID:        readDSStr("document_id"),
		ValidAsOfDate:     readDSStr("valid_as_of_date"),
		DocumentCreatedAt: readDSStrPtr("document_created_at"),
		DocumentType:      readDSStrPtr("document_type"),
		OriginalURI:       readDSStrPtr("original_uri"),
	}
	if ds.HasAttr("file_size") {
		if v, err := ds.ReadInt64Attr("file_size"); err == nil {
			sfcf.FileSize = int(v)
		}
	}

	if includeBinary {
		dims, err := ds.Dims()
		if err == nil && len(dims) == 1 {
			buf := make([]byte, dims[0])
			if err := ds.ReadUint8(buf); err == nil {
				sfcf.RawBytes = buf
				if sfcf.FileSize == 0 {
					sfcf.FileSize = int(dims[0])
				}
			}
		}
	}
	return sfcf, nil
}

// parseOpcua: Change 2 — reads from Extensions/TM_OPCUA/ now, not root-level
// OPCUA/. Internal Client/Pipe/Triggers shape is unchanged.
func parseOpcua(f *h5c.File, root *h5c.Group) (*OpcuaConfig, error) {
	if !root.LinkExists(layout.RootOPCUA) {
		return nil, nil
	}
	og, err := f.Group(layout.RootOPCUA)
	if err != nil {
		return nil, err
	}
	defer og.Close()
	if !og.LinkExists("Client") {
		return nil, nil
	}
	cg, err := og.OpenGroup("Client")
	if err != nil {
		return nil, err
	}
	defer cg.Close()
	clientKnownKeys := map[string]bool{
		"Server_URL": true, "Auth_Mode": true, "Security_Mode": true,
		"Security_Policy": true, "BFS_Max_Depth": true, "Publish_Interval": true,
		"Sampling_Interval": true, "Session_Timeout": true,
		"Keep_Alive_Count": true, "Lifetime_Count": true, "Machine_Profile": true,
		"Queue_Policy": true, "Queue_Size_Data_Change": true, "Queue_Size_Events": true,
		"Reconnect_Interval": true, "Root_Node": true,
		"Sync_Loop_Interval_Initial": true, "Sync_Loop_Interval_Settled": true,
	}
	client := OpcuaClientConfig{
		ServerURL:      readRequiredStr(cg, "Server_URL"),
		AuthMode:       readRequiredStr(cg, "Auth_Mode"),
		SecurityMode:   readRequiredStr(cg, "Security_Mode"),
		SecurityPolicy: readRequiredStr(cg, "Security_Policy"),
		Extra:          readGroupExtras(cg, clientKnownKeys),
	}
	if v, err := readIntAttr(cg, "BFS_Max_Depth"); err == nil && v != nil {
		client.BfsMaxDepth = *v
	}
	if v, err := readIntAttr(cg, "Publish_Interval"); err == nil && v != nil {
		client.PublishInterval = *v
	}
	if v, err := readIntAttr(cg, "Sampling_Interval"); err == nil && v != nil {
		client.SamplingInterval = *v
	}
	if v, err := readIntAttr(cg, "Session_Timeout"); err == nil && v != nil {
		client.SessionTimeout = *v
	}
	if v, err := readIntAttr(cg, "Keep_Alive_Count"); err == nil {
		client.KeepAliveCount = v
	}
	if v, err := readIntAttr(cg, "Lifetime_Count"); err == nil {
		client.LifetimeCount = v
	}
	client.MachineProfile = readStrAttr(cg, "Machine_Profile")
	client.QueuePolicy = readStrAttr(cg, "Queue_Policy")
	if v, err := readIntAttr(cg, "Queue_Size_Data_Change"); err == nil {
		client.QueueSizeDataChange = v
	}
	if v, err := readIntAttr(cg, "Queue_Size_Events"); err == nil {
		client.QueueSizeEvents = v
	}
	if v, err := readIntAttr(cg, "Reconnect_Interval"); err == nil {
		client.ReconnectInterval = v
	}
	client.RootNode = readStrAttr(cg, "Root_Node")
	if v, err := readIntAttr(cg, "Sync_Loop_Interval_Initial"); err == nil {
		client.SyncLoopIntervalInitial = v
	}
	if v, err := readIntAttr(cg, "Sync_Loop_Interval_Settled"); err == nil {
		client.SyncLoopIntervalSettled = v
	}

	pipeKnownKeys := map[string]bool{
		"Pipe_Enabled": true, "Buffer_Size": true,
		"Configure_Client": true, "Inbound_Rate_Limit": true,
		"Max_Inbound_Message_Size": true, "Min_Integrity_Level": true,
		"Pipe_Name": true, "User_Access_Level": true,
	}
	pipe := OpcuaPipeConfig{Extra: map[string]any{}}
	if og.LinkExists("Pipe") {
		pg, err := og.OpenGroup("Pipe")
		if err == nil {
			if b, err := readBoolFromIntAttr(pg, "Pipe_Enabled"); err == nil && b != nil {
				pipe.PipeEnabled = *b
			}
			if v, err := readIntAttr(pg, "Buffer_Size"); err == nil && v != nil {
				pipe.BufferSize = *v
			}
			if b, err := readBoolFromIntAttr(pg, "Configure_Client"); err == nil {
				pipe.ConfigureClient = b
			}
			if v, err := readIntAttr(pg, "Inbound_Rate_Limit"); err == nil {
				pipe.InboundRateLimit = v
			}
			if v, err := readIntAttr(pg, "Max_Inbound_Message_Size"); err == nil {
				pipe.MaxInboundMessageSize = v
			}
			pipe.MinIntegrityLevel = readStrAttr(pg, "Min_Integrity_Level")
			pipe.PipeName = readStrAttr(pg, "Pipe_Name")
			pipe.UserAccessLevel = readStrAttr(pg, "User_Access_Level")
			pipe.Extra = readGroupExtras(pg, pipeKnownKeys)
			pg.Close()
		}
	}

	triggers := map[string]OpcuaTrigger{}
	var triggersEnabled *bool
	var triggerStopCeilingLayers *int
	if og.LinkExists("Triggers") {
		tg, err := og.OpenGroup("Triggers")
		if err == nil {
			if b, err := readBoolFromIntAttr(tg, "Triggers_Enabled"); err == nil {
				triggersEnabled = b
			}
			if v, err := readIntAttr(tg, "Trigger_Stop_Ceiling_Layers"); err == nil {
				triggerStopCeilingLayers = v
			}
			triggerKnownKeys := map[string]bool{
				"ID": true, "Signal": true, "Subsystem": true,
				"Rule_Enabled": true, "Start_Value": true, "Stop_Value": true,
				"Case_Sensitivity": true, "Component": true, "Cooldown_Period": true,
				"Event": true, "Max_Fires_Per_Job": true, "Trigger_Label": true,
			}
			for _, subname := range tg.SubGroupNames() {
				sg, err := tg.OpenGroup(subname)
				if err != nil {
					continue
				}
				re, _ := readBoolFromIntAttr(sg, "Rule_Enabled")
				var cooldownPeriod, maxFiresPerJob *int
				if v, err := readIntAttr(sg, "Cooldown_Period"); err == nil {
					cooldownPeriod = v
				}
				if v, err := readIntAttr(sg, "Max_Fires_Per_Job"); err == nil {
					maxFiresPerJob = v
				}
				trig := OpcuaTrigger{
					ID:              readStrAttr(sg, "ID"),
					Signal:          readStrAttr(sg, "Signal"),
					Subsystem:       readStrAttr(sg, "Subsystem"),
					RuleEnabled:     re,
					StartValue:      readStrAttr(sg, "Start_Value"),
					StopValue:       readStrAttr(sg, "Stop_Value"),
					CaseSensitivity: readStrAttr(sg, "Case_Sensitivity"),
					Component:       readStrAttr(sg, "Component"),
					CooldownPeriod:  cooldownPeriod,
					Event:           readStrAttr(sg, "Event"),
					MaxFiresPerJob:  maxFiresPerJob,
					TriggerLabel:    readStrAttr(sg, "Trigger_Label"),
					Extra:           readGroupExtras(sg, triggerKnownKeys),
				}
				sg.Close()
				triggers[subname] = trig
			}
			tg.Close()
		}
	}

	return &OpcuaConfig{
		Client:                   client,
		Pipe:                     pipe,
		Triggers:                 triggers,
		TriggersEnabled:          triggersEnabled,
		TriggerStopCeilingLayers: triggerStopCeilingLayers,
	}, nil
}

// GetCorrectionData loads the ClearBox Correction_Data grid for train index.
func (r *Adapter) GetCorrectionData(trainIndex int) (*CorrectionData, error) {
	return r.readCorrection(layout.ClearboxPath(trainIndex) + "/" + layout.DSCorrectionData)
}

// GetInverseCorrectionData loads the ClearBox Inverse_Correction_Data grid for train index.
func (r *Adapter) GetInverseCorrectionData(trainIndex int) (*CorrectionData, error) {
	return r.readCorrection(layout.ClearboxPath(trainIndex) + "/" + layout.DSInverseCorrectionData)
}

func (r *Adapter) readCorrection(path string) (*CorrectionData, error) {
	f, err := h5c.Open(r.path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	ds, err := f.OpenDataset(path)
	if err != nil {
		return nil, err
	}
	defer ds.Close()
	dims, err := ds.Dims()
	if err != nil || len(dims) != 3 {
		return nil, fmt.Errorf("expected 3D dataset")
	}
	n := dims[0] * dims[1] * dims[2]
	data := make([]float64, n)
	if err := ds.ReadFloat64(data); err != nil {
		return nil, err
	}
	return &CorrectionData{
		Data:  data,
		Shape: [3]int{int(dims[0]), int(dims[1]), int(dims[2])},
	}, nil
}
