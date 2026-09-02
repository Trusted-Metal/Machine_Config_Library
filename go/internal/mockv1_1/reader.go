// File_Version "1.1-mock" HDF5 reader.
//
// A fully independent reimplementation of v1.0's reader
// (machine-config-go/capabilities/v1_0/hdf5/hdf5.go) with the 10 documented
// differences from docs/migrations/mock_v1_0_to_v1_1.md applied natively,
// rather than delegating to v1.0's Parse and patching the delta afterward.
// See the package doc in mockv1_1.go for why: no version's adapter,
// including a test-only mock one, may import or call into another version's
// adapter code.
//
// Every "unchanged" field/subcomponent below (light_source, collimator,
// scanner_card, clearbox, sfcf, opcua, and every Scanner/AxisConfig field
// besides the one renamed one) is a byte-for-byte duplicate of the
// corresponding v1.0 parse* function — intentional duplication, not a
// shortcut. Binary datasets (ClearBox correction grids, SFCF raw bytes) are
// never read here, matching this mock's pre-existing behavior (it always
// called the v1.0 parser with includeBinary=false).
package mockv1_1

import (
	"fmt"
	"strings"

	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

const schemaVersion = "v1"

// MockV1_1Reader parses a File_Version "1.1-mock" HDF5 file.
type MockV1_1Reader struct {
	path string
}

// NewReader constructs a reader for path.
func NewReader(path string) *MockV1_1Reader {
	return &MockV1_1Reader{path: path}
}

// Parse reads the file at r.path as mock v1.1.
func (r *MockV1_1Reader) Parse() (*MachineConfig, error) {
	f, err := h5c.Open(r.path)
	if err != nil {
		return nil, fmt.Errorf("open %q: %w", r.path, err)
	}
	defer f.Close()
	return parse(f)
}

func parse(f *h5c.File) (*MachineConfig, error) {
	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()

	fv := readRequiredStr(root, "File_Version")

	// ADDITION (x2): Facility_ID / Config_Author are typed root attrs in
	// mock v1.1 (absent entirely in v1.0) — keep them out of Extra like
	// every other known key.
	metaKnownKeys := map[string]bool{
		"File_Version": true, "machine_name": true, "manufacturer": true,
		"model": true, "serial_number": true, "Export_Date": true,
		"Configuration_Hash": true, AttrFacilityID: true, AttrConfigAuthor: true,
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
		FacilityID:        readStrAttr(root, AttrFacilityID),
		ConfigAuthor:      readStrAttr(root, AttrConfigAuthor),
		Extra:             readGroupExtras(root, metaKnownKeys),
	}

	machineGrp, err := f.Group(RootMachine)
	if err != nil {
		return nil, err
	}
	defer machineGrp.Close()

	machine, err := parseMachine(machineGrp)
	if err != nil {
		return nil, err
	}

	trainsGrp, err := f.Group(RootOpticalTrains)
	if err != nil {
		return nil, err
	}
	defer trainsGrp.Close()

	var trains []OpticalTrain
	for i := 1; ; i++ {
		tid := TrainID(i - 1)
		if !trainsGrp.LinkExists(tid) {
			break
		}
		train, err := parseTrain(f, tid)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", tid, err)
		}
		trains = append(trains, train)
	}

	opcua, err := parseOpcua(f)
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

// parseMachine reads Machine/ attrs.
//
// REMOVAL (x2): Gas_Flow_Direction and Recoat_Direction are never read in
// mock v1.1 — always nil.
// NAME: Machine_Name -> Machine_Label.
// PATH / NAME+PATH (x4): the build-plate value attrs live in
// Machine/Dimensions/ (Width/Height replacing X/Y_Dimension;
// Z_Dimension/Corner_Radius keep their v1.0 keys, just moved). Unit attrs
// are unaffected — they stay on Machine/ with their v1.0 keys.
func parseMachine(g *h5c.Group) (Machine, error) {
	bxu, _ := readStrLocked(g, "Build_Plate_X_Dimension_unit", "mm")
	byu, _ := readStrLocked(g, "Build_Plate_Y_Dimension_unit", "mm")
	bzu, _ := readStrLocked(g, "Build_Plate_Z_Dimension_unit", "mm")
	bru, _ := readStrLocked(g, "Build_Plate_Corner_Radius_unit", "mm")

	var bx, by, bz, br *float64
	if g.LinkExists(GroupDimensions) {
		dg, err := g.OpenGroup(GroupDimensions)
		if err != nil {
			return Machine{}, err
		}
		defer dg.Close()
		if bx, err = readFloatAttr(dg, AttrBPWidth); err != nil {
			return Machine{}, err
		}
		if by, err = readFloatAttr(dg, AttrBPHeight); err != nil {
			return Machine{}, err
		}
		if bz, err = readFloatAttr(dg, "Build_Plate_Z_Dimension"); err != nil {
			return Machine{}, err
		}
		if br, err = readFloatAttr(dg, "Build_Plate_Corner_Radius"); err != nil {
			return Machine{}, err
		}
	}

	return Machine{
		ID:                   readStrAttr(g, "ID"),
		MachineName:          readRequiredStr(g, AttrMachineLabel),
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
		GasFlowDirection:     nil,
		RecoatDirection:      nil,
	}, nil
}

func parseTrain(f *h5c.File, trainID string) (OpticalTrain, error) {
	base := TrainPathByID(trainID)
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

	sg, err := mustGroup("Scanner")
	if err != nil {
		return OpticalTrain{}, err
	}
	defer sg.Close()
	scanner, err := parseScanner(sg)
	if err != nil {
		return OpticalTrain{}, err
	}

	lg, err := mustGroup("Light_Source")
	if err != nil {
		return OpticalTrain{}, err
	}
	defer lg.Close()
	light, err := parseLightSource(lg)
	if err != nil {
		return OpticalTrain{}, err
	}

	cg, err := mustGroup("Collimator")
	if err != nil {
		return OpticalTrain{}, err
	}
	defer cg.Close()
	collimator, err := parseCollimator(cg)
	if err != nil {
		return OpticalTrain{}, err
	}

	scg, err := mustGroup("Scanner_Card")
	if err != nil {
		return OpticalTrain{}, err
	}
	defer scg.Close()
	card, err := parseScannerCard(scg)
	if err != nil {
		return OpticalTrain{}, err
	}

	var clearbox *ClearBox
	if g.LinkExists("Optional_Components") {
		oc, err := g.OpenGroup("Optional_Components")
		if err != nil {
			return OpticalTrain{}, err
		}
		if oc.LinkExists("ClearBox") {
			cbg, err := oc.OpenGroup("ClearBox")
			if err != nil {
				oc.Close()
				return OpticalTrain{}, err
			}
			cb, err := parseClearBox(cbg)
			cbg.Close()
			oc.Close()
			if err != nil {
				return OpticalTrain{}, err
			}
			clearbox = &cb
		} else {
			oc.Close()
		}
	}

	var sfcf *ScanFieldCorrectionFile
	if g.LinkExists("scan_field_correction_file") {
		sfcf, err = parseSFCF(f, base+"/scan_field_correction_file")
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

// parseScanner reads Scanner/ attrs.
//
// NAME: Working_Distance -> Focal_Distance for the value attr;
// Working_Distance_unit is unchanged.
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
	wd, err := readFloatAttr(g, AttrFocalDistance)
	if err != nil {
		return Scanner{}, err
	}
	wdu, _ := readStrLocked(g, "Working_Distance_unit", "mm")
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
		TuningParameters:           readStrAttr(g, "Tuning_Parameters"),
		TuningType:                 readStrAttr(g, "Tuning_Type"),
	}, nil
}

func parseLightSource(g *h5c.Group) (LightSource, error) {
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

func parseClearBox(g *h5c.Group) (ClearBox, error) {
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
	sc, err := readBoolFromIntAttr(g, "Show_Console")
	if err != nil {
		return ClearBox{}, err
	}
	std, err := readIntAttr(g, "Software_Trigger_Delay")
	if err != nil {
		return ClearBox{}, err
	}
	return ClearBox{
		IPAddress:                 readRequiredStr(g, "Ip_Address"),
		SerialNumber:              readStrAttr(g, "Serial_Number"),
		DataPort:                  dp,
		ServerPort:                sp,
		ActualTimingOffset:        ato,
		CommandedTimingOffset:     cto,
		Manufacturer:              readStrAttr(g, "Manufacturer"),
		Model:                     readStrAttr(g, "Model"),
		OutputPath:                readStrAttr(g, "Output_Path"),
		SelectedCamera:            readStrAttr(g, "Selected_Camera"),
		CustomVideoFormat:         readStrAttr(g, "Custom_Video_Format"),
		VideoOutput:               readStrAttr(g, "Video_Output"),
		ShowConsole:               sc,
		SoftwareTriggerDelay:      std,
		CorrectionGridDomainShape: readStrAttr(g, "Correction_Grid_Domain_Shape"),
		InverseGridDomainShape:    readStrAttr(g, "Inverse_Grid_Domain_Shape"),
		SynchronousSensors:        parseSynchronousSensors(g),
	}, nil
}

// parseSynchronousSensors enumerates Synchronous_Sensors/<name> sub-groups,
// the identical mechanism the OpcuaTrigger loop in parseOpcua uses for
// OPCUA/Triggers/<name>. Returns an empty (non-nil) map when the group
// doesn't exist at all.
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

// parseSynchronousSensor reads one Synchronous Sensor's 18 scalar attributes
// plus its two compound datasets, defaulting each dataset to an empty
// (non-nil) slice if it's itself absent.
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

func parseSFCF(f *h5c.File, path string) (*ScanFieldCorrectionFile, error) {
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
	return sfcf, nil
}

func parseOpcua(f *h5c.File) (*OpcuaConfig, error) {
	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()
	if !root.LinkExists("OPCUA") {
		return nil, nil
	}
	og, err := f.Group("OPCUA")
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
			// Enumerate named trigger subgroups.
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
