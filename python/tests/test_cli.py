"""Phase 1.4 — CLI tests using Click's CliRunner."""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from click.testing import CliRunner

from machine_config.cli import main

_REPO_ROOT = Path(__file__).parent.parent.parent
REFERENCE_H5 = str(_REPO_ROOT / "fixtures" / "reference_config.h5")


@pytest.fixture
def runner() -> CliRunner:
    return CliRunner()


# ===========================================================================
# inspect
# ===========================================================================

class TestInspect:
    def test_exit_code_zero(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert result.exit_code == 0, result.output

    def test_shows_machine_name(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "TM-LPBF-02" in result.output

    def test_shows_manufacturer(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "Aconity3D" in result.output

    def test_shows_build_plate_dimension(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "250" in result.output

    def test_shows_optical_train_count(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "Optical Trains: 2" in result.output

    def test_shows_working_distance(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "670" in result.output

    def test_shows_both_train_ids(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5])
        assert "Optical_Train_01" in result.output
        assert "Optical_Train_02" in result.output

    def test_verbose_shows_collimator(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert result.exit_code == 0, result.output
        assert "collimator:" in result.output

    def test_verbose_shows_scanner_card_model(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "SP-ICE-3" in result.output

    def test_verbose_shows_thermal_lensing_result(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "thermal_lensing_passed: false" in result.output  # train 01
        assert "thermal_lensing_passed: true" in result.output   # train 02

    def test_verbose_shows_clearbox(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "clearbox:" in result.output

    def test_verbose_shows_correction_file(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "scan_field_correction_file:" in result.output

    def test_verbose_shows_beam_waist(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "beam_waist_major:" in result.output

    def test_verbose_shows_m2(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "m2_major:" in result.output

    def test_verbose_shows_rayleigh(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "rayleigh_length_major:" in result.output

    def test_verbose_shows_power(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "power_max_nominal:" in result.output

    def test_verbose_shows_full_config_header(self, runner):
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "Full Configuration" in result.output

    def test_verbose_no_duplicate_manufacturer_in_device_id(self, runner):
        # old curated block had "IPG IPG" — YAML just emits the raw field value
        result = runner.invoke(main, ["inspect", REFERENCE_H5, "--verbose"])
        assert "IPG IPG" not in result.output

    def test_nonexistent_file_exits_nonzero(self, runner):
        result = runner.invoke(main, ["inspect", "nonexistent_config.h5"])
        assert result.exit_code != 0


# ===========================================================================
# validate
# ===========================================================================

class TestValidate:
    def test_passes_reference_fixture(self, runner):
        result = runner.invoke(main, ["validate", REFERENCE_H5])
        assert result.exit_code == 0, result.output

    def test_output_contains_pass(self, runner):
        result = runner.invoke(main, ["validate", REFERENCE_H5])
        assert "PASS" in result.output

    def test_output_contains_machine_name(self, runner):
        result = runner.invoke(main, ["validate", REFERENCE_H5])
        assert "TM-LPBF-02" in result.output

    def test_output_contains_train_count(self, runner):
        result = runner.invoke(main, ["validate", REFERENCE_H5])
        assert "2 optical train" in result.output


# ===========================================================================
# export-json
# ===========================================================================

class TestExportJson:
    def test_stdout_is_valid_json(self, runner):
        result = runner.invoke(main, ["export-json", REFERENCE_H5])
        assert result.exit_code == 0, result.output
        parsed = json.loads(result.output)
        assert isinstance(parsed, dict)

    def test_stdout_machine_name(self, runner):
        result = runner.invoke(main, ["export-json", REFERENCE_H5])
        parsed = json.loads(result.output)
        assert parsed["meta"]["machine_name"] == "TM-LPBF-02: AconityMIDI+_OG"

    def test_stdout_optical_train_count(self, runner):
        result = runner.invoke(main, ["export-json", REFERENCE_H5])
        parsed = json.loads(result.output)
        assert len(parsed["optical_trains"]) == 2

    def test_to_file(self, runner, tmp_path):
        out = str(tmp_path / "output.json")
        result = runner.invoke(main, ["export-json", REFERENCE_H5, "--output", out])
        assert result.exit_code == 0
        data = json.loads(Path(out).read_text(encoding="utf-8"))
        assert data["meta"]["machine_name"] == "TM-LPBF-02: AconityMIDI+_OG"

    def test_to_file_confirmation_message(self, runner, tmp_path):
        out = str(tmp_path / "output.json")
        result = runner.invoke(main, ["export-json", REFERENCE_H5, "--output", out])
        assert "Written to" in result.output

    def test_indent_option(self, runner):
        result_2 = runner.invoke(main, ["export-json", REFERENCE_H5, "--indent", "2"])
        result_4 = runner.invoke(main, ["export-json", REFERENCE_H5, "--indent", "4"])
        assert result_4.exit_code == 0
        # 4-space indent produces wider output than 2-space
        assert len(result_4.output) > len(result_2.output)

    def test_default_excludes_correction_data(self, runner):
        result = runner.invoke(main, ["export-json", REFERENCE_H5])
        parsed = json.loads(result.output)
        cb = parsed["optical_trains"][0]["clearbox"]
        assert "correction_data" not in cb

    def test_include_binary_flag_adds_correction_data(self, runner):
        result = runner.invoke(main, ["export-json", REFERENCE_H5, "--include-binary"])
        assert result.exit_code == 0
        parsed = json.loads(result.output)
        cb = parsed["optical_trains"][0]["clearbox"]
        assert "correction_data" in cb
        assert len(cb["correction_data"]) > 0


# ===========================================================================
# Phase 1.5 — write / build / demo commands
# ===========================================================================

class TestPhase15Stubs:
    """These tests originally checked stub exit-code behaviour.
    Phase 1.5 is now implemented; they now verify the commands actually work.
    """

    def test_write_exits_2(self, runner):
        # write with a non-existent JSON file should still exit non-zero via Click
        result = runner.invoke(main, ["write", "config.json", "--output", "out.h5"])
        assert result.exit_code != 0

    def test_build_mock_exits_2(self, runner, tmp_path):
        # build --mock should now succeed (exit 0)
        result = runner.invoke(
            main, ["build", "--mock", "--output", str(tmp_path / "out.h5")]
        )
        assert result.exit_code == 0

    def test_build_yaml_exits_2(self, runner, tmp_path):
        # build --from-yaml with a non-existent file should exit non-zero via Click
        result = runner.invoke(
            main, ["build", "--from-yaml", "spec.yaml", "--output", str(tmp_path / "out.h5")]
        )
        assert result.exit_code != 0

    def test_demo_exits_2(self, runner):
        # demo should now succeed (exit 0)
        result = runner.invoke(main, ["demo"])
        assert result.exit_code == 0

    def test_write_error_message_mentions_phase(self, runner):
        # With a missing JSON file, Click emits a "does not exist" message
        result = runner.invoke(main, ["write", "config.json", "--output", "out.h5"])
        assert result.exit_code != 0
        assert "config.json" in result.output

    def test_build_error_message_mentions_phase(self, runner, tmp_path):
        # With --mock the build now works; confirm success message is present
        result = runner.invoke(
            main, ["build", "--mock", "--output", str(tmp_path / "out.h5")]
        )
        assert result.exit_code == 0
        assert "Written to" in result.output


# ===========================================================================
# CLI meta
# ===========================================================================

class TestCliMeta:
    def test_version_flag(self, runner):
        result = runner.invoke(main, ["--version"])
        assert result.exit_code == 0
        assert "0.1.0" in result.output

    def test_help_lists_all_commands(self, runner):
        result = runner.invoke(main, ["--help"])
        assert result.exit_code == 0
        for cmd in ("inspect", "validate", "export-json", "write", "build", "demo"):
            assert cmd in result.output

    def test_inspect_help(self, runner):
        result = runner.invoke(main, ["inspect", "--help"])
        assert result.exit_code == 0

    def test_validate_help(self, runner):
        result = runner.invoke(main, ["validate", "--help"])
        assert result.exit_code == 0

    def test_export_json_help(self, runner):
        result = runner.invoke(main, ["export-json", "--help"])
        assert result.exit_code == 0
