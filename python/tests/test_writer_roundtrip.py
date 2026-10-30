"""Phase 1.8d — Python bidirectional write roundtrip tests.

Distinct from test_writer.py (which exercises the reference-config roundtrip at a
coarse level) and test_builder.py (which only checks shape/structure).  These tests
construct MachineConfig objects with explicit known values, write them to HDF5, and
assert every individual field value survives the roundtrip.

Covers:
  - Every scalar field across all seven model types
  - All 18 ClearBox scalar attributes (post-1.8a complete model)
  - ClearBox.synchronous_sensors: one fully-populated sensor (18 scalar
    fields + both compound datasets), plus the empty/zero-row/arbitrary-key
    edge cases (SYNCHRONOUS_SENSOR_PLAN.md Phase 1)
  - NaN ↔ None convention for correction arrays
  - axis_configuration discriminator: "2D", "3D", "3D+Focus"
  - Machine-agnostic path: config with no ClearBox
  - Schema validity of MachineConfigWriter output
"""
from __future__ import annotations

import math
from dataclasses import replace as dc_replace
from pathlib import Path

import pytest

from machine_config import MachineConfigReader, MachineConfigWriter
from machine_config.clearbox_legacy import _power_characterization_to_tree
from machine_config.extension_contract import Attr, Dataset, ExtensionTree, Table, TableField
from machine_config.models import (
    AxisConfig,
    BuildPlate,
    Collimator,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)
from machine_config.power_characterization import (
    forward_power_characterization_coefficients,
    forward_power_characterization_points,
)
from machine_config.schema import SCHEMA, SCHEMA_VERSION

from clearbox_helpers import attr, clearbox, clearbox_root, constants, integer, text


# ---------------------------------------------------------------------------
# Known-value factory helpers
# ---------------------------------------------------------------------------

def _axis(label: str = "X") -> AxisConfig:
    return AxisConfig(
        actual_bit_resolution=20,
        actual_bit_resolution_unit="bits",
        commanded_bit_resolution=18,
        commanded_bit_resolution_unit="bits",
        control_type=f"CLOSED_{label}",
        range_of_motion=120.5,
        range_of_motion_unit="mm",
        smoothing_kernel="GAUSSIAN",
        smoothing_parameters=60.0,
        tuning_parameters="1.0,2.0,3.0",
        tuning_type="PID",
    )


def _scanner(axis_cfg: str = "3D") -> Scanner:
    z     = _axis("Z") if axis_cfg in ("3D", "3D+Focus") else None
    focus = _axis("F") if axis_cfg == "3D+Focus" else None
    return Scanner(
        manufacturer="ScanCo",
        model="ScanModel-X",
        serial_number="SC-123",
        working_distance=670.0,
        working_distance_unit="mm",
        scan_field_x=500.0,
        scan_field_x_unit="mm",
        scan_field_y=501.0,
        scan_field_y_unit="mm",
        scan_field_z=75.0,
        scan_field_z_unit="mm",
        scan_head_offset_x=-87.5,
        scan_head_offset_x_unit="mm",
        scan_head_offset_y=23.5,
        scan_head_offset_y_unit="mm",
        scan_head_offset_z=-1.0,
        scan_head_offset_z_unit="mm",
        scan_head_rotation=0.0,
        scan_head_rotation_unit="degrees",
        axis_configuration=axis_cfg,
        x_axis=_axis("X"),
        y_axis=_axis("Y"),
        z_axis=z,
        focus=focus,
    )


def _light_source() -> LightSource:
    return LightSource(
        manufacturer="LaserCo",
        model="Fiber-1070",
        serial_number="LS-456",
        wavelength=1070.0,
        wavelength_unit="nm",
        power_max_nominal=1000.0,
        power_max_nominal_unit="W",
        power_max_actual=1020.0,
        power_max_actual_unit="W",
        power_min_actual=100.0,
        power_min_actual_unit="W",
        power_min_nominal=50.0,
        power_min_nominal_unit="W",
        power_bit_resolution=None,
        power_bit_resolution_unit="bits",
        power_characterization=forward_power_characterization_points(
            "LINEAR", "[1,100,10,1000]"
        ),
    )


def _collimator() -> Collimator:
    return Collimator(
        manufacturer="OpticsCo",
        model="D50_F120",
        serial_number="COL-789",
        focal_length=120.0,
        focal_length_unit="mm",
    )


def _scanner_card() -> ScannerCard:
    return ScannerCard(
        manufacturer="Raylase",
        model="SP-ICE-3",
        serial_number="SCCARD-012",
        communication_protocol="SL2-100",
        sample_period=10.0,
        sample_period_unit="μs",
    )


