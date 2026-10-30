"""Phase 1.3 / 1.6 — MachineConfigReader tests against real AconityMIDI fixtures."""
import json

import numpy as np
import pytest

from machine_config.builder import MockConfigBuilder
from machine_config.reader import MachineConfigReader
from machine_config.schema import SCHEMA

from clearbox_helpers import (
    attr,
    clearbox,
    clearbox_root,
    grid,
    integer,
    text,
)


# ===========================================================================
# MachineConfigMeta
# ===========================================================================

class TestMeta:
    def test_machine_name(self, reference_config):
        assert reference_config.meta.machine_name == "TM-LPBF-02: AconityMIDI+_OG"

    def test_manufacturer(self, reference_config):
        assert reference_config.meta.manufacturer == "Aconity3D"

    def test_configuration_hash_64_chars(self, reference_config):
        assert len(reference_config.meta.configuration_hash) == 64

    def test_file_version(self, reference_config):
        assert reference_config.meta.file_version == "1.0"


# ===========================================================================
# Machine / BuildPlate
# ===========================================================================

class TestMachine:
    def test_build_plate_x(self, reference_config):
        assert reference_config.machine.build_plate.x == 250.0

    def test_build_plate_y(self, reference_config):
        assert reference_config.machine.build_plate.y == 250.0

    def test_build_plate_z(self, reference_config):
        assert reference_config.machine.build_plate.z == 20.0

    def test_build_plate_units_locked_mm(self, reference_config):
        bp = reference_config.machine.build_plate
        assert bp.x_unit == "mm"
        assert bp.y_unit == "mm"
        assert bp.z_unit == "mm"


# ===========================================================================
# Optical trains — structural
# ===========================================================================

class TestOpticalTrains:
    def test_count(self, reference_config):
        assert len(reference_config.optical_trains) == 2

    def test_train_ids(self, reference_config):
        ids = [t.train_id for t in reference_config.optical_trains]
        assert ids == ["Optical_Train_01", "Optical_Train_02"]


# ===========================================================================
# Scanner (train 0)
# ===========================================================================

class TestScanner:
    def test_working_distance_train0(self, reference_config):
        assert reference_config.optical_trains[0].scanner.working_distance == 670.0

    def test_scan_head_offset_x_train0(self, reference_config):
        assert reference_config.optical_trains[0].scanner.scan_head_offset_x == -87.5

    def test_scan_head_offset_y_train0(self, reference_config):
        assert reference_config.optical_trains[0].scanner.scan_head_offset_y == pytest.approx(23.5, abs=1e-3)

    def test_scan_head_offset_x_train1(self, reference_config):
        assert reference_config.optical_trains[1].scanner.scan_head_offset_x == pytest.approx(86.074, abs=1e-3)

    def test_scan_head_offset_y_train1(self, reference_config):
        assert reference_config.optical_trains[1].scanner.scan_head_offset_y == pytest.approx(-21.695, abs=1e-3)

    def test_scan_head_rotation_train0(self, reference_config):
        assert reference_config.optical_trains[0].scanner.scan_head_rotation == 0.0

    def test_scan_head_rotation_train1(self, reference_config):
        assert reference_config.optical_trains[1].scanner.scan_head_rotation == 180.0

    def test_working_distance_unit_locked(self, reference_config):
        assert reference_config.optical_trains[0].scanner.working_distance_unit == "mm"

    def test_scan_head_rotation_unit_locked(self, reference_config):
        assert reference_config.optical_trains[0].scanner.scan_head_rotation_unit == "degrees"

    def test_empty_str_attrs_are_none(self, reference_config):
        """Any empty-string HDF5 attribute must be parsed as None, not ''."""
        for train in reference_config.optical_trains:
            val = train.scanner.axis_configuration
            assert val is None or (isinstance(val, str) and val != ""), (
                "empty string must become None"
            )

    def test_axis_configuration_is_3D(self, reference_config):
        assert reference_config.optical_trains[0].scanner.axis_configuration == "3D"

    def test_invert_flags_absent_from_fixture_read_as_false(self, reference_config):
        # reference_config.h5 has none of the four Invert_* attributes at all.
        s = reference_config.optical_trains[0].scanner
        assert s.invert_actual_x is False
        assert s.invert_actual_y is False
        assert s.invert_commanded_x is False
        assert s.invert_commanded_y is False

    def test_invert_flags_omitted_from_json_when_false(self, reference_reader):
        import json
        d = json.loads(reference_reader.to_json())
        scanner_d = d["optical_trains"][0]["scanner"]
        assert "invert_actual_x" not in scanner_d
        assert "invert_actual_y" not in scanner_d
        assert "invert_commanded_x" not in scanner_d
        assert "invert_commanded_y" not in scanner_d

    def test_x_axis_smoothing_kernel(self, reference_config):
        assert reference_config.optical_trains[0].scanner.x_axis.smoothing_kernel == "GAUSSIAN"

    def test_y_axis_bit_resolution(self, reference_config):
        assert reference_config.optical_trains[0].scanner.y_axis.actual_bit_resolution == 20

    def test_z_axis_present_for_3D(self, reference_config):
        for train in reference_config.optical_trains:
            if train.scanner.axis_configuration == "3D":
                assert train.scanner.z_axis is not None
                assert train.scanner.focus is None

    def test_focus_absent_for_3D(self, reference_config):
        assert reference_config.optical_trains[0].scanner.focus is None


