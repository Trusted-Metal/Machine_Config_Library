package hdf5

import (
	"fmt"
	"strings"

	"machine-config-go/capabilities/v1_0/layout"
	"machine-config-go/extension"
	"machine-config-go/internal/extio"
	"machine-config-go/internal/h5c"
	. "machine-config-go/internal/models"
)

const schemaVersion = "v1"

// Adapter is the File_Version 1.0 HDF5 reader.
type Adapter struct {
	path string
}

// Parse reads path as File_Version 1.0.
func Parse(path string, includeBinary bool) (*MachineConfig, error) {
	r := &Adapter{path: path}
	return r.parseWithOptions(includeBinary)
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

	machineGrp, err := f.Group("Machine")
	if err != nil {
		return nil, err
	}
	defer machineGrp.Close()

	machine, err := parseMachine(machineGrp)
	if err != nil {
		return nil, err
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
		train, err := parseTrain(f, tid, includeBinary)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", tid, err)
		}
		trains = append(trains, train)
	}

	trainIDs := make([]string, len(trains))
	for i := range trains {
		trainIDs[i] = trains[i].TrainID
	}
	extensions, err := parseExtensions(f, trainIDs)
	if err != nil {
		return nil, err
	}

	return &MachineConfig{
		Meta:          meta,
		Machine:       machine,
		OpticalTrains: trains,
		Extensions:    extensions,
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
		RecoaterBladeType:    readStrAttr(g, "Recoater_Blade_Type"),
	}, nil
}

func parseTrain(f *h5c.File, trainID string, includeBinary bool) (OpticalTrain, error) {
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

	var sfcf *ScanFieldCorrectionFile
	if g.LinkExists("scan_field_correction_file") {
		ds, err := g.OpenDataset("scan_field_correction_file")
		if err == nil {
			// attributes live on the dataset — open as group path via file
			ds.Close()
		}
		// Dataset attrs: open via a dedicated path helper — use Machine group Link
		sfcf, err = parseSFCF(f, base+"/scan_field_correction_file", includeBinary)
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
	// absent or explicitly 0 (user-confirmed, 2026-08-21). See the doc
	// comment on Scanner.InvertActualX in models.go.
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
	wattsToVoltsAlgorithm := readStrAttr(g, "Watts_To_Volts_Algorithm")
	wattsToVoltsParams := readStrAttr(g, "Watts_To_Volts_Params")
	pc, err := ForwardPowerCharacterizationPoints(wattsToVoltsAlgorithm, wattsToVoltsParams)
	if err != nil {
		return LightSource{}, err
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

func parseSFCF(f *h5c.File, path string, includeBinary bool) (*ScanFieldCorrectionFile, error) {
	ds, err := f.OpenDataset(path)
	if err != nil {
		return nil, nil
	}
	defer ds.Close()

	// Read required string attrs from the dataset.
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

// parseExtensions: File_Version 1.0 stores the legacy OPCUA/ group at the root, which core exposes as
// the opaque "TM_OPCUA" extension tree, and ClearBox per optical train under
// Optional_Components/ClearBox/, which core shims into the single File_Version 1.1-layout "ClearBox"
// tree (see models.ClearBoxFromV10Trains).
func parseExtensions(f *h5c.File, trainIDs []string) (map[string]*extension.Tree, error) {
	out := map[string]*extension.Tree{}

	var cbTrains []ClearBoxTrain
	for _, tid := range trainIDs {
		g, err := openClearBox(f, tid)
		if err != nil {
			return nil, err
		}
		if g == nil {
			continue
		}
		tree, err := extio.ReadTree(g, ClearBoxExtensionName+"/"+tid)
		g.Close()
		if err != nil {
			return nil, err
		}
		cbTrains = append(cbTrains, ClearBoxTrain{TrainID: tid, Tree: tree})
	}
	cb, err := ClearBoxFromV10Trains(cbTrains)
	if err != nil {
		return nil, err
	}
	if cb != nil {
		out[ClearBoxExtensionName] = cb
	}

	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()
	if root.LinkExists(layout.RootOPCUA) {
		g, err := f.Group(layout.RootOPCUA)
		if err != nil {
			return nil, err
		}
		defer g.Close()
		name := layout.LegacyOpcuaExtensionName
		tree, err := extio.ReadTree(g, name)
		if err != nil {
			return nil, err
		}
		out[name] = tree
	}
	if len(out) == 0 {
		return nil, nil
	}
	return out, nil
}

// openClearBox opens train tid's Optional_Components/ClearBox group, or returns (nil, nil) when the
// train has none.
func openClearBox(f *h5c.File, tid string) (*h5c.Group, error) {
tg, err := f.Group(layout.TrainPathByID(tid))
if err != nil {
return nil, err
}
defer tg.Close()
if !tg.LinkExists(layout.GroupOptionalComponents) {
return nil, nil
}
oc, err := tg.OpenGroup(layout.GroupOptionalComponents)
if err != nil {
return nil, err
}
defer oc.Close()
if !oc.LinkExists(layout.GroupClearBox) {
return nil, nil
}
return oc.OpenGroup(layout.GroupClearBox)
}