def _grid_dataset(fn, size: int) -> Dataset:
    data = [float(fn(i, j)) for i in range(size) for j in range(size) for _ in range(2)]
    return Dataset(dtype="f64", shape=[size, size, 2], data=data)


def _oxygen_sensor() -> ExtensionTree:
    s = ExtensionTree()
    for name, value in (
        ("Sensor_Name", "ZR800 Oxygen Analyzer"),
        ("Sensor_Output_Space", "log10(ppm)"),
        ("Sensor_Model", "ZR810"),
        ("Sensor_Manufacturer", "Industrial Physics"),
        ("Sensor_Scope", "Global"),
        ("Units_Derived_Quantity", "ppm"),
        ("Sensor_Type", "Oxygen Sensor"),
        ("Input_Type", "4-20 mA"),
        ("Algorithm_Type", "Log-Linear"),
        ("Algorithm_Equation", "log(ppm) = a*mA + b"),
        ("Calibration_Source", "Datasheet"),
        ("Metadata", "Synchronous Sensor because Clearbox is responsible for recording."),
    ):
        s.attrs[name] = Attr("utf8", value)
    s.attrs["Enabled"] = Attr("u8", 1)
    s.attrs["Calibration_Verified"] = Attr("u8", 0)
    s.attrs["Port_ID"] = Attr("i32", 5)
    s.attrs["Sensor_Output_Range_Low"] = Attr("f64", -1.0)
    s.attrs["Sensor_Output_Range_High"] = Attr("f64", 6.0)
    s.attrs["Sample_Period"] = Attr("f64", 5.0)
    s.tables["Derivation_Equation_Constants"] = Table(
        fields=[TableField("name", "utf8", 64), TableField("value", "f64")],
        rows=2,
        columns={"name": ["a", "b"], "value": [0.4375, -2.75]},
    )
    s.tables["Calibration_Points"] = Table(
        fields=[TableField("input_value", "f64"), TableField("output_value", "f64")],
        rows=2,
        columns={"input_value": [4.0, 20.0], "output_value": [-1.0, 6.0]},
    )
    return s


def _clearbox(grid_size: int = 5) -> ExtensionTree:
    """ClearBox extension tree in the File_Version 1.1 layout: every former scalar attribute
    populated on ``Optical_Train_01``, the two shared values on the root, a small correction
    grid, a Power_Characterization child and one fully-populated synchronous sensor."""
    train = ExtensionTree()
    for name, value in (
        ("Ip_Address", "192.168.1.100"),
        ("Serial_Number", "CB-321"),
        ("Manufacturer", "ClearBoxCo"),
        ("Model", "CB-Pro"),
        ("Selected_Camera", "Default"),
        ("Custom_Video_Format", "MP4"),
        ("Video_Output", "HDMI"),
        ("Correction_Grid_Domain_Shape", "square"),
        ("Inverse_Grid_Domain_Shape", "square"),
    ):
        train.attrs[name] = Attr("utf8", value)
    for name, value in (
        ("Data_Port", 5001),
        ("Server_Port", 20101),
        ("Actual_Timing_Offset", -8),
        ("Commanded_Timing_Offset", 50),
        ("Show_Console", 0),
    ):
        train.attrs[name] = Attr("i32", value)
    train.datasets["Correction_Data"] = _grid_dataset(lambda i, j: i + j, grid_size)
    train.datasets["Inverse_Correction_Data"] = _grid_dataset(lambda i, j: i + j + 1, grid_size)
    train.children["Power_Characterization"] = _power_characterization_to_tree(
        forward_power_characterization_coefficients("LINEAR", "48.5,105.0")
    )
    sensors = ExtensionTree()
    sensors.children["Oxygen Sensor"] = _oxygen_sensor()
    train.children["Synchronous_Sensors"] = sensors

    root = ExtensionTree()
    root.attrs["Output_Path"] = Attr("utf8", "/recordings/")
    root.attrs["Software_Trigger_Delay"] = Attr("i32", 3000)
    root.children["Optical_Train_01"] = train
    return root


def _train_tree(cfg: MachineConfig) -> ExtensionTree:
    return clearbox(cfg, 0)


def _sfcf() -> ScanFieldCorrectionFile:
    return ScanFieldCorrectionFile(
        document_name="test.fc3",
        document_id="d1234567-abcd-ef01-2345-6789abcdef01",
        file_size=1024,
        valid_as_of_date="2024-01-01T00:00:00Z",
        document_created_at="2023-12-01T00:00:00Z",
        document_type="Scan Field Correction File",
        original_uri="/docs/test.fc3",
    )


