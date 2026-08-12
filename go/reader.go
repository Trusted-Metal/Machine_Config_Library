package machineconfig

import (
	"fmt"
	"log"
	"math"
	"strings"

	"machine-config-go/internal/h5c"
)

const schemaVersion = "v1"

// MachineConfigReader reads LPBF machine-config HDF5 files into MachineConfig.
type MachineConfigReader struct {
	path string
}

// NewReader constructs a reader for path.
func NewReader(path string) *MachineConfigReader {
	return &MachineConfigReader{path: path}
}

// ParseOptions controls binary dataset loading.
type ParseOptions struct {
	IncludeBinary bool
}

// Parse opens the file and returns a populated MachineConfig (scalars + nested groups).
// Binary correction grids / fc3 bytes are omitted unless IncludeBinary is set via ParseWithOptions.
func (r *MachineConfigReader) Parse() (*MachineConfig, error) {
	return r.ParseWithOptions(ParseOptions{})
}

// ParseWithOptions is Parse with explicit options.
func (r *MachineConfigReader) ParseWithOptions(opts ParseOptions) (*MachineConfig, error) {
	f, err := h5c.Open(r.path)
	if err != nil {
		return nil, fmt.Errorf("open %q: %w", r.path, err)
	}
	defer f.Close()
	return r.parse(f, opts)
}

