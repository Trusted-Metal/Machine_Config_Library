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

from dataclasses import replace as dc_replace
from pathlib import Path

import pytest

from machine_config import MachineConfigReader, MachineConfigWriter
from machine_config.models import (
    AxisConfig,
    BuildPlate,
    CalibrationPoint,
    ClearBox,
    Collimator,
    EquationConstant,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    OptionalComponents,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
    SynchronousSensor,
)
from machine_config.schema import SCHEMA, SCHEMA_VERSION


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
        watts_to_volts_algorithm="LINEAR",
        watts_to_volts_params="[1,100,10,1000]",
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


def _clearbox(grid_size: int = 5) -> ClearBox:
    """ClearBox with all 18 scalar attributes populated; small correction grid."""
    grid = [
        [[float(i + j) for _ in range(2)] for j in range(grid_size)]
        for i in range(grid_size)
    ]
    inv = [
        [[float(i + j + 1) for _ in range(2)] for j in range(grid_size)]
        for i in range(grid_size)
    ]
    return ClearBox(
        ip_address="192.168.1.100",
        serial_number="CB-321",
        data_port=5001,
        server_port=20101,
        actual_timing_offset=-8,
        commanded_timing_offset=50,
        correction_data=grid,
        inverse_correction_data=inv,
        manufacturer="ClearBoxCo",
        model="CB-Pro",
        output_path="/recordings/",
        selected_camera="Default",
        custom_video_format="MP4",
        video_output="HDMI",
        show_console=False,
        software_trigger_delay=3000,
        volts_to_watts_algorithm="LINEAR",
        volts_to_watts_params="48.5,105.0",
        correction_grid_domain_shape="square",
        inverse_grid_domain_shape="square",
        synchronous_sensors={
            "Oxygen Sensor": SynchronousSensor(
                enabled=True,
                sensor_name="ZR800 Oxygen Analyzer",
                sensor_output_range_low=-1.0,
                sensor_output_range_high=6.0,
                sensor_output_space="log10(ppm)",
                sensor_model="ZR810",
                sensor_manufacturer="Industrial Physics",
                sensor_scope="Global",
                units_derived_quantity="ppm",
                port_id=5,
                sensor_type="Oxygen Sensor",
                input_type="4-20 mA",
                algorithm_type="Log-Linear",
                algorithm_equation="log(ppm) = a*mA + b",
                calibration_source="Datasheet",
                calibration_verified=False,
                sample_period=5.0,
                metadata="Synchronous Sensor because Clearbox is responsible for recording.",
                derivation_equation_constants=[
                    EquationConstant(name="a", value=0.4375),
                    EquationConstant(name="b", value=-2.75),
                ],
                calibration_points=[
                    CalibrationPoint(input_value=4.0, output_value=-1.0),
                    CalibrationPoint(input_value=20.0, output_value=6.0),
                ],
            ),
        },
    )


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
        optional_components=OptionalComponents(clearbox=_clearbox() if include_clearbox else None),
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
        ),
        optical_trains=[_train(axis_cfg, include_clearbox)],
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
def clearbox_cb(tmp_path_factory: pytest.TempPathFactory) -> ClearBox:
    """ClearBox from a written config; used by TestClearBoxAttributeRoundtrip."""
    out = tmp_path_factory.mktemp("cb_attrs") / "cb.h5"
    MachineConfigWriter(_config()).write(out)
    return MachineConfigReader(out).parse().optical_trains[0].optional_components.clearbox  # type: ignore[return-value]