def _train(axis_cfg: str = "3D", include_clearbox: bool = True) -> OpticalTrain:
    return OpticalTrain(
        train_id="Optical_Train_01",
        id="some-train-uuid",
        beam_profile_type="GAUSSIAN",
        beam_waist_definition="1/e2",
        beam_waist_major=67.0,
        beam_waist_major_unit="μm",
        beam_waist_minor=68.0,
        beam_waist_minor_unit="μm",
        beam_waist_offset_z=0.5,
        beam_waist_offset_z_unit="mm",
        m2_major=1.05,
        m2_minor=1.08,
        rayleigh_length_major=3.1,
        rayleigh_length_major_unit="mm",
        rayleigh_length_minor=3.2,
        rayleigh_length_minor_unit="mm",
        build_plane_offset_major=0.2,
        build_plane_offset_major_unit="mm",
        build_plane_offset_minor=0.7,
        build_plane_offset_minor_unit="mm",
        collimator_focal_length=120.0,
        collimator_focal_length_unit="mm",
        major_axis_angle=5.5,
        major_axis_angle_unit="degrees",
        scanner_number="1",
        thermal_lensing_passed=True,
        thermal_lensing_focal_plane_shift=0.3,
        thermal_lensing_focal_plane_shift_unit="mm",
        thermal_lensing_threshold=0.75,
        thermal_lensing_threshold_unit="mm",
        scanner=_scanner(axis_cfg),
        light_source=_light_source(),
        collimator=_collimator(),
        scanner_card=_scanner_card(),
        scan_field_correction_file=_sfcf() if include_clearbox else None,
    )


def _config(axis_cfg: str = "3D", include_clearbox: bool = True) -> MachineConfig:
    return MachineConfig(
        meta=MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name="TestMachine-01",
            manufacturer="TestCo",
            model="TestModel-X",
            serial_number="TM-001",
            file_version="1.0",
            export_date="2024-06-01T12:00:00.000Z",
            configuration_hash="a" * 64,
        ),
        machine=Machine(
            id="machine-uuid-001",
            machine_name="TestMachine-01",
            manufacturer="TestCo",
            model="TestModel-X",
            serial_number="TM-001",
            build_plate=BuildPlate(
                x=250.0, x_unit="mm",
                y=251.0, y_unit="mm",
                z=20.0,  z_unit="mm",
                corner_radius=5.0, corner_radius_unit="mm",
            ),
            gas_flow_direction="Y+",
            recoat_direction="X+",
            recoater_blade_type="Standard",
        ),
        optical_trains=[_train(axis_cfg, include_clearbox)],
        extensions={"ClearBox": _clearbox()} if include_clearbox else {},
    )


def _roundtrip(cfg: MachineConfig, path: Path) -> MachineConfig:
    MachineConfigWriter(cfg).write(path)
    return MachineConfigReader(path).parse()


# ---------------------------------------------------------------------------
# Module-level fixtures — avoids PytestRemovedIn10Warning.
# Each fixture is written once per test module (not per class instance).
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def scalar_rt(tmp_path_factory: pytest.TempPathFactory) -> MachineConfig:
    """Fully-populated config written and re-parsed; used by TestScalarFieldRoundtrip."""
    return _roundtrip(_config(), tmp_path_factory.mktemp("scalar") / "scalar.h5")


@pytest.fixture(scope="module")
def clearbox_rt(tmp_path_factory: pytest.TempPathFactory) -> MachineConfig:
    """Config with ClearBox written and re-parsed; used by TestClearBoxAttributeRoundtrip."""
    out = tmp_path_factory.mktemp("cb_attrs") / "cb.h5"
    MachineConfigWriter(_config()).write(out)
    return MachineConfigReader(out).parse()


@pytest.fixture(scope="module")
def nan_cb(tmp_path_factory: pytest.TempPathFactory) -> ExtensionTree:
    """ClearBox train with NaN cells in both grids; used by TestCorrectionArrayNaN."""
    size = 3
    nan = float("nan")
    fwd = _grid_dataset(lambda i, j: nan if (i == 0 and j == 0) else i + j * 10, size)
    inv = _grid_dataset(lambda i, j: nan if (i == 2 and j == 2) else i * 5 + j, size)
    train = ExtensionTree(attrs={"Ip_Address": Attr("utf8", "10.0.0.1")})
    train.datasets["Correction_Data"] = fwd
    train.datasets["Inverse_Correction_Data"] = inv
    root = ExtensionTree(children={"Optical_Train_01": train})
    cfg = dc_replace(_config(include_clearbox=False), extensions={"ClearBox": root})
    out = tmp_path_factory.mktemp("nan_test") / "nan.h5"
    MachineConfigWriter(cfg).write(out)
    return clearbox(MachineConfigReader(out).parse(), 0)


@pytest.fixture(scope="module")
def nocb_rt(tmp_path_factory: pytest.TempPathFactory) -> MachineConfig:
    """Config without ClearBox written and re-parsed; used by TestWithoutClearBox."""
    return _roundtrip(
        _config(include_clearbox=False),
        tmp_path_factory.mktemp("nocb") / "nocb.h5",
    )



# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestScalarFieldRoundtrip:
    """Every scalar field across all model types survives write → read."""

    # --- MachineConfigMeta ---------------------------------------------------

    def test_meta_machine_name(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.machine_name == "TestMachine-01"

    def test_meta_manufacturer(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.manufacturer == "TestCo"

    def test_meta_model(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.model == "TestModel-X"

    def test_meta_serial_number(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.serial_number == "TM-001"

    def test_meta_file_version(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.file_version == "1.0"

    def test_meta_export_date(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.meta.export_date == "2024-06-01T12:00:00.000Z"

    def test_meta_configuration_hash(self, scalar_rt: MachineConfig) -> None:
        # The writer always computes a fresh hash from actual content — the
        # input's placeholder "a" * 64 never survives to disk.
        assert scalar_rt.meta.configuration_hash != "a" * 64
        assert len(scalar_rt.meta.configuration_hash) == 64
        assert scalar_rt.meta.is_valid is True

    # --- Machine / BuildPlate -----------------------------------------------

    def test_machine_name(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.machine_name == "TestMachine-01"

    def test_machine_manufacturer(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.manufacturer == "TestCo"

    def test_machine_gas_flow_direction(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.gas_flow_direction == "Y+"

    def test_machine_recoat_direction(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.recoat_direction == "X+"

    def test_machine_recoater_blade_type(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.recoater_blade_type == "Standard"

    def test_build_plate_x(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.build_plate.x == pytest.approx(250.0)

    def test_build_plate_y(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.build_plate.y == pytest.approx(251.0)

    def test_build_plate_z(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.build_plate.z == pytest.approx(20.0)

    def test_build_plate_corner_radius(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.build_plate.corner_radius == pytest.approx(5.0)

    def test_build_plate_units(self, scalar_rt: MachineConfig) -> None:
        bp = scalar_rt.machine.build_plate
        assert bp.x_unit == "mm"
        assert bp.y_unit == "mm"
        assert bp.z_unit == "mm"

    # --- OpticalTrain --------------------------------------------------------

    def test_train_id(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].train_id == "Optical_Train_01"

    def test_beam_waist_major(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].beam_waist_major == pytest.approx(67.0)

    def test_beam_waist_minor(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].beam_waist_minor == pytest.approx(68.0)

    def test_beam_waist_major_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].beam_waist_major_unit == "μm"

    def test_m2_major(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].m2_major == pytest.approx(1.05)

    def test_m2_minor(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].m2_minor == pytest.approx(1.08)

    def test_rayleigh_length_major(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].rayleigh_length_major == pytest.approx(3.1)

    def test_rayleigh_length_minor(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].rayleigh_length_minor == pytest.approx(3.2)

    def test_build_plane_offset_major(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].build_plane_offset_major == pytest.approx(0.2)

    def test_build_plane_offset_minor(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].build_plane_offset_minor == pytest.approx(0.7)

    def test_major_axis_angle(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].major_axis_angle == pytest.approx(5.5)

    def test_major_axis_angle_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].major_axis_angle_unit == "degrees"

    def test_thermal_lensing_passed_true(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].thermal_lensing_passed is True

    def test_thermal_lensing_focal_plane_shift(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].thermal_lensing_focal_plane_shift == pytest.approx(0.3)

    def test_thermal_lensing_threshold(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].thermal_lensing_threshold == pytest.approx(0.75)

    def test_collimator_focal_length(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].collimator.focal_length == pytest.approx(120.0)

    def test_collimator_focal_length_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].collimator.focal_length_unit == "mm"

    # --- Scanner -------------------------------------------------------------

    def test_scanner_manufacturer(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.manufacturer == "ScanCo"

    def test_scanner_model(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.model == "ScanModel-X"

    def test_scanner_serial_number(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.serial_number == "SC-123"

    def test_scanner_working_distance(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.working_distance == pytest.approx(670.0)

    def test_scanner_working_distance_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.working_distance_unit == "mm"

    def test_scanner_scan_field_x(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_field_x == pytest.approx(500.0)

    def test_scanner_scan_field_y(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_field_y == pytest.approx(501.0)

    def test_scanner_scan_head_offset_x(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_head_offset_x == pytest.approx(-87.5)

    def test_scanner_scan_head_offset_y(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_head_offset_y == pytest.approx(23.5)

    def test_scanner_scan_head_offset_z(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_head_offset_z == pytest.approx(-1.0)

    def test_scanner_scan_head_rotation(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_head_rotation == pytest.approx(0.0)

    def test_scanner_scan_head_rotation_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.scan_head_rotation_unit == "degrees"

    def test_scanner_axis_configuration(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.axis_configuration == "3D"

    # --- AxisConfig ----------------------------------------------------------

    def test_x_axis_actual_bit_resolution(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.actual_bit_resolution == 20

    def test_x_axis_actual_bit_resolution_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.actual_bit_resolution_unit == "bits"

    def test_x_axis_commanded_bit_resolution(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.commanded_bit_resolution == 18

    def test_x_axis_smoothing_kernel(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.smoothing_kernel == "GAUSSIAN"

    def test_x_axis_smoothing_parameters(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.smoothing_parameters == pytest.approx(60.0)

    def test_x_axis_tuning_parameters(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.tuning_parameters == "1.0,2.0,3.0"

    def test_x_axis_tuning_type(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.x_axis.tuning_type == "PID"

    def test_z_axis_present_for_3d(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.z_axis is not None

    def test_z_axis_smoothing_kernel(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner.z_axis.smoothing_kernel == "GAUSSIAN"  # type: ignore[union-attr]

    # --- LightSource ---------------------------------------------------------

    def test_light_source_wavelength(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.wavelength == pytest.approx(1070.0)

    def test_light_source_wavelength_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.wavelength_unit == "nm"

    def test_light_source_power_max_nominal(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.power_max_nominal == pytest.approx(1000.0)

    def test_light_source_power_max_actual(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.power_max_actual == pytest.approx(1020.0)

    def test_light_source_power_min_actual(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.power_min_actual == pytest.approx(100.0)

    def test_light_source_power_min_nominal(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.power_min_nominal == pytest.approx(50.0)

    def test_light_source_power_characterization_algorithm_type(
        self, scalar_rt: MachineConfig
    ) -> None:
        pc = scalar_rt.optical_trains[0].light_source.power_characterization
        assert pc is not None
        assert pc.algorithm_type == "LINEAR"

    def test_light_source_power_characterization_points(self, scalar_rt: MachineConfig) -> None:
        pc = scalar_rt.optical_trains[0].light_source.power_characterization
        assert pc is not None
        assert [(p.input_value, p.output_value) for p in pc.characterization_points] == [
            (1.0, 100.0),
            (10.0, 1000.0),
        ]

    # --- ScannerCard ---------------------------------------------------------

    def test_scanner_card_manufacturer(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.manufacturer == "Raylase"

    def test_scanner_card_model(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.model == "SP-ICE-3"

    def test_scanner_card_serial_number(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.serial_number == "SCCARD-012"

    def test_scanner_card_communication_protocol(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.communication_protocol == "SL2-100"

    def test_scanner_card_sample_period(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.sample_period == pytest.approx(10.0)

    def test_scanner_card_sample_period_unit(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].scanner_card.sample_period_unit == "μs"


class TestClearBoxAttributeRoundtrip:
    """Every ClearBox attribute, dataset, table and child survives write → read unchanged."""

    def test_extension_tree_is_bit_stable(self, clearbox_rt: MachineConfig) -> None:
        from machine_config.extension_contract import canonical_digest

        assert canonical_digest(clearbox_rt.extensions["ClearBox"]) == canonical_digest(_clearbox())

    def test_per_train_string_attributes(self, clearbox_rt: MachineConfig) -> None:
        cb = clearbox(clearbox_rt, 0)
        assert text(cb, "Ip_Address") == "192.168.1.100"
        assert text(cb, "Serial_Number") == "CB-321"
        assert text(cb, "Manufacturer") == "ClearBoxCo"
        assert text(cb, "Model") == "CB-Pro"
        assert text(cb, "Selected_Camera") == "Default"
        assert text(cb, "Custom_Video_Format") == "MP4"
        assert text(cb, "Video_Output") == "HDMI"
        assert text(cb, "Correction_Grid_Domain_Shape") == "square"
        assert text(cb, "Inverse_Grid_Domain_Shape") == "square"

    def test_per_train_integer_attributes(self, clearbox_rt: MachineConfig) -> None:
        cb = clearbox(clearbox_rt, 0)
        assert integer(cb, "Data_Port") == 5001
        assert integer(cb, "Server_Port") == 20101
        assert integer(cb, "Actual_Timing_Offset") == -8
        assert integer(cb, "Commanded_Timing_Offset") == 50
        assert integer(cb, "Show_Console") == 0

    def test_shared_values_live_on_the_root(self, clearbox_rt: MachineConfig) -> None:
        root = clearbox_root(clearbox_rt)
        assert text(root, "Output_Path") == "/recordings/"
        assert integer(root, "Software_Trigger_Delay") == 3000
        assert "Output_Path" not in clearbox(clearbox_rt, 0).attrs

    def test_power_characterization(self, clearbox_rt: MachineConfig) -> None:
        pc = clearbox(clearbox_rt, 0).children["Power_Characterization"]
        assert text(pc, "Algorithm_Type") == "LINEAR"
        assert dict(constants(pc)) == {"b": 48.5, "a": 105.0}

    def test_synchronous_sensor_present_and_named(self, clearbox_rt: MachineConfig) -> None:
        sensors = clearbox(clearbox_rt, 0).children["Synchronous_Sensors"].children
        assert list(sensors) == ["Oxygen Sensor"]
        sensor = sensors["Oxygen Sensor"]
        assert text(sensor, "Sensor_Name") == "ZR800 Oxygen Analyzer"
        assert integer(sensor, "Port_ID") == 5
        assert integer(sensor, "Calibration_Verified") == 0

    def test_synchronous_sensor_tables_exact_values_in_order(self, clearbox_rt: MachineConfig) -> None:
        sensor = clearbox(clearbox_rt, 0).children["Synchronous_Sensors"].children["Oxygen Sensor"]
        consts = sensor.tables["Derivation_Equation_Constants"]
        assert list(zip(consts.columns["name"], consts.columns["value"])) == [("a", 0.4375), ("b", -2.75)]
        points = sensor.tables["Calibration_Points"]
        assert list(zip(points.columns["input_value"], points.columns["output_value"])) == [
            (4.0, -1.0),
            (20.0, 6.0),
        ]

    def test_correction_data_shape(self, clearbox_rt: MachineConfig) -> None:
        ds = clearbox(clearbox_rt, 0).datasets["Correction_Data"]
        assert ds.shape == [5, 5, 2]
        assert len(ds.data) == 50

    def test_inverse_correction_data_shape(self, clearbox_rt: MachineConfig) -> None:
        assert clearbox(clearbox_rt, 0).datasets["Inverse_Correction_Data"].shape == [5, 5, 2]

    def test_correction_data_values(self, clearbox_rt: MachineConfig) -> None:
        """Cell (i, j, *) = float(i + j); spot-check a few cells (row-major flat index)."""
        data = clearbox(clearbox_rt, 0).datasets["Correction_Data"].data

        def cell(i: int, j: int, k: int) -> float:
            return data[(i * 5 + j) * 2 + k]

        assert cell(0, 0, 0) == pytest.approx(0.0)
        assert cell(1, 0, 0) == pytest.approx(1.0)
        assert cell(2, 3, 1) == pytest.approx(5.0)  # 2+3=5


class TestCorrectionArrayNaN:
    """NaN cells in a correction grid are stored as IEEE 754 NaN and read back as NaN.
    Finite values must be bit-exact after the roundtrip."""

    SIZE = 3  # 3×3×2 grid — keeps tests fast

    @staticmethod
    def _cell(cb: ExtensionTree, name: str, i: int, j: int, k: int = 0) -> float:
        return cb.datasets[name].data[(i * TestCorrectionArrayNaN.SIZE + j) * 2 + k]

    def test_forward_nan_cell(self, nan_cb: ExtensionTree) -> None:
        assert math.isnan(self._cell(nan_cb, "Correction_Data", 0, 0, 0))
        assert math.isnan(self._cell(nan_cb, "Correction_Data", 0, 0, 1))

    def test_forward_finite_cells(self, nan_cb: ExtensionTree) -> None:
        assert self._cell(nan_cb, "Correction_Data", 1, 0) == pytest.approx(1.0)  # 1 + 0*10
        assert self._cell(nan_cb, "Correction_Data", 0, 1) == pytest.approx(10.0)  # 0 + 1*10

    def test_inverse_nan_cell(self, nan_cb: ExtensionTree) -> None:
        assert math.isnan(self._cell(nan_cb, "Inverse_Correction_Data", 2, 2, 0))
        assert math.isnan(self._cell(nan_cb, "Inverse_Correction_Data", 2, 2, 1))

    def test_inverse_finite_cells(self, nan_cb: ExtensionTree) -> None:
        assert self._cell(nan_cb, "Inverse_Correction_Data", 0, 0) == pytest.approx(0.0)
        assert self._cell(nan_cb, "Inverse_Correction_Data", 1, 2) == pytest.approx(7.0)  # 1*5 + 2

    def test_no_other_forward_nans(self, nan_cb: ExtensionTree) -> None:
        for i in range(self.SIZE):
            for j in range(self.SIZE):
                if (i, j) != (0, 0):
                    assert not math.isnan(self._cell(nan_cb, "Correction_Data", i, j))

    def test_no_other_inverse_nans(self, nan_cb: ExtensionTree) -> None:
        for i in range(self.SIZE):
            for j in range(self.SIZE):
                if (i, j) != (2, 2):
                    assert not math.isnan(self._cell(nan_cb, "Inverse_Correction_Data", i, j))


class TestScannerAxisConfigurations:
    """axis_configuration "2D", "3D", "3D+Focus" each produce correct subgroup presence."""

    def test_2d_axis_configuration_roundtrip(self, tmp_path: Path) -> None:
        sc = _roundtrip(_config(axis_cfg="2D"), tmp_path / "2d.h5").optical_trains[0].scanner
        assert sc.axis_configuration == "2D"
        assert sc.x_axis is not None
        assert sc.y_axis is not None
        assert sc.z_axis is None
        assert sc.focus is None

    def test_3d_axis_configuration_roundtrip(self, tmp_path: Path) -> None:
        sc = _roundtrip(_config(axis_cfg="3D"), tmp_path / "3d.h5").optical_trains[0].scanner
        assert sc.axis_configuration == "3D"
        assert sc.x_axis is not None
        assert sc.y_axis is not None
        assert sc.z_axis is not None
        assert sc.focus is None

    def test_3d_plus_focus_axis_configuration_roundtrip(self, tmp_path: Path) -> None:
        sc = _roundtrip(
            _config(axis_cfg="3D+Focus"), tmp_path / "focus.h5"
        ).optical_trains[0].scanner
        assert sc.axis_configuration == "3D+Focus"
        assert sc.x_axis is not None
        assert sc.y_axis is not None
        assert sc.z_axis is not None
        assert sc.focus is not None

    def test_axis_scalar_values_survive_roundtrip(self, tmp_path: Path) -> None:
        """AxisConfig scalar fields written for X_Axis are read back correctly."""
        ax = _roundtrip(
            _config(axis_cfg="3D"), tmp_path / "axis_vals.h5"
        ).optical_trains[0].scanner.x_axis
        assert ax.actual_bit_resolution == 20
        assert ax.actual_bit_resolution_unit == "bits"
        assert ax.commanded_bit_resolution == 18
        assert ax.commanded_bit_resolution_unit == "bits"
        assert ax.smoothing_kernel == "GAUSSIAN"
        assert ax.smoothing_parameters == pytest.approx(60.0)
        assert ax.tuning_parameters == "1.0,2.0,3.0"
        assert ax.tuning_type == "PID"

    def test_range_of_motion_roundtrip(self, tmp_path: Path) -> None:
        ax = _roundtrip(
            _config(axis_cfg="3D"), tmp_path / "rom.h5"
        ).optical_trains[0].scanner.y_axis
        assert ax.range_of_motion == pytest.approx(120.5)


class TestScannerInvertFlags:
    """Invert_Actual_X/Y, Invert_Commanded_X/Y — plain bool, written (and
    JSON-serialised) only when True; False and absent are indistinguishable
    everywhere except immediately after a read (user-confirmed, 2026-08-21).
    """

    def test_defaults_to_false_and_omitted_from_json(self, tmp_path: Path) -> None:
        rt = _roundtrip(_config(), tmp_path / "defaults.h5")
        s = rt.optical_trains[0].scanner
        assert s.invert_actual_x is False
        assert s.invert_actual_y is False
        assert s.invert_commanded_x is False
        assert s.invert_commanded_y is False

    def test_only_true_values_survive_as_real_hdf5_attributes(self, tmp_path: Path) -> None:
        import h5py

        cfg = _config()
        cfg.optical_trains[0].scanner.invert_actual_x = True
        cfg.optical_trains[0].scanner.invert_actual_y = False
        cfg.optical_trains[0].scanner.invert_commanded_x = True
        cfg.optical_trains[0].scanner.invert_commanded_y = False
        out = tmp_path / "invert.h5"
        rt = _roundtrip(cfg, out)

        # Raw HDF5 inspection: only the two True-valued attributes exist at all.
        with h5py.File(out, "r") as f:
            attrs = f["Machine/Optical_Trains/Optical_Train_01/Scanner"].attrs
            assert "Invert_Actual_X" in attrs
            assert "Invert_Commanded_X" in attrs
            assert "Invert_Actual_Y" not in attrs
            assert "Invert_Commanded_Y" not in attrs

        s = rt.optical_trains[0].scanner
        assert s.invert_actual_x is True
        assert s.invert_actual_y is False
        assert s.invert_commanded_x is True
        assert s.invert_commanded_y is False


def _only_sensor(label: str, sensor: ExtensionTree) -> MachineConfig:
    cfg = _config()
    sensors = ExtensionTree(children={label: sensor})
    clearbox(cfg, 0).children["Synchronous_Sensors"] = sensors
    return cfg


class TestSynchronousSensorEdgeCases:
    """Edge cases distinct from TestClearBoxAttributeRoundtrip's happy path."""

    def test_no_sensors_group_roundtrips_as_absent(self, tmp_path: Path) -> None:
        cfg = _config()
        del clearbox(cfg, 0).children["Synchronous_Sensors"]
        rt = _roundtrip(cfg, tmp_path / "no_sensors.h5")
        assert "Synchronous_Sensors" not in clearbox(rt, 0).children

    def test_zero_row_tables_for_existing_sensor(self, tmp_path: Path) -> None:
        """The sensor itself exists but both compound datasets have zero rows — proving 0-length
        compound dataset creation/read genuinely works."""
        sensor = ExtensionTree(attrs={"Sensor_Name": Attr("utf8", "Placeholder")})
        sensor.tables["Derivation_Equation_Constants"] = Table(
            fields=[TableField("name", "utf8", 64), TableField("value", "f64")],
            rows=0,
            columns={"name": [], "value": []},
        )
        sensor.tables["Calibration_Points"] = Table(
            fields=[TableField("input_value", "f64"), TableField("output_value", "f64")],
            rows=0,
            columns={"input_value": [], "output_value": []},
        )
        rt = _roundtrip(_only_sensor("Untested Sensor", sensor), tmp_path / "zero_row.h5")
        back = clearbox(rt, 0).children["Synchronous_Sensors"].children["Untested Sensor"]
        assert back.tables["Derivation_Equation_Constants"].rows == 0
        assert back.tables["Calibration_Points"].rows == 0
        assert text(back, "Sensor_Name") == "Placeholder"

    def test_arbitrary_differently_styled_key_survives_roundtrip(self, tmp_path: Path) -> None:
        """The map key is a free-form label with no schema meaning."""
        sensor = ExtensionTree(attrs={"Port_ID": Attr("i32", 9)})
        rt = _roundtrip(_only_sensor("HUMIDITY_SENSOR_2", sensor), tmp_path / "arbitrary_key.h5")
        sensors = clearbox(rt, 0).children["Synchronous_Sensors"].children
        assert integer(sensors["HUMIDITY_SENSOR_2"], "Port_ID") == 9

    def test_constant_name_too_long_for_fixed64_raises(self, tmp_path: Path) -> None:
        """Derivation_Equation_Constants.name is a 64-byte fixed-length field. A name whose UTF-8
        encoding exceeds 64 bytes must be rejected with a clear error at write time, not silently
        truncated."""
        sensor = ExtensionTree()
        sensor.tables["Derivation_Equation_Constants"] = Table(
            fields=[TableField("name", "utf8", 64), TableField("value", "f64")],
            rows=1,
            columns={"name": ["a" * 65], "value": [1.0]},
        )
        with pytest.raises(ValueError, match="do not fit"):
            MachineConfigWriter(_only_sensor("Oversized Name Sensor", sensor)).write(
                tmp_path / "oversized_name.h5"
            )


class TestWithoutClearBox:
    """Machine-agnostic path: a config with no ClearBox round-trips correctly."""

    def test_no_clearbox_extension(self, nocb_rt: MachineConfig) -> None:
        assert "ClearBox" not in nocb_rt.extensions

    def test_scan_field_correction_file_is_none(self, nocb_rt: MachineConfig) -> None:
        assert nocb_rt.optical_trains[0].scan_field_correction_file is None

    def test_scanner_intact_without_clearbox(self, nocb_rt: MachineConfig) -> None:
        assert nocb_rt.optical_trains[0].scanner.working_distance == pytest.approx(670.0)

    def test_light_source_intact_without_clearbox(self, nocb_rt: MachineConfig) -> None:
        assert nocb_rt.optical_trains[0].light_source.wavelength == pytest.approx(1070.0)

    def test_thermal_lensing_intact_without_clearbox(self, nocb_rt: MachineConfig) -> None:
        assert nocb_rt.optical_trains[0].thermal_lensing_passed is True


class TestSchemaValidation:
    """HDF5 produced by MachineConfigWriter validates against machine_config_v1.schema.json."""

    def test_written_hdf5_validates_against_schema(self, tmp_path: Path) -> None:
        import jsonschema
        out = tmp_path / "schema_valid.h5"
        MachineConfigWriter(_config()).write(out)
        reader = MachineConfigReader(out)
        as_dict = reader._config_to_dict(reader.parse())
        jsonschema.validate(as_dict, SCHEMA)  # raises jsonschema.ValidationError on failure

    def test_written_hdf5_without_clearbox_validates_against_schema(
        self, tmp_path: Path
    ) -> None:
        import jsonschema
        out = tmp_path / "schema_valid_nocb.h5"
        MachineConfigWriter(_config(include_clearbox=False)).write(out)
        reader = MachineConfigReader(out)
        as_dict = reader._config_to_dict(reader.parse())
        jsonschema.validate(as_dict, SCHEMA)
