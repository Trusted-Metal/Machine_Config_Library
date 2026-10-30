package machineconfig

import (
	"fmt"
	"math"

	"machine-config-go/capabilities/v1_0/layout"
	"machine-config-go/extension"
	"machine-config-go/internal/models"
)

const (
	mockExportDate    = "2026-01-01T00:00:00.000Z"
	mockSchemaVersion = "v1"
)

// mustPowerCharacterizationCoefficients/Points derive PowerCharacterization
// from fixed, known-valid literals — the forward functions can only fail on
// a genuinely malformed CSV float, which these mock literals never are, so
// the error is asserted away rather than threaded through this
// non-error-returning builder.
func mustPowerCharacterizationCoefficients(algorithmType, params string) *models.PowerCharacterization {
	pc, err := models.ForwardPowerCharacterizationCoefficients(StrPtr(algorithmType), StrPtr(params))
	if err != nil {
		panic(err)
	}
	return pc
}

func mustPowerCharacterizationPoints(algorithmType, params string) *models.PowerCharacterization {
	pc, err := models.ForwardPowerCharacterizationPoints(StrPtr(algorithmType), StrPtr(params))
	if err != nil {
		panic(err)
	}
	return pc
}

// MockConfigBuilder generates deterministic synthetic machine-config files for testing.
// Values are fixed so tests are reproducible across platforms.
type MockConfigBuilder struct {
	NLasers         int
	MachineName     string
	BuildPlateX     float64
	BuildPlateY     float64
	IncludeClearbox bool
}

// NewMockConfigBuilder returns a builder with sensible defaults.
func NewMockConfigBuilder() *MockConfigBuilder {
	return &MockConfigBuilder{
		NLasers:         2,
		MachineName:     "MockMachine",
		BuildPlateX:     250.0,
		BuildPlateY:     250.0,
		IncludeClearbox: true,
	}
}

// Build constructs the in-memory MachineConfig without writing to disk.
func (b *MockConfigBuilder) Build() *MachineConfig {
	meta := MachineConfigMeta{
		SchemaVersion:     mockSchemaVersion,
		MachineName:       b.MachineName,
		Manufacturer:      "MockCo",
		Model:             "MockMIDI+",
		SerialNumber:      "MOCK-001",
		FileVersion:       "1.0",
		ExportDate:        mockExportDate,
		ConfigurationHash: "0000000000000000000000000000000000000000000000000000000000000000",
		Extra:             map[string]any{},
	}

	machineID := "00000000-0000-0000-0000-000000000001"
	machine := Machine{
		ID:                StrPtr(machineID),
		MachineName:       b.MachineName,
		Manufacturer:      "MockCo",
		Model:             "MockMIDI+",
		SerialNumber:      "MOCK-001",
		BuildPlateX:       Float64Ptr(b.BuildPlateX),
		BuildPlateXUnit:   StrPtr("mm"),
		BuildPlateY:       Float64Ptr(b.BuildPlateY),
		BuildPlateYUnit:   StrPtr("mm"),
		BuildPlateZ:       Float64Ptr(20.0),
		BuildPlateZUnit:   StrPtr("mm"),
		GasFlowDirection:  StrPtr("Y+"),
		RecoatDirection:   StrPtr("X+"),
		RecoaterBladeType: StrPtr("Standard"),
	}

	trains := make([]OpticalTrain, b.NLasers)
	for i := range trains {
		trains[i] = b.buildTrain(i)
	}

	config := &MachineConfig{
		Meta:          meta,
		Machine:       machine,
		OpticalTrains: trains,
	}
	if b.IncludeClearbox && b.NLasers > 0 {
		groups := make([]models.ClearBoxTrain, b.NLasers)
		for i := range groups {
			groups[i] = models.ClearBoxTrain{TrainID: trains[i].TrainID, Tree: mockClearboxGroup(i)}
		}
		clearBox, err := models.ClearBoxFromV10Trains(groups)
		if err != nil {
			panic(err) // fixed, known-valid inputs
		}
		config.Extensions = map[string]*extension.Tree{models.ClearBoxExtensionName: clearBox}
	}
	return config
}

// Save builds the config and writes it to path.
func (b *MockConfigBuilder) Save(path string) error {
	return NewWriter().Write(b.Build(), path)
}