func (r *MachineConfigReader) parse(f *h5c.File, opts ParseOptions) (*MachineConfig, error) {
	root, err := f.Root()
	if err != nil {
		return nil, err
	}
	defer root.Close()

	fv := readRequiredStr(root, "File_Version")
	if strings.TrimSpace(fv) != "1.0" {
		log.Printf("warning: File_Version is %q; this reader targets \"1.0\"", fv)
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
		Extra:             map[string]any{},
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

	trainsGrp, err := f.Group("Machine/Optical_Trains")
	if err != nil {
		return nil, err
	}
	defer trainsGrp.Close()

	var trains []OpticalTrain
	for i := 1; ; i++ {
		tid := fmt.Sprintf("Optical_Train_%02d", i)
		if !trainsGrp.LinkExists(tid) {
			break
		}
		train, err := parseTrain(f, tid, opts)
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

func parseTrain(f *h5c.File, trainID string, opts ParseOptions) (OpticalTrain, error) {
	base := "Machine/Optical_Trains/" + trainID
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
			cb, err := parseClearBox(f, base+"/Optional_Components/ClearBox", cbg, opts)
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
		ds, err := g.OpenDataset("scan_field_correction_file")
		if err == nil {
			// attributes live on the dataset — open as group path via file
			ds.Close()
		}
		// Dataset attrs: open via a dedicated path helper — use Machine group Link
		sfcf, err = parseSFCF(f, base+"/scan_field_correction_file", opts)
		if err != nil {
			return OpticalTrain{}, err
		}
	}

	tlp, err := readBoolFromIntAttr(g, "Thermal_Lensing_Passed")
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
	return Scanner{
		Manufacturer:          readRequiredStr(g, "Manufacturer"),
		Model:                 readRequiredStr(g, "Model"),
		SerialNumber:          readRequiredStr(g, "Serial_Number"),
		WorkingDistance:       wd,
		WorkingDistanceUnit:   wdu,
		ScanFieldX:            mustFloat(g, "Scan_Field_Size_X"),
		ScanFieldXUnit:        readStrAttr(g, "Scan_Field_Size_X_unit"),
		ScanFieldY:            mustFloat(g, "Scan_Field_Size_Y"),
		ScanFieldYUnit:        readStrAttr(g, "Scan_Field_Size_Y_unit"),
		ScanFieldZ:            mustFloat(g, "Scan_Field_Size_Z"),
		ScanFieldZUnit:        readStrAttr(g, "Scan_Field_Size_Z_unit"),
		ScanHeadOffsetX:       mustFloat(g, "Scan_Head_Offset_X"),
		ScanHeadOffsetXUnit:   readStrAttr(g, "Scan_Head_Offset_X_unit"),
		ScanHeadOffsetY:       mustFloat(g, "Scan_Head_Offset_Y"),
		ScanHeadOffsetYUnit:   readStrAttr(g, "Scan_Head_Offset_Y_unit"),
		ScanHeadOffsetZ:       mustFloat(g, "Scan_Head_Offset_Z"),
		ScanHeadOffsetZUnit:   readStrAttr(g, "Scan_Head_Offset_Z_unit"),
		ScanHeadRotation:      mustFloat(g, "Scan_Head_Rotation"),
		ScanHeadRotationUnit:  readStrAttr(g, "Scan_Head_Rotation_unit"),
		AxisConfiguration:     readStrAttr(g, "Axis_Configuration"),
		XAxis:                 xa,
		YAxis:                 ya,
		ZAxis:                 za,
		Focus:                 focus,
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
	rom, err := readFloatAttr(g, "Range_of_Motion")
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
		RangeOfMotionUnit:          readStrAttr(g, "Range_of_Motion_unit"),
		SmoothingKernel:            readStrAttr(g, "Smoothing_Kernel"),
		SmoothingParameters:        sp,
		TuningParameters:           readStrAttr(g, "Tuning_Parameters"),
		TuningType:                 readStrAttr(g, "Tuning_Type"),
	}, nil
}

func parseLightSource(g *h5c.Group) (LightSource, error) {
	return LightSource{
		Manufacturer:            readRequiredStr(g, "Manufacturer"),
		Model:                   readRequiredStr(g, "Model"),
		SerialNumber:            readRequiredStr(g, "Serial_Number"),
		Wavelength:              mustFloat(g, "Light_Wavelength"),
		WavelengthUnit:          readStrAttr(g, "Light_Wavelength_unit"),
		PowerMaxNominal:         mustFloat(g, "Power_Max_Nominal"),
		PowerMaxNominalUnit:     readStrAttr(g, "Power_Max_Nominal_unit"),
		PowerMaxActual:          mustFloat(g, "Power_Max_Actual"),
		PowerMaxActualUnit:      readStrAttr(g, "Power_Max_Actual_unit"),
		PowerMinActual:          mustFloat(g, "Power_Min_Actual"),
		PowerMinActualUnit:      readStrAttr(g, "Power_Min_Actual_unit"),
		PowerMinNominal:         mustFloat(g, "Power_Min_Nominal"),
		PowerMinNominalUnit:     readStrAttr(g, "Power_Min_Nominal_unit"),
		PowerBitResolution:      mustFloat(g, "Power_Bit_Resolution"),
		PowerBitResolutionUnit:  readStrAttr(g, "Power_Bit_Resolution_unit"),
		WattsToVoltsAlgorithm:   readStrAttr(g, "Watts_To_Volts_Algorithm"),
		WattsToVoltsParams:      readStrAttr(g, "Watts_To_Volts_Params"),
	}, nil
}

func parseCollimator(g *h5c.Group) (Collimator, error) {
	fl, err := readFloatAttr(g, "Focal_Length")
	if err != nil {
		return Collimator{}, err
	}
	flu, _ := readStrLocked(g, "Focal_Length_unit", "mm")
	return Collimator{
		Manufacturer:     readRequiredStr(g, "Manufacturer"),
		Model:            readRequiredStr(g, "Model"),
		SerialNumber:     readRequiredStr(g, "Serial_Number"),
		FocalLength:      fl,
		FocalLengthUnit:  flu,
	}, nil
}

func parseScannerCard(g *h5c.Group) (ScannerCard, error) {
	sp, err := readFloatAttr(g, "Sample_Period")
	if err != nil {
		return ScannerCard{}, err
	}
	return ScannerCard{
		Manufacturer:           readRequiredStr(g, "Manufacturer"),
		Model:                  readRequiredStr(g, "Model"),
		SerialNumber:           readRequiredStr(g, "Serial_Number"),
		CommunicationProtocol:  readStrAttr(g, "Communication_Protocol"),
		SamplePeriod:           sp,
		SamplePeriodUnit:       readStrAttr(g, "Sample_Period_unit"),
	}, nil
}

func parseClearBox(f *h5c.File, path string, g *h5c.Group, opts ParseOptions) (ClearBox, error) {
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
	cb := ClearBox{
		IPAddress:              readRequiredStr(g, "Ip_Address"),
		SerialNumber:           readStrAttr(g, "Serial_Number"),
		DataPort:               dp,
		ServerPort:             sp,
		ActualTimingOffset:     ato,
		CommandedTimingOffset:  cto,
		Manufacturer:           readStrAttr(g, "Manufacturer"),
		Model:                  readStrAttr(g, "Model"),
		OutputPath:             readStrAttr(g, "Output_Path"),
		SelectedCamera:         readStrAttr(g, "Selected_Camera"),
		CustomVideoFormat:      readStrAttr(g, "Custom_Video_Format"),
		VideoOutput:            readStrAttr(g, "Video_Output"),
		ShowConsole:            sc,
		SoftwareTriggerDelay:   std,
		VoltsToWattsAlgorithm:  readStrAttr(g, "Volts_To_Watts_Algorithm"),
		VoltsToWattsParams:     readStrAttr(g, "Volts_To_Watts_Params"),
		CorrectionGridDomainShape: readStrAttr(g, "Correction_Grid_Domain_Shape"),
		InverseGridDomainShape:    readStrAttr(g, "Inverse_Grid_Domain_Shape"),
	}
	if opts.IncludeBinary {
		if grid, err := readFloatGrid(f, path+"/Correction_Data"); err == nil {
			cb.CorrectionData = grid
		}
		if grid, err := readFloatGrid(f, path+"/Inverse_Correction_Data"); err == nil {
			cb.InverseCorrectionData = grid
		}
	}
	return cb, nil
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
	out := make([][][]*float64, dims[0])
	idx := 0
	for i := range out {
		out[i] = make([][]*float64, dims[1])
		for j := range out[i] {
			out[i][j] = make([]*float64, dims[2])
			for k := range out[i][j] {
				v := flat[idx]
				idx++
				if math.IsNaN(v) {
					out[i][j][k] = nil
				} else {
					vv := v
					out[i][j][k] = &vv
				}
			}
		}
	}
	return &out, nil
}

func parseSFCF(f *h5c.File, path string, opts ParseOptions) (*ScanFieldCorrectionFile, error) {
	// Dataset attributes: open dataset and... our h5c doesn't expose dataset attrs yet.
	// For v1 scaffold, open via a temporary approach — attributes on datasets need API.
	// Skip detailed SFCF attr parse until Dataset.HasAttr is added; return placeholder
	// with document name from path if dataset exists.
	ds, err := f.OpenDataset(path)
	if err != nil {
		return nil, nil
	}
	defer ds.Close()
	sfcf := &ScanFieldCorrectionFile{
		DocumentName: "scan_field_correction_file",
	}
	if opts.IncludeBinary {
		dims, err := ds.Dims()
		if err == nil && len(dims) == 1 {
			buf := make([]byte, dims[0])
			if err := ds.ReadUint8(buf); err == nil {
				sfcf.RawBytes = buf
				sfcf.FileSize = int(dims[0])
			}
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
	client := OpcuaClientConfig{
		ServerURL:        readRequiredStr(cg, "Server_URL"),
		AuthMode:         readRequiredStr(cg, "Auth_Mode"),
		SecurityMode:     readRequiredStr(cg, "Security_Mode"),
		SecurityPolicy:   readRequiredStr(cg, "Security_Policy"),
		Extra:            map[string]any{},
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
			pg.Close()
		}
	}

	triggers := map[string]OpcuaTrigger{}
	var triggersEnabled *bool
	if og.LinkExists("Triggers") {
		tg, err := og.OpenGroup("Triggers")
		if err == nil {
			if b, err := readBoolFromIntAttr(tg, "Triggers_Enabled"); err == nil {
				triggersEnabled = b
			}
			// Named trigger subgroups are not enumerated yet (needs H5Literate).
			tg.Close()
		}
	}

	return &OpcuaConfig{
		Client:          client,
		Pipe:            pipe,
		Triggers:        triggers,
		TriggersEnabled: triggersEnabled,
	}, nil
}

// GetCorrectionData loads the ClearBox Correction_Data grid for train index.
func (r *MachineConfigReader) GetCorrectionData(trainIndex int) (*CorrectionData, error) {
	path := fmt.Sprintf(
		"Machine/Optical_Trains/Optical_Train_%02d/Optional_Components/ClearBox/Correction_Data",
		trainIndex+1,
	)
	return r.readCorrection(path)
}

func (r *MachineConfigReader) readCorrection(path string) (*CorrectionData, error) {
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