# ===========================================================================
# LightSource
# ===========================================================================

class TestLightSource:
    def test_wavelength_unit_locked_nm(self, reference_config):
        assert reference_config.optical_trains[0].light_source.wavelength_unit == "nm"

    def test_power_max_nominal_unit_locked_w(self, reference_config):
        assert reference_config.optical_trains[0].light_source.power_max_nominal_unit == "W"

    def test_power_min_nominal_unit_locked_w(self, reference_config):
        assert reference_config.optical_trains[0].light_source.power_min_nominal_unit == "W"


# ===========================================================================
# Collimator
# ===========================================================================

class TestCollimator:
    def test_focal_length_both_trains(self, reference_config):
        for train in reference_config.optical_trains:
            assert train.collimator is not None
            assert train.collimator.focal_length == 120.0

    def test_focal_length_unit_locked_mm(self, reference_config):
        for train in reference_config.optical_trains:
            assert train.collimator.focal_length_unit == "mm"

    def test_train_level_collimator_focal_length(self, reference_config):
        """The duplicate train-level attribute must also be parsed."""
        for train in reference_config.optical_trains:
            assert train.collimator_focal_length == 120.0
            assert train.collimator_focal_length_unit == "mm"


# ===========================================================================
# ScannerCard
# ===========================================================================

class TestScannerCard:
    def test_model_both_trains(self, reference_config):
        for train in reference_config.optical_trains:
            assert train.scanner_card is not None
            assert train.scanner_card.model == "SP-ICE-3"

    def test_sample_period_unit_locked_us(self, reference_config):
        for train in reference_config.optical_trains:
            assert train.scanner_card.sample_period_unit == "μs"


# ===========================================================================
# ClearBox — core exposes it as the opaque "ClearBox" extension tree
# ===========================================================================

class TestClearBox:
    def test_clearbox_present(self, reference_config):
        assert list(clearbox_root(reference_config).children) == ["Optical_Train_01", "Optical_Train_02"]

    def test_clearbox_ip_address(self, reference_config):
        assert text(clearbox(reference_config, 0), "Ip_Address")

    def test_clearbox_data_port_is_int(self, reference_config):
        assert integer(clearbox(reference_config, 0), "Data_Port") is not None

    def test_shared_values_live_on_the_extension_root_once(self, reference_config):
        root = clearbox_root(reference_config)
        assert text(root, "Output_Path")
        assert integer(root, "Software_Trigger_Delay") is not None
        assert "Output_Path" not in clearbox(reference_config, 0).attrs

    def test_v1_0_only_attributes_ride_along_on_each_train(self, reference_config):
        assert "Show_Console" in clearbox(reference_config, 0).attrs
        assert "Volts_To_Watts_Algorithm" not in clearbox(reference_config, 0).attrs

    def test_synchronous_sensors_absent_when_fixture_has_none(self, reference_config):
        assert "Synchronous_Sensors" not in clearbox(reference_config, 0).children


class TestClearBoxCorrectionGrids:
    """The grids are ordinary extension datasets; they feed the ``correction-hash`` CLI."""

    def test_forward_grid_shape(self, reference_config):
        ds = clearbox(reference_config, 0).datasets["Correction_Data"]
        assert ds.dtype == "f64"
        assert ds.shape == [257, 257, 2]
        assert len(ds.data) == 257 * 257 * 2

    def test_inverse_grid_shape(self, reference_config):
        ds = clearbox(reference_config, 0).datasets["Inverse_Correction_Data"]
        assert ds.shape == [257, 257, 2]

    def test_nan_is_preserved(self, reference_config):
        assert np.isnan(grid(reference_config, 0, "Correction_Data")).any()

    def test_forward_and_inverse_differ(self, reference_config):
        fwd = grid(reference_config, 0, "Correction_Data")
        inv = grid(reference_config, 0, "Inverse_Correction_Data")
        assert not np.array_equal(fwd, inv, equal_nan=True)

    def test_train_1_differs_from_train_0(self, reference_config):
        a = grid(reference_config, 0, "Correction_Data")
        b = grid(reference_config, 1, "Correction_Data")
        assert not np.array_equal(a, b, equal_nan=True)