func (b *MockConfigBuilder) buildTrain(index int) OpticalTrain {
	sign := -1.0
	if index%2 != 0 {
		sign = 1.0
	}
	rotation := 0.0
	if index%2 != 0 {
		rotation = 180.0
	}

	ax := mockAxis()
	scanner := Scanner{
		Manufacturer:          "MockCo",
		Model:                 "MockScan",
		SerialNumber:          fmt.Sprintf("MOCK-SC-%02d", index+1),
		WorkingDistance:       Float64Ptr(670.0),
		WorkingDistanceUnit:   StrPtr("mm"),
		ScanFieldX:            Float64Ptr(600.0),
		ScanFieldXUnit:        StrPtr("mm"),
		ScanFieldY:            Float64Ptr(600.0),
		ScanFieldYUnit:        StrPtr("mm"),
		ScanFieldZ:            Float64Ptr(76.5),
		ScanFieldZUnit:        StrPtr("mm"),
		ScanHeadOffsetX:       Float64Ptr(sign * 87.5),
		ScanHeadOffsetXUnit:   StrPtr("mm"),
		ScanHeadOffsetY:       Float64Ptr(sign * -23.5),
		ScanHeadOffsetYUnit:   StrPtr("mm"),
		ScanHeadOffsetZ:       Float64Ptr(-1.0),
		ScanHeadOffsetZUnit:   StrPtr("mm"),
		ScanHeadRotation:      Float64Ptr(rotation),
		ScanHeadRotationUnit:  StrPtr("degrees"),
		AxisConfiguration:     StrPtr("3D"),
		XAxis:                 ax,
		YAxis:                 ax,
		ZAxis:                 &ax,
		Focus:                 nil,
	}

	ls := LightSource{
		Manufacturer:          "MockLaser",
		Model:                 "MockFiber-1070",
		SerialNumber:          fmt.Sprintf("MOCK-LS-%02d", index+1),
		Wavelength:            Float64Ptr(1070.0),
		WavelengthUnit:        StrPtr("nm"),
		PowerMaxNominal:       Float64Ptr(1000.0),
		PowerMaxNominalUnit:   StrPtr("W"),
		PowerMaxActual:        Float64Ptr(1020.0),
		PowerMaxActualUnit:    StrPtr("W"),
		PowerMinActual:        Float64Ptr(100.0),
		PowerMinActualUnit:    StrPtr("W"),
		PowerMinNominalUnit:   StrPtr("W"),
		PowerBitResolutionUnit: StrPtr("bits"),
		PowerCharacterization: mustPowerCharacterizationPoints("LINEAR", "[1,100,10,1000]"),
	}

	col := Collimator{
		Manufacturer:    "MockOptics",
		Model:           "D50_F120",
		SerialNumber:    fmt.Sprintf("MOCK-COL-%02d", index+1),
		FocalLength:     Float64Ptr(120.0),
		FocalLengthUnit: StrPtr("mm"),
	}

	sc := ScannerCard{
		Manufacturer:          "Raylase",
		Model:                 "SP-ICE-3",
		SerialNumber:          fmt.Sprintf("MOCK-SC-CARD-%02d", index+1),
		CommunicationProtocol: StrPtr("SL2-100"),
		SamplePeriod:          Float64Ptr(10.0),
		SamplePeriodUnit:      StrPtr("μs"),
	}

	var sfcf *ScanFieldCorrectionFile
	if b.IncludeClearbox {
		s := mockSFCF(index)
		sfcf = &s
	}

	return OpticalTrain{
		TrainID:                          layout.TrainID(index),
		BeamWaistDefinition:              StrPtr("knife-edge"),
		BeamWaistMajor:                   Float64Ptr(67.0),
		BeamWaistMajorUnit:               StrPtr("μm"),
		BeamWaistMinor:                   Float64Ptr(68.0),
		BeamWaistMinorUnit:               StrPtr("μm"),
		BeamWaistOffsetZ:                 Float64Ptr(0.5),
		BeamWaistOffsetZUnit:             StrPtr("mm"),
		BuildPlaneOffsetMajor:            Float64Ptr(0.2),
		BuildPlaneOffsetMajorUnit:        StrPtr("mm"),
		BuildPlaneOffsetMinor:            Float64Ptr(0.7),
		BuildPlaneOffsetMinorUnit:        StrPtr("mm"),
		CollimatorFocalLength:            Float64Ptr(120.0),
		CollimatorFocalLengthUnit:        StrPtr("mm"),
		M2Major:                          Float64Ptr(1.05),
		M2Minor:                          Float64Ptr(1.08),
		MajorAxisAngle:                   Float64Ptr(0.0),
		MajorAxisAngleUnit:               StrPtr("degrees"),
		RayleighLengthMajor:              Float64Ptr(3.1),
		RayleighLengthMajorUnit:          StrPtr("mm"),
		RayleighLengthMinor:              Float64Ptr(3.2),
		RayleighLengthMinorUnit:          StrPtr("mm"),
		ThermalLensingPassed:             BoolPtr(false),
		ThermalLensingFocalPlaneShift:    Float64Ptr(1.0),
		ThermalLensingFocalPlaneShiftUnit: StrPtr("mm"),
		ThermalLensingThreshold:          Float64Ptr(0.75),
		ThermalLensingThresholdUnit:      StrPtr("mm"),
		Scanner:                          scanner,
		LightSource:                      ls,
		Collimator:                       col,
		ScannerCard:                      sc,
		ScanFieldCorrectionFile:          sfcf,
	}
}

