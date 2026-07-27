"""Phase 1.3 / 1.6 — MachineConfigReader tests against real AconityMIDI fixtures."""
import json
import warnings

import numpy as np
import pytest

from machine_config.builder import MockConfigBuilder
from machine_config.reader import MachineConfigReader
from machine_config.schema import SCHEMA


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
# ClearBox
# ===========================================================================

class TestClearBox:
    def test_clearbox_present(self, reference_config):
        for train in reference_config.optical_trains:
            assert train.clearbox is not None

    def test_correction_data_shape(self, reference_config):
        data = reference_config.optical_trains[0].clearbox.correction_data
        assert data is not None
        assert len(data) == 257
        assert len(data[0]) == 257
        assert len(data[0][0]) == 2

    def test_inverse_correction_data_shape(self, reference_config):
        data = reference_config.optical_trains[0].clearbox.inverse_correction_data
        assert data is not None
        assert len(data) == 257
        assert len(data[0]) == 257
        assert len(data[0][0]) == 2

    def test_correction_data_values_are_float_or_none(self, reference_config):
        data = reference_config.optical_trains[0].clearbox.correction_data
        # Sample a few cells to verify type contract
        for i in range(0, 257, 64):
            for j in range(0, 257, 64):
                for k in range(2):
                    v = data[i][j][k]
                    assert v is None or isinstance(v, float), (
                        f"Expected float or None at [{i}][{j}][{k}], got {type(v)}"
                    )

    def test_correction_data_numpy_api_unchanged(self, reference_reader):
        """get_correction_data() still returns the raw numpy array."""
        data = reference_reader.get_correction_data(0)
        assert data.shape == (257, 257, 2)
        assert data.dtype == np.float64

    def test_inverse_correction_data_numpy_api_unchanged(self, reference_reader):
        data = reference_reader.get_inverse_correction_data(0)
        assert data.shape == (257, 257, 2)
        assert data.dtype == np.float64

    def test_clearbox_ip_address(self, reference_config):
        assert reference_config.optical_trains[0].clearbox.ip_address != ""

    def test_clearbox_data_port_is_int(self, reference_config):
        dp = reference_config.optical_trains[0].clearbox.data_port
        assert dp is None or isinstance(dp, int)


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
# OPCUA — model-level (parse() result on reference_config_opcua.h5)
# These complement TestOpcua (get_raw_group level) and assert that parse()
# now returns a fully-populated OpcuaConfig object.
# ===========================================================================

class TestOpcuaModel:
    def test_opcua_is_none_on_reference_config(self, reference_config):
        """Standard fixture has no OPCUA group — config.opcua must be None."""
        assert reference_config.opcua is None

    def test_opcua_is_populated_on_opcua_fixture(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua is not None

    def test_opcua_client_server_url(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.client.server_url == (
            "opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"
        )

    def test_opcua_client_auth_mode(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.client.auth_mode == "UsernamePassword"

    def test_opcua_client_integer_fields(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.client.bfs_max_depth == 16
        assert config.opcua.client.publish_interval == 250
        assert config.opcua.client.sampling_interval == 250
        assert config.opcua.client.session_timeout == 60000

    def test_opcua_pipe_enabled(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.pipe.pipe_enabled is True

    def test_opcua_pipe_buffer_size(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.pipe.buffer_size == 65536

    def test_opcua_triggers_enabled(self, opcua_reader):
        config = opcua_reader.parse()
        assert config.opcua.triggers_enabled is True

    def test_opcua_trigger_laser_emission_interlock(self, opcua_reader):
        config = opcua_reader.parse()
        t = config.opcua.triggers["Laser Emission Interlock"]
        assert t.id == "trigger_1"
        assert t.signal == "yellow_light"
        assert t.subsystem == "Chamber"
        assert t.rule_enabled is True

    def test_opcua_trigger_chamber_oxygen(self, opcua_reader):
        config = opcua_reader.parse()
        t = config.opcua.triggers["Chamber Oxygen Level"]
        assert t.id == "trigger_2"
        assert t.signal == "oxygen_level"
        assert t.start_value == "700"
        assert t.stop_value == "1000"


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


def test_absent_clearbox_is_none(tmp_path):
    builder = MockConfigBuilder(n_lasers=1, include_clearbox=False)
    out = tmp_path / "no_clearbox.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert config.optical_trains[0].clearbox is None


def test_file_version_warning(tmp_path):
    builder = MockConfigBuilder(n_lasers=1, file_version="2.0")
    out = tmp_path / "future.h5"
    builder.save(out)
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        MachineConfigReader(out).parse()
    assert any("File_Version" in str(warning.message) for warning in w)


# ===========================================================================
# include_binary flag
# ===========================================================================

class TestIncludeBinaryFlag:
    """to_json() include_binary=False (default) omits correction arrays and
    raw fc3 bytes; include_binary=True restores them."""

    def test_default_excludes_correction_data(self, reference_reader):
        output = json.loads(reference_reader.to_json())
        cb = output["optical_trains"][0]["clearbox"]
        assert "correction_data" not in cb

    def test_default_excludes_inverse_correction_data(self, reference_reader):
        output = json.loads(reference_reader.to_json())
        cb = output["optical_trains"][0]["clearbox"]
        assert "inverse_correction_data" not in cb

    def test_default_excludes_raw_bytes(self, reference_reader):
        output = json.loads(reference_reader.to_json())
        sfcf = output["optical_trains"][0]["scan_field_correction_file"]
        assert "raw_bytes" not in sfcf

    def test_include_binary_adds_correction_data(self, reference_reader):
        output = json.loads(reference_reader.to_json(include_binary=True))
        cb = output["optical_trains"][0]["clearbox"]
        assert "correction_data" in cb
        assert len(cb["correction_data"]) > 0

    def test_include_binary_adds_inverse_correction_data(self, reference_reader):
        output = json.loads(reference_reader.to_json(include_binary=True))
        cb = output["optical_trains"][0]["clearbox"]
        assert "inverse_correction_data" in cb
        assert len(cb["inverse_correction_data"]) > 0

    def test_include_binary_adds_raw_bytes(self, reference_reader):
        output = json.loads(reference_reader.to_json(include_binary=True))
        sfcf = output["optical_trains"][0]["scan_field_correction_file"]
        assert "raw_bytes" in sfcf
        assert sfcf["raw_bytes"]  # non-empty base64 string

    def test_clearbox_scalar_fields_present_regardless_of_flag(self, reference_reader):
        """Scalar clearbox fields appear in both modes."""
        for flag in (False, True):
            output = json.loads(reference_reader.to_json(include_binary=flag))
            cb = output["optical_trains"][0]["clearbox"]
            assert "ip_address" in cb
            assert "data_port" in cb
            assert "correction_grid_domain_shape" in cb