# ===========================================================================
# Synchronous sensors — reference_config_synchronous_sensors.h5, not
# reference_config.h5, which deliberately has no sensors.
# ===========================================================================

def _sensors_of(cfg):
    return clearbox(cfg, 0).children["Synchronous_Sensors"].children


@pytest.fixture(scope="module")
def sensor(sensors_reader):
    """Module-level, not a class-instance-method fixture (avoids PytestRemovedIn10Warning)."""
    sensors = _sensors_of(sensors_reader.parse())
    assert list(sensors) == ["Oxygen Sensor"]
    return sensors["Oxygen Sensor"]


class TestSynchronousSensor:
    def test_scalar_attributes_match_the_real_zr800_example(self, sensor):
        assert text(sensor, "Sensor_Name") == "ZR800 Oxygen Analyzer"
        assert attr(sensor, "Sensor_Output_Range_Low") == -1.0
        assert attr(sensor, "Sensor_Output_Range_High") == 6.0
        assert text(sensor, "Sensor_Output_Space") == "log10(ppm)"
        assert text(sensor, "Sensor_Model") == "ZR810"
        assert text(sensor, "Sensor_Manufacturer") == "Industrial Physics"
        assert text(sensor, "Sensor_Scope") == "Global"
        assert text(sensor, "Units_Derived_Quantity") == "ppm"
        assert integer(sensor, "Port_ID") == 5
        assert text(sensor, "Sensor_Type") == "Oxygen Sensor"
        assert text(sensor, "Input_Type") == "4-20 mA"
        assert text(sensor, "Algorithm_Type") == "Log-Linear"
        assert text(sensor, "Algorithm_Equation") == "log(ppm) = a*mA + b"
        assert text(sensor, "Calibration_Source") == "Datasheet"
        assert attr(sensor, "Sample_Period") == 5.0
        assert "100KHz" in text(sensor, "Metadata")

    def test_derivation_equation_constants_in_order(self, sensor):
        t = sensor.tables["Derivation_Equation_Constants"]
        assert t.columns["name"] == ["a", "b"]
        assert t.columns["value"] == [0.4375, -2.75]

    def test_calibration_points_in_order_and_in_log_space(self, sensor):
        points = sensor.tables["Calibration_Points"]
        assert points.columns["input_value"] == [4.0, 20.0]
        assert points.columns["output_value"] == [-1.0, 6.0]
        a, b = sensor.tables["Derivation_Equation_Constants"].columns["value"]
        for x, y in zip(points.columns["input_value"], points.columns["output_value"]):
            assert abs(a * x + b - y) < 1e-9

    def test_combined_opcua_and_sensors_fixture_has_both(self, opcua_sensors_reader):
        config = opcua_sensors_reader.parse()
        assert "TM_OPCUA" in config.extensions
        t = _sensors_of(config)["Oxygen Sensor"].tables["Derivation_Equation_Constants"]
        assert t.columns["name"] == ["a", "b"]


# ===========================================================================
# ScanFieldCorrectionFile
# ===========================================================================

class TestScanFieldCorrectionFile:
    def test_file_size_train0(self, reference_config):
        assert reference_config.optical_trains[0].scan_field_correction_file.file_size == 1138799

    def test_file_size_train1(self, reference_config):
        assert reference_config.optical_trains[1].scan_field_correction_file.file_size == 1142763

    def test_scan_field_correction_bytes_length_matches_metadata(self, reference_config, reference_reader):
        raw = reference_reader.get_scan_field_correction_bytes(0)
        expected = reference_config.optical_trains[0].scan_field_correction_file.file_size
        assert len(raw) == expected


# ===========================================================================
# Thermal lensing
# ===========================================================================

class TestThermalLensing:
    def test_train0_failed(self, reference_config):
        assert reference_config.optical_trains[0].thermal_lensing_passed is False

    def test_train1_passed(self, reference_config):
        assert reference_config.optical_trains[1].thermal_lensing_passed is True

    def test_threshold_unit_locked_mm(self, reference_config):
        for train in reference_config.optical_trains:
            if train.thermal_lensing_threshold is not None:
                assert train.thermal_lensing_threshold_unit == "mm"