func mockAxis() AxisConfig {
	return AxisConfig{
		ActualBitResolution:         IntPtr(20),
		ActualBitResolutionUnit:     StrPtr("bits"),
		CommandedBitResolution:      IntPtr(20),
		CommandedBitResolutionUnit:  StrPtr("bits"),
		SmoothingKernel:             StrPtr("GAUSSIAN"),
		SmoothingParameters:         Float64Ptr(60.0),
		RangeOfMotionUnit:           StrPtr("mm"),
	}
}

// mockClearboxGroup is one train's ClearBox group, authored the way a File_Version 1.0 file stores
// it; Build() runs it through the v1.0 shim like the file reader does.
func mockClearboxGroup(index int) *extension.Tree {
	fwd := gaussianCorrectionGrid()
	inv := make([]float64, len(fwd))
	for i, v := range fwd {
		inv[i] = v * 0.9
	}
	shape := []uint64{257, 257, 2}

	tree := extension.NewTree()
	for name, value := range map[string]string{
		"Ip_Address":               fmt.Sprintf("192.168.1.%d", 10+index),
		"Serial_Number":            fmt.Sprintf("%03d", index+1),
		"Output_Path":              "/recordings/",
		"Selected_Camera":          "Default",
		"Custom_Video_Format":      "MP4",
		"Video_Output":             "HDMI",
		"Volts_To_Watts_Algorithm": "LINEAR",
		"Volts_To_Watts_Params":    "50.0,100.0",
	} {
		tree.Attrs[name] = extension.Attr{Dtype: extension.Utf8, Value: value}
	}
	for name, value := range map[string]int64{
		"Data_Port":               5001,
		"Server_Port":             20101,
		"Actual_Timing_Offset":    -8,
		"Commanded_Timing_Offset": 50,
		"Show_Console":            0,
		"Software_Trigger_Delay":  3000,
	} {
		tree.Attrs[name] = extension.Attr{Dtype: extension.I64, Value: value}
	}
	tree.Datasets["Correction_Data"] = &extension.Dataset{Dtype: extension.F64, Shape: shape, Data: fwd, Attrs: map[string]extension.Attr{}}
	tree.Datasets["Inverse_Correction_Data"] = &extension.Dataset{Dtype: extension.F64, Shape: shape, Data: inv, Attrs: map[string]extension.Attr{}}
	return tree
}

func mockSFCF(index int) ScanFieldCorrectionFile {
	return ScanFieldCorrectionFile{
		DocumentName:  fmt.Sprintf("mock_laser_%d.fc3", index+1),
		DocumentID:    fmt.Sprintf("00000000-0000-0000-0000-%012d", index+1),
		FileSize:      1024,
		ValidAsOfDate: mockExportDate,
		DocumentType:  StrPtr("Scan Field Correction File"),
	}
}

// gaussianCorrectionGrid builds a flat, row-major 257×257×2 Gaussian warp pattern.
// Matches the Rust builder exactly: x/y normalized to [-1,1], warp = 2*exp(-(x²+y²)/0.5).
func gaussianCorrectionGrid() []float64 {
	const n = 257
	grid := make([]float64, n*n*2)
	for i := 0; i < n; i++ {
		x := -1.0 + 2.0*float64(i)/float64(n-1)
		for j := 0; j < n; j++ {
			y := -1.0 + 2.0*float64(j)/float64(n-1)
			warp := 2.0 * math.Exp(-(x*x+y*y)/0.5)
			grid[(i*n+j)*2] = warp
			grid[(i*n+j)*2+1] = warp * 0.8
		}
	}
	return grid
}
