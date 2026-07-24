"""Python library smoke test — run from repo root before Phase 1.7 commit.

Exercises the public API as a real caller would after `pip install machine-config-library`.
Complements the pytest suite: catches missing exports, path-dependent bugs, and API usability
issues that pytest's pythonpath injection can mask.

Usage:
    .venv/Scripts/python.exe scratch/smoke_test.py    (Git Bash)
    .venv\\Scripts\\python.exe scratch/smoke_test.py  (PowerShell)
"""
from __future__ import annotations

import json
import pathlib
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).parent.parent
FIXTURE   = REPO_ROOT / "fixtures" / "reference_config.h5"
FIXTURE_OPCUA = REPO_ROOT / "fixtures" / "reference_config_opcua.h5"


def check(label: str, condition: bool, detail: str = "") -> None:
    if condition:
        print(f"  PASS  {label}")
    else:
        print(f"  FAIL  {label}" + (f": {detail}" if detail else ""))
        sys.exit(1)


def section(title: str) -> None:
    print(f"\n-- {title} " + "-" * max(0, 60 - len(title)))


# ---------------------------------------------------------------------------
# 1. Imports — verify everything in __init__.py is actually importable
# ---------------------------------------------------------------------------
section("Imports")

from machine_config import (           # noqa: E402
    BuildPlate,
    ClearBox,
    Collimator,
    ConfigEditor,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    MachineConfigReader,
    MachineConfigWriter,
    MockConfigBuilder,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
    YamlConfigBuilder,
    config_from_dict,
)
from machine_config.builder import MockConfigBuilder, YamlConfigBuilder, ConfigEditor  # noqa: F811
from machine_config.schema import SCHEMA

check("All public symbols importable from machine_config", True)
check("SCHEMA is a dict with '$schema' key", isinstance(SCHEMA, dict) and "$schema" in SCHEMA)

# ---------------------------------------------------------------------------
# 2. Parse the reference fixture
# ---------------------------------------------------------------------------
section("Parse reference fixture")

config = MachineConfigReader(FIXTURE).parse()

check("meta.machine_name is non-empty",      bool(config.meta.machine_name))
check("meta.configuration_hash is 64 chars", len(config.meta.configuration_hash) == 64)
check("meta.file_version == '1.0'",          config.meta.file_version == "1.0")
check("machine.build_plate.x == 250.0",      config.machine.build_plate.x == 250.0)
check("machine.build_plate.y == 250.0",      config.machine.build_plate.y == 250.0)
check("machine.build_plate.z == 20.0",       config.machine.build_plate.z == 20.0)
check("2 optical trains",                    len(config.optical_trains) == 2)

t0 = config.optical_trains[0]
t1 = config.optical_trains[1]

check("train 0 working_distance == 670.0",   t0.scanner.working_distance == 670.0)
check("train 0 offset_x == -87.5",           t0.scanner.scan_head_offset_x == -87.5)
check("train 0 offset_y == 23.5",            t0.scanner.scan_head_offset_y == 23.5)
check("train 0 thermal_lensing == False",    t0.thermal_lensing_passed is False)
check("train 1 thermal_lensing == True",     t1.thermal_lensing_passed is True)
check("train 0 clearbox present",            t0.clearbox is not None)
check("train 0 sfcf file_size == 1138799",   t0.scan_field_correction_file.file_size == 1138799)
check("train 1 sfcf file_size == 1142763",   t1.scan_field_correction_file.file_size == 1142763)

# ---------------------------------------------------------------------------
# 3. JSON export and schema validation
# ---------------------------------------------------------------------------
section("JSON export + schema validation")

import jsonschema  # noqa: E402

reader = MachineConfigReader(FIXTURE)
json_str = reader.to_json(indent=2)
parsed = json.loads(json_str)

check("to_json() produces valid JSON",            isinstance(parsed, dict))
check("JSON contains 'meta' key",                 "meta" in parsed)
check("JSON contains 'optical_trains' key",       "optical_trains" in parsed)
check("JSON machine_name matches",
      parsed["meta"]["machine_name"] == config.meta.machine_name)