# ===========================================================================
# Rule 8 — locked units
# ===========================================================================

class TestLockedUnits:
    def test_all_expected_units_train0(self, reference_config):
        train = reference_config.optical_trains[0]
        assert train.scanner.working_distance_unit == "mm"
        assert train.light_source.wavelength_unit == "nm"
        assert train.light_source.power_max_nominal_unit == "W"
        assert train.collimator.focal_length_unit == "mm"
        assert train.scanner_card.sample_period_unit == "μs"
        assert train.scanner.scan_head_rotation_unit == "degrees"


# ===========================================================================
# get_raw_group
# ===========================================================================

class TestGetRawGroup:
    def test_missing_path_returns_empty_dict(self, reference_reader):
        result = reference_reader.get_raw_group("NonExistent/Path/That/Does/Not/Exist")
        assert result == {}

    def test_opcua_absent_in_reference_fixture(self, reference_reader):
        """Standard fixture has no OPCUA group — must return {} without raising."""
        result = reference_reader.get_raw_group("OPCUA")
        assert isinstance(result, dict)

    def test_opcua_present_in_opcua_fixture(self, opcua_reader):
        """OPCUA/Client sub-group has attributes in the OPCUA variant."""
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert isinstance(result, dict)
        assert len(result) > 0
        assert "Server_URL" in result

    def test_machine_group_has_attrs(self, reference_reader):
        result = reference_reader.get_raw_group("Machine")
        assert "Machine_Name" in result


# ===========================================================================
# OPCUA — raw group values (reference_config_opcua.h5)
# These protect the golden file: if get_raw_group() misreads any OPCUA field
# the test fails before fixtures/reference_output.json is ever generated.
# ===========================================================================

