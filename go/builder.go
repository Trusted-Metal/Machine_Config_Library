package machineconfig

import (
	"fmt"
	"math"

	"machine-config-go/capabilities/v1_0/layout"
)

const (
	mockExportDate    = "2026-01-01T00:00:00.000Z"
	mockSchemaVersion = "v1"
)

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
		ID:              StrPtr(machineID),
		MachineName:     b.MachineName,
		Manufacturer:    "MockCo",
		Model:           "MockMIDI+",
		SerialNumber:    "MOCK-001",
		BuildPlateX:     Float64Ptr(b.BuildPlateX),
		BuildPlateXUnit: StrPtr("mm"),
		BuildPlateY:     Float64Ptr(b.BuildPlateY),
		BuildPlateYUnit: StrPtr("mm"),
		BuildPlateZ:     Float64Ptr(20.0),
		BuildPlateZUnit: StrPtr("mm"),
		GasFlowDirection: StrPtr("Y+"),
		RecoatDirection:  StrPtr("X+"),
	}

	trains := make([]OpticalTrain, b.NLasers)
	for i := range trains {
		trains[i] = b.buildTrain(i)
	}

	return &MachineConfig{
		Meta:          meta,
		Machine:       machine,
		OpticalTrains: trains,
		Opcua:         nil,
	}
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
		WattsToVoltsAlgorithm: StrPtr("LINEAR"),
		WattsToVoltsParams:    StrPtr("[1,100,10,1000]"),
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

	var cb *ClearBox
	var sfcf *ScanFieldCorrectionFile
	if b.IncludeClearbox {
		c := mockClearbox(index)
		cb = &c
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
		OptionalComponents:               OptionalComponents{Clearbox: cb},
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

func mockClearbox(index int) ClearBox {
	fwd := gaussianCorrectionGrid()
	inv := scaleGrid(fwd, 0.9)
	return ClearBox{
		IPAddress:             fmt.Sprintf("192.168.1.%d", 10+index),
		SerialNumber:          StrPtr(fmt.Sprintf("%03d", index+1)),
		DataPort:              IntPtr(5001),
		ServerPort:            IntPtr(20101),
		ActualTimingOffset:    IntPtr(-8),
		CommandedTimingOffset: IntPtr(50),
		CorrectionData:        &fwd,
		InverseCorrectionData: &inv,
		OutputPath:            StrPtr("/recordings/"),
		SelectedCamera:        StrPtr("Default"),
		CustomVideoFormat:     StrPtr("MP4"),
		VideoOutput:           StrPtr("HDMI"),
		ShowConsole:           BoolPtr(false),
		SoftwareTriggerDelay:  IntPtr(3000),
		VoltsToWattsAlgorithm: StrPtr("LINEAR"),
		VoltsToWattsParams:    StrPtr("50.0,100.0"),
	}
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

// gaussianCorrectionGrid builds a 257×257×2 Gaussian warp pattern.
// Matches the Rust builder exactly: x/y normalized to [-1,1], warp = 2*exp(-(x²+y²)/0.5).
func gaussianCorrectionGrid() [][][]*float64 {
	const n = 257
	grid := make([][][]*float64, n)
	for i := 0; i < n; i++ {
		x := -1.0 + 2.0*float64(i)/float64(n-1)
		grid[i] = make([][]*float64, n)
		for j := 0; j < n; j++ {
			y := -1.0 + 2.0*float64(j)/float64(n-1)
			warp := 2.0 * math.Exp(-(x*x+y*y)/0.5)
			v0, v1 := warp, warp*0.8
			grid[i][j] = []*float64{&v0, &v1}
		}
	}
	return grid
}

// scaleGrid multiplies every non-nil cell by factor.
func scaleGrid(src [][][]*float64, factor float64) [][][]*float64 {
	dst := make([][][]*float64, len(src))
	for i, row := range src {
		dst[i] = make([][]*float64, len(row))
		for j, col := range row {
			dst[i][j] = make([]*float64, len(col))
			for k, cell := range col {
				if cell == nil {
					dst[i][j][k] = nil
				} else {
					v := *cell * factor
					dst[i][j][k] = &v
				}
			}
		}
	}
	return dst
}