try:
    jsonschema.validate(parsed, SCHEMA)
    check("JSON validates against schema", True)
except jsonschema.ValidationError as e:
    check("JSON validates against schema", False, str(e.message))

# ---------------------------------------------------------------------------
# 4. config_from_dict — JSON → MachineConfig roundtrip
# ---------------------------------------------------------------------------
section("config_from_dict (JSON → MachineConfig)")

config2 = config_from_dict(parsed)

check("machine_name survives JSON roundtrip",
      config2.meta.machine_name == config.meta.machine_name)
check("configuration_hash survives JSON roundtrip",
      config2.meta.configuration_hash == config.meta.configuration_hash)
check("train count survives JSON roundtrip",
      len(config2.optical_trains) == len(config.optical_trains))

# ---------------------------------------------------------------------------
# 5. MachineConfigWriter — HDF5 roundtrip
# ---------------------------------------------------------------------------
section("MachineConfigWriter (HDF5 roundtrip)")

with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    rt_path = pathlib.Path(f.name)

try:
    MachineConfigWriter(config).write(rt_path)
    config3 = MachineConfigReader(rt_path).parse()

    check("machine_name survives HDF5 roundtrip",
          config3.meta.machine_name == config.meta.machine_name)
    check("configuration_hash survives HDF5 roundtrip",
          config3.meta.configuration_hash == config.meta.configuration_hash)
    check("build_plate.x survives HDF5 roundtrip",
          config3.machine.build_plate.x == config.machine.build_plate.x)
    check("train count survives HDF5 roundtrip",
          len(config3.optical_trains) == len(config.optical_trains))
    check("offset_x train 0 survives HDF5 roundtrip",
          config3.optical_trains[0].scanner.scan_head_offset_x == -87.5)
    check("thermal_lensing train 0 survives HDF5 roundtrip",
          config3.optical_trains[0].thermal_lensing_passed is False)
    check("thermal_lensing train 1 survives HDF5 roundtrip",
          config3.optical_trains[1].thermal_lensing_passed is True)
finally:
    rt_path.unlink(missing_ok=True)

# ---------------------------------------------------------------------------
# 6. MockConfigBuilder
# ---------------------------------------------------------------------------
section("MockConfigBuilder")

with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    mock_path = pathlib.Path(f.name)

try:
    MockConfigBuilder(n_lasers=2, machine_name="SmokeTestMachine").save(mock_path)
    mock = MachineConfigReader(mock_path).parse()

    check("mock machine_name matches constructor arg",
          mock.meta.machine_name == "SmokeTestMachine")
    check("mock produces 2 optical trains",  len(mock.optical_trains) == 2)
    check("mock clearbox present train 0",   mock.optical_trains[0].clearbox is not None)
    check("mock correction_data_shape is (257, 257, 2)",
          mock.optical_trains[0].clearbox.correction_data_shape == (257, 257, 2))
finally:
    mock_path.unlink(missing_ok=True)

# 1-laser, no clearbox
with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    mock1_path = pathlib.Path(f.name)

try:
    MockConfigBuilder(n_lasers=1, include_clearbox=False).save(mock1_path)
    mock1 = MachineConfigReader(mock1_path).parse()

    check("mock 1-laser: 1 optical train",       len(mock1.optical_trains) == 1)
    check("mock no-clearbox: clearbox is None",  mock1.optical_trains[0].clearbox is None)
finally:
    mock1_path.unlink(missing_ok=True)

# ---------------------------------------------------------------------------
# 7. YamlConfigBuilder
# ---------------------------------------------------------------------------
section("YamlConfigBuilder")

import yaml  # noqa: E402

spec = {
    "machine": {
        "name": "SmokeYAML",
        "manufacturer": "TestCo",
        "build_plate_x": 300,
        "build_plate_y": 280,
    },
    "optical_trains": [
        {
            "scanner": {"working_distance": 670, "scan_head_offset_x": -87.5},
            "light_source": {"wavelength": 1070, "power_max_nominal": 1000},
        }
    ],
}