class TestOpcua:
    def test_opcua_top_level_groups_present(self, opcua_reader):
        client  = opcua_reader.get_raw_group("OPCUA/Client")
        pipe    = opcua_reader.get_raw_group("OPCUA/Pipe")
        triggers = opcua_reader.get_raw_group("OPCUA/Triggers")
        assert client and pipe and triggers, "OPCUA/Client, /Pipe, /Triggers must all exist"

    def test_opcua_client_server_url(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert result["Server_URL"] == "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"

    def test_opcua_client_auth_mode(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert result["Auth_Mode"] == "UsernamePassword"

    def test_opcua_client_security_mode(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert result["Security_Mode"] == "SignAndEncrypt"

    def test_opcua_client_security_policy(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert result["Security_Policy"] == "ECC_brainpoolP384r1"

    def test_opcua_client_integer_fields(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Client")
        assert int(result["BFS_Max_Depth"]) == 16
        assert int(result["Publish_Interval"]) == 250
        assert int(result["Sampling_Interval"]) == 250
        assert int(result["Session_Timeout"]) == 60000

    def test_opcua_pipe_enabled(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Pipe")
        assert int(result["Pipe_Enabled"]) == 1

    def test_opcua_pipe_buffer_size(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Pipe")
        assert int(result["Buffer_Size"]) == 65536

    def test_opcua_triggers_enabled(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Triggers")
        assert float(result["Triggers_Enabled"]) == 1.0

    def test_opcua_trigger_laser_emission_interlock(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Triggers/Laser Emission Interlock")
        assert result["ID"] == "trigger_1"
        assert result["Signal"] == "yellow_light"
        assert result["Subsystem"] == "Chamber"
        assert int(result["Rule_Enabled"]) == 1

    def test_opcua_trigger_chamber_oxygen(self, opcua_reader):
        result = opcua_reader.get_raw_group("OPCUA/Triggers/Chamber Oxygen Level")
        assert result["ID"] == "trigger_2"
        assert result["Signal"] == "oxygen_level"
        assert result["Start_Value"] == "700"
        assert result["Stop_Value"] == "1000"

    def test_opcua_absent_in_reference_config(self, reference_reader):
        """Standard (non-OPCUA) fixture must have no OPCUA group at all."""
        result = reference_reader.get_raw_group("OPCUA")
        assert result == {}


# ===========================================================================
# Extensions - parse() exposes the legacy OPCUA group as an opaque "TM_OPCUA" tree
# ===========================================================================

class TestExtensionsModel:
    def test_only_clearbox_on_reference_config(self, reference_config):
        assert list(reference_config.extensions) == ["ClearBox"]

    def test_opcua_group_is_an_opaque_extension(self, opcua_reader):
        config = opcua_reader.parse()
        assert sorted(config.extensions) == ["ClearBox", "TM_OPCUA"]
        tree = config.extensions["TM_OPCUA"]
        assert set(tree.children) >= {"Client", "Pipe", "Triggers"}

    def test_opaque_attrs_keep_their_stored_dtype(self, opcua_reader):
        client = opcua_reader.parse().extensions["TM_OPCUA"].children["Client"]
        assert client.attrs["Server_URL"].dtype == "utf8"
        assert client.attrs["Server_URL"].value == (
            "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"
        )
        assert client.attrs["Publish_Interval"].value == 250

    def test_trigger_groups_are_children(self, opcua_reader):
        triggers = opcua_reader.parse().extensions["TM_OPCUA"].children["Triggers"]
        assert "Laser Emission Interlock" in triggers.children
        assert "Chamber Oxygen Level" in triggers.children


# ===========================================================================
# JSON / schema validation
# ===========================================================================

class TestJsonOutput:
    def test_to_json_is_valid_json(self, reference_reader):
        raw = reference_reader.to_json()
        parsed = json.loads(raw)  # must not raise
        assert isinstance(parsed, dict)

    def test_schema_validation(self, reference_reader):
        import jsonschema
        output = json.loads(reference_reader.to_json())
        jsonschema.validate(output, SCHEMA)  # must not raise

    def test_to_json_machine_name_present(self, reference_reader):
        output = json.loads(reference_reader.to_json())
        assert output["meta"]["machine_name"] == "TM-LPBF-02: AconityMIDI+_OG"


# ===========================================================================
# Tests that require Phase 1.5 MockConfigBuilder
# ===========================================================================

def test_roundtrip(tmp_path, reference_config):
    builder = MockConfigBuilder.from_config(reference_config)
    out = tmp_path / "roundtrip.h5"
    builder.save(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.meta.machine_name == reference_config.meta.machine_name


def test_unit_mismatch_raises_value_error(tmp_path):
    builder = MockConfigBuilder(n_lasers=1, working_distance_unit="inches")
    out = tmp_path / "bad_units.h5"
    builder.save(out)
    with pytest.raises(ValueError, match="inches"):
        MachineConfigReader(out).parse()


def test_absent_clearbox_has_no_extension(tmp_path):
    builder = MockConfigBuilder(n_lasers=1, include_clearbox=False)
    out = tmp_path / "no_clearbox.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert "ClearBox" not in config.extensions


def test_unknown_file_version_does_not_use_v1_layout(tmp_path):
    """Peek File_Version before walking groups — a v2 file with no Machine
    group must not be parsed by the v1.0 adapter."""
    import h5py

    from machine_config.capabilities import open_machine_config
    from machine_config.capabilities.file_version import UnsupportedFileVersion

    out = tmp_path / "future.h5"
    with h5py.File(out, "w") as f:
        f.attrs["File_Version"] = "2.0"

    with pytest.raises(UnsupportedFileVersion, match="2.0"):
        MachineConfigReader(out)

    result = open_machine_config(out)
    assert not result.ok
    assert result.error.code == "UnsupportedVersion"



# ===========================================================================
# include_binary flag
# ===========================================================================

class TestIncludeBinaryFlag:
    """to_json() include_binary=False (default) omits the raw fc3 bytes; True restores them.
    ClearBox's correction grids are extension datasets and always appear in full."""

    def test_default_excludes_raw_bytes(self, reference_reader):
        output = json.loads(reference_reader.to_json())
        sfcf = output["optical_trains"][0]["scan_field_correction_file"]
        assert "raw_bytes" not in sfcf

    def test_include_binary_adds_raw_bytes(self, reference_reader):
        output = json.loads(reference_reader.to_json(include_binary=True))
        sfcf = output["optical_trains"][0]["scan_field_correction_file"]
        assert "raw_bytes" in sfcf
        assert sfcf["raw_bytes"]  # non-empty base64 string

    def test_correction_grids_present_regardless_of_flag(self, reference_reader):
        for flag in (False, True):
            output = json.loads(reference_reader.to_json(include_binary=flag))
            train = output["extensions"]["ClearBox"]["children"]["Optical_Train_01"]
            assert train["datasets"]["Correction_Data"]["shape"] == [257, 257, 2]
            assert len(train["datasets"]["Inverse_Correction_Data"]["data"]) == 257 * 257 * 2