@pytest.fixture(scope="module")
def nan_cb(tmp_path_factory: pytest.TempPathFactory) -> ClearBox:
    """ClearBox with None cells in both grids; used by TestCorrectionArrayNullNaN."""
    size = 3
    fwd = [
        [
            [None if (i == 0 and j == 0) else float(i + j * 10) for _ in range(2)]
            for j in range(size)
        ]
        for i in range(size)
    ]
    inv = [
        [
            [None if (i == 2 and j == 2) else float(i * 5 + j) for _ in range(2)]
            for j in range(size)
        ]
        for i in range(size)
    ]
    cb_obj = ClearBox(
        ip_address="10.0.0.1",
        serial_number=None,
        data_port=None,
        server_port=None,
        actual_timing_offset=None,
        commanded_timing_offset=None,
        correction_data=fwd,
        inverse_correction_data=inv,
        manufacturer=None,
        model=None,
        output_path=None,
        selected_camera=None,
        custom_video_format=None,
        video_output=None,
        show_console=None,
        software_trigger_delay=None,
        volts_to_watts_algorithm=None,
        volts_to_watts_params=None,
        correction_grid_domain_shape=None,
        inverse_grid_domain_shape=None,
        synchronous_sensors={},
    )
    base = _config(include_clearbox=False)
    patched = dc_replace(base.optical_trains[0], optional_components=OptionalComponents(clearbox=cb_obj))
    cfg = dc_replace(base, optical_trains=[patched])
    out = tmp_path_factory.mktemp("nan_test") / "nan.h5"
    MachineConfigWriter(cfg).write(out)
    return MachineConfigReader(out).parse().optical_trains[0].optional_components.clearbox  # type: ignore[return-value]


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
        assert scalar_rt.meta.configuration_hash == "a" * 64

    # --- Machine / BuildPlate -----------------------------------------------

    def test_machine_name(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.machine_name == "TestMachine-01"

    def test_machine_manufacturer(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.manufacturer == "TestCo"

    def test_machine_gas_flow_direction(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.gas_flow_direction == "Y+"

    def test_machine_recoat_direction(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.machine.recoat_direction == "X+"

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

    def test_light_source_watts_to_volts_algorithm(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.watts_to_volts_algorithm == "LINEAR"

    def test_light_source_watts_to_volts_params(self, scalar_rt: MachineConfig) -> None:
        assert scalar_rt.optical_trains[0].light_source.watts_to_volts_params == "[1,100,10,1000]"

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
    """All 18 ClearBox scalar attributes survive write → read with identical values."""

    def test_ip_address(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.ip_address == "192.168.1.100"

    def test_serial_number(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.serial_number == "CB-321"

    def test_data_port(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.data_port == 5001

    def test_server_port(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.server_port == 20101

    def test_actual_timing_offset(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.actual_timing_offset == -8

    def test_commanded_timing_offset(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.commanded_timing_offset == 50

    def test_manufacturer(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.manufacturer == "ClearBoxCo"

    def test_model(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.model == "CB-Pro"

    def test_output_path(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.output_path == "/recordings/"

    def test_selected_camera(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.selected_camera == "Default"

    def test_custom_video_format(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.custom_video_format == "MP4"

    def test_video_output(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.video_output == "HDMI"

    def test_show_console_false(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.show_console is False

    def test_software_trigger_delay(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.software_trigger_delay == 3000

    def test_volts_to_watts_algorithm(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.volts_to_watts_algorithm == "LINEAR"

    def test_volts_to_watts_params(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.volts_to_watts_params == "48.5,105.0"

    def test_correction_grid_domain_shape(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.correction_grid_domain_shape == "square"

    def test_inverse_grid_domain_shape(self, clearbox_cb: ClearBox) -> None:
        assert clearbox_cb.inverse_grid_domain_shape == "square"

    def test_synchronous_sensor_present_and_named(self, clearbox_cb: ClearBox) -> None:
        assert len(clearbox_cb.synchronous_sensors) == 1
        sensor = clearbox_cb.synchronous_sensors["Oxygen Sensor"]
        assert sensor.sensor_name == "ZR800 Oxygen Analyzer"
        assert sensor.port_id == 5
        assert sensor.calibration_verified is False

    def test_synchronous_sensor_compound_datasets_exact_values_in_order(
        self, clearbox_cb: ClearBox
    ) -> None:
        sensor = clearbox_cb.synchronous_sensors["Oxygen Sensor"]
        assert [(c.name, c.value) for c in sensor.derivation_equation_constants] == [
            ("a", 0.4375),
            ("b", -2.75),
        ]
        assert [(p.input_value, p.output_value) for p in sensor.calibration_points] == [
            (4.0, -1.0),
            (20.0, 6.0),
        ]

    def test_correction_data_shape(self, clearbox_cb: ClearBox) -> None:
        assert isinstance(clearbox_cb.correction_data, list)
        assert len(clearbox_cb.correction_data) == 5
        assert len(clearbox_cb.correction_data[0]) == 5
        assert len(clearbox_cb.correction_data[0][0]) == 2

    def test_inverse_correction_data_shape(self, clearbox_cb: ClearBox) -> None:
        assert isinstance(clearbox_cb.inverse_correction_data, list)
        assert len(clearbox_cb.inverse_correction_data) == 5

    def test_correction_data_values(self, clearbox_cb: ClearBox) -> None:
        """Cell (i, j, *) = float(i + j); spot-check a few cells."""
        assert clearbox_cb.correction_data[0][0][0] == pytest.approx(0.0)
        assert clearbox_cb.correction_data[1][0][0] == pytest.approx(1.0)
        assert clearbox_cb.correction_data[2][3][1] == pytest.approx(5.0)  # 2+3=5


class TestCorrectionArrayNullNaN:
    """None cells in the model write as IEEE 754 NaN in HDF5 and read back as None.
    Non-None float values must be bit-exact after the roundtrip.
    """

    SIZE = 3  # 3×3×2 grid — keeps tests fast

    def test_forward_null_cell(self, nan_cb: ClearBox) -> None:
        """Cell (0,0,*) was None → must read back as None."""
        assert nan_cb.correction_data[0][0][0] is None  # type: ignore[index]
        assert nan_cb.correction_data[0][0][1] is None  # type: ignore[index]

    def test_forward_finite_cell_i1_j0(self, nan_cb: ClearBox) -> None:
        """i=1, j=0 → float(1 + 0×10) = 1.0."""
        assert nan_cb.correction_data[1][0][0] == pytest.approx(1.0)  # type: ignore[index]

    def test_forward_finite_cell_i0_j1(self, nan_cb: ClearBox) -> None:
        """i=0, j=1 → float(0 + 1×10) = 10.0."""
        assert nan_cb.correction_data[0][1][0] == pytest.approx(10.0)  # type: ignore[index]

    def test_inverse_null_cell(self, nan_cb: ClearBox) -> None:
        """Cell (2,2,*) in the inverse grid was None → must read back as None."""
        assert nan_cb.inverse_correction_data[2][2][0] is None  # type: ignore[index]
        assert nan_cb.inverse_correction_data[2][2][1] is None  # type: ignore[index]

    def test_inverse_finite_cell_i0_j0(self, nan_cb: ClearBox) -> None:
        """i=0, j=0 → float(0×5 + 0) = 0.0."""
        assert nan_cb.inverse_correction_data[0][0][0] == pytest.approx(0.0)  # type: ignore[index]

    def test_inverse_finite_cell_i1_j2(self, nan_cb: ClearBox) -> None:
        """i=1, j=2 → float(1×5 + 2) = 7.0."""
        assert nan_cb.inverse_correction_data[1][2][0] == pytest.approx(7.0)  # type: ignore[index]

    def test_no_other_forward_nulls(self, nan_cb: ClearBox) -> None:
        """All non-(0,0) cells in the forward grid must not be None."""
        for i in range(TestCorrectionArrayNullNaN.SIZE):
            for j in range(TestCorrectionArrayNullNaN.SIZE):
                if i == 0 and j == 0:
                    continue
                assert nan_cb.correction_data[i][j][0] is not None  # type: ignore[index]

    def test_no_other_inverse_nulls(self, nan_cb: ClearBox) -> None:
        """All non-(2,2) cells in the inverse grid must not be None."""
        for i in range(TestCorrectionArrayNullNaN.SIZE):
            for j in range(TestCorrectionArrayNullNaN.SIZE):
                if i == 2 and j == 2:
                    continue
                assert nan_cb.inverse_correction_data[i][j][0] is not None  # type: ignore[index]


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


class TestSynchronousSensorEdgeCases:
    """Edge cases distinct from TestClearBoxAttributeRoundtrip's happy path."""

    def test_empty_sensors_map_roundtrips_as_empty_not_absent(self, tmp_path: Path) -> None:
        """The write path must skip creating the Synchronous_Sensors group
        entirely when the map is empty (not write an empty placeholder), and
        the read path must default back to an empty dict, not error.
        """
        cfg = _config()
        cfg.optical_trains[0].optional_components.clearbox.synchronous_sensors = {}
        rt = _roundtrip(cfg, tmp_path / "empty_sensors.h5")
        assert rt.optical_trains[0].optional_components.clearbox.synchronous_sensors == {}

    def test_zero_row_compound_datasets_for_existing_sensor(self, tmp_path: Path) -> None:
        """Distinct edge case from the empty-*map* test above: here the
        sensor itself exists (its group is created), but both compound
        datasets have zero rows — proving 0-length compound dataset
        creation/read genuinely works, not assumed.
        """
        cfg = _config()
        cfg.optical_trains[0].optional_components.clearbox.synchronous_sensors = {
            "Untested Sensor": SynchronousSensor(
                enabled=False,
                sensor_name="Placeholder",
                sensor_output_range_low=None,
                sensor_output_range_high=None,
                sensor_output_space=None,
                sensor_model=None,
                sensor_manufacturer=None,
                sensor_scope=None,
                units_derived_quantity=None,
                port_id=None,
                sensor_type=None,
                input_type=None,
                algorithm_type=None,
                algorithm_equation=None,
                calibration_source=None,
                calibration_verified=None,
                sample_period=None,
                metadata=None,
                derivation_equation_constants=[],
                calibration_points=[],
            ),
        }
        rt = _roundtrip(cfg, tmp_path / "zero_row.h5")
        sensor = rt.optical_trains[0].optional_components.clearbox.synchronous_sensors["Untested Sensor"]
        assert sensor.derivation_equation_constants == []
        assert sensor.calibration_points == []
        assert sensor.sensor_name == "Placeholder"

    def test_arbitrary_differently_styled_key_survives_roundtrip(self, tmp_path: Path) -> None:
        """The map key is a free-form label with no schema meaning — proves a
        key unlike the fixture's own "Oxygen Sensor" (different style:
        underscore-joined, all-caps) survives a write->read cycle verbatim.
        """
        cfg = _config()
        cfg.optical_trains[0].optional_components.clearbox.synchronous_sensors = {
            "HUMIDITY_SENSOR_2": SynchronousSensor(
                enabled=True,
                sensor_name=None,
                sensor_output_range_low=None,
                sensor_output_range_high=None,
                sensor_output_space=None,
                sensor_model=None,
                sensor_manufacturer=None,
                sensor_scope=None,
                units_derived_quantity=None,
                port_id=9,
                sensor_type=None,
                input_type=None,
                algorithm_type=None,
                algorithm_equation=None,
                calibration_source=None,
                calibration_verified=None,
                sample_period=None,
                metadata=None,
                derivation_equation_constants=[],
                calibration_points=[],
            ),
        }
        rt = _roundtrip(cfg, tmp_path / "arbitrary_key.h5")
        sensors = rt.optical_trains[0].optional_components.clearbox.synchronous_sensors
        assert "HUMIDITY_SENSOR_2" in sensors
        assert sensors["HUMIDITY_SENSOR_2"].port_id == 9

    def test_constant_name_too_long_for_fixed64_raises(self, tmp_path: Path) -> None:
        """Derivation_Equation_Constants.name is a 64-byte fixed-length
        field (see SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string
        convention"). A name whose UTF-8 encoding exceeds 64 bytes must be
        rejected with a clear error at write time, not silently truncated
        by numpy's fixed-width string dtype.
        """
        cfg = _config()
        cfg.optical_trains[0].optional_components.clearbox.synchronous_sensors = {
            "Oversized Name Sensor": SynchronousSensor(
                enabled=None,
                sensor_name=None,
                sensor_output_range_low=None,
                sensor_output_range_high=None,
                sensor_output_space=None,
                sensor_model=None,
                sensor_manufacturer=None,
                sensor_scope=None,
                units_derived_quantity=None,
                port_id=None,
                sensor_type=None,
                input_type=None,
                algorithm_type=None,
                algorithm_equation=None,
                calibration_source=None,
                calibration_verified=None,
                sample_period=None,
                metadata=None,
                derivation_equation_constants=[EquationConstant(name="a" * 65, value=1.0)],
                calibration_points=[],
            ),
        }
        with pytest.raises(ValueError, match="does not fit"):
            MachineConfigWriter(cfg).write(tmp_path / "oversized_name.h5")


class TestWithoutClearBox:
    """Machine-agnostic path: a config with no ClearBox round-trips correctly."""

    def test_clearbox_is_none(self, nocb_rt: MachineConfig) -> None:
        assert nocb_rt.optical_trains[0].optional_components.clearbox is None

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