with tempfile.NamedTemporaryFile(suffix=".yaml", delete=False, mode="w", encoding="utf-8") as f:
    yaml.dump(spec, f)
    yaml_path = pathlib.Path(f.name)

with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    yaml_out = pathlib.Path(f.name)

try:
    YamlConfigBuilder(yaml_path).save(yaml_out)
    yc = MachineConfigReader(yaml_out).parse()

    check("YAML machine_name matches spec",   yc.meta.machine_name == "SmokeYAML")
    check("YAML build_plate_x == 300.0",      yc.machine.build_plate.x == 300.0)
    check("YAML build_plate_y == 280.0",      yc.machine.build_plate.y == 280.0)
    check("YAML 1 optical train",             len(yc.optical_trains) == 1)
    check("YAML working_distance == 670.0",   yc.optical_trains[0].scanner.working_distance == 670.0)
    check("YAML offset_x == -87.5",           yc.optical_trains[0].scanner.scan_head_offset_x == -87.5)
    check("YAML no clearbox",                 yc.optical_trains[0].clearbox is None)
finally:
    yaml_path.unlink(missing_ok=True)
    yaml_out.unlink(missing_ok=True)

# ---------------------------------------------------------------------------
# 8. ConfigEditor
# ---------------------------------------------------------------------------
section("ConfigEditor")

with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    src_path = pathlib.Path(f.name)
with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
    dst_path = pathlib.Path(f.name)

try:
    MockConfigBuilder(n_lasers=2).save(src_path)
    editor = ConfigEditor(src_path)
    editor.set_scanner_offset(train_index=0, x=-99.9, y=7.7)
    editor.save(dst_path)

    edited = MachineConfigReader(dst_path).parse()
    original = MachineConfigReader(src_path).parse()

    check("ConfigEditor: offset_x updated",
          abs(edited.optical_trains[0].scanner.scan_head_offset_x - (-99.9)) < 1e-6)
    check("ConfigEditor: offset_y updated",
          abs(edited.optical_trains[0].scanner.scan_head_offset_y - 7.7) < 1e-6)
    check("ConfigEditor: train 1 offset_x unchanged",
          edited.optical_trains[1].scanner.scan_head_offset_x ==
          original.optical_trains[1].scanner.scan_head_offset_x)
    check("ConfigEditor: source file not modified",
          original.optical_trains[0].scanner.scan_head_offset_x !=
          edited.optical_trains[0].scanner.scan_head_offset_x)
finally:
    src_path.unlink(missing_ok=True)
    dst_path.unlink(missing_ok=True)

# ---------------------------------------------------------------------------
# 9. OPCUA raw group access
# ---------------------------------------------------------------------------
section("OPCUA raw group access")

opcua_reader = MachineConfigReader(FIXTURE_OPCUA)

client = opcua_reader.get_raw_group("OPCUA/Client")
check("OPCUA/Client is non-empty dict",       isinstance(client, dict) and len(client) > 0)
check("OPCUA Server_URL is present",          "Server_URL" in client)
check("OPCUA Auth_Mode == UsernamePassword",  client.get("Auth_Mode") == "UsernamePassword")
check("OPCUA Security_Mode == SignAndEncrypt",client.get("Security_Mode") == "SignAndEncrypt")

pipe = opcua_reader.get_raw_group("OPCUA/Pipe")
check("OPCUA/Pipe Pipe_Enabled == 1",         int(pipe.get("Pipe_Enabled", -1)) == 1)

interlock = opcua_reader.get_raw_group("OPCUA/Triggers/Laser Emission Interlock")
check("OPCUA trigger ID == trigger_1",        interlock.get("ID") == "trigger_1")

no_opcua = MachineConfigReader(FIXTURE).get_raw_group("OPCUA")
check("Standard fixture has no OPCUA group", no_opcua == {})

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
print(f"\n{'=' * 62}")
print("  All smoke checks passed -- Python library API is correct.")
print(f"{'=' * 62}\n")
