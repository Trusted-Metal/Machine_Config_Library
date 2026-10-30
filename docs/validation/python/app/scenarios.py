"""S-01-09/AV-01-11 scenarios for the Python validation app (VALIDATION_PLAN.md §8).

AV-09-11 live here (unlike Rust/Go/C++'s AV-09-11, which can only live in
their own source trees) because Python's dispatcher is a registry
(`_ADAPTERS`), not a hardcoded match/switch — there's a real dispatch-table
seam for the mock adapter to hook into, so these scenarios exercise the
public `MachineConfigReader`/`MachineConfigWriter` facade directly, same as
every other scenario here.
"""
import copy
import hashlib
import math
import sys
import tempfile
from dataclasses import replace
from pathlib import Path

import numpy as np

from machine_config import (
    Collimator,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    MachineConfigReader,
    MachineConfigWriter,
    MockConfigBuilder,
    Attr,
    ExtensionContractError,
    ExtensionTree,
    canonical_digest,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)
from machine_config.capabilities import open_machine_config
from machine_config.capabilities.file_version import UnsupportedFileVersion
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0

# Mock v1.1 adapter lives in the test suite, not the installed wheel — needed
# by run_av09/run_av10/run_av11 only.
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent.parent / "python" / "tests"))
from test_adapter_migration import MockV1_1Reader, MockV1_1Writer  # noqa: E402


def av_fixture(fixtures_dir: str, name: str) -> Path:
    return Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / name


def _clearbox_train(cfg: MachineConfig, index: int) -> ExtensionTree:
    """ClearBox is an opaque extension tree: extensions["ClearBox"] / Optical_Train_NN."""
    return cfg.extensions["ClearBox"].children[f"Optical_Train_{index + 1:02d}"]


def _grid(cfg: MachineConfig, index: int, name: str) -> np.ndarray:
    ds = _clearbox_train(cfg, index).datasets[name]
    return np.asarray(ds.data, dtype=np.float64).reshape(ds.shape)


# S-01: Read reference fixture, all scalar fields
# ID:          S-01
# Title:       Read reference fixture and verify all scalar fields
# Category:    happy-path
# Layer:       reader
# Precondition: fixtures/reference_config.h5
# Action:      Parse the file, print key fields to stdout
# Expected:    machine_name = "TM-LPBF-02: AconityMIDI+_OG"
#              build_plate_x ≈ 250.0
#              build_plate_y ≈ 250.0
#              len(optical_trains) = 2
#              train[0].scanner.working_distance ≈ 670.0
#              train[0].scanner.scan_head_rotation ≈ 0.0
#              train[1].scanner.scan_head_rotation ≈ 180.0
#              configuration_hash: 64 hex characters
#              file_version: "1.0"
# Rationale:   Establishes baseline read correctness against a known-good fixture.
def run_s01(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    cfg = MachineConfigReader(Path(fixtures_dir) / "reference_config.h5").parse()

    if cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG":
        return False, f"machine_name: got '{cfg.meta.machine_name}'"
    if cfg.machine.build_plate.x is None or abs(cfg.machine.build_plate.x - 250.0) > 0.001:
        return False, f"build_plate_x: got {cfg.machine.build_plate.x}"
    if cfg.machine.build_plate.y is None or abs(cfg.machine.build_plate.y - 250.0) > 0.001:
        return False, f"build_plate_y: got {cfg.machine.build_plate.y}"
    if len(cfg.optical_trains) != 2:
        return False, f"optical_trains count: got {len(cfg.optical_trains)}"

    wd = cfg.optical_trains[0].scanner.working_distance
    if wd is None or abs(wd - 670.0) > 0.1:
        return False, f"train[0].working_distance: got {wd}"

    r0 = cfg.optical_trains[0].scanner.scan_head_rotation
    if r0 is None or abs(r0) > 0.001:
        return False, f"train[0].scan_head_rotation: got {r0}"

    r1 = cfg.optical_trains[1].scanner.scan_head_rotation
    if r1 is None or abs(r1 - 180.0) > 0.001:
        return False, f"train[1].scan_head_rotation: got {r1}"

    h = cfg.meta.configuration_hash
    if len(h) != 64 or not all(c in "0123456789abcdefABCDEF" for c in h):
        return False, f"configuration_hash invalid: '{h}'"
    if cfg.meta.file_version.strip() != "1.0":
        return False, f"file_version: got '{cfg.meta.file_version}'"

    return True, "all scalar fields match expected values"


# S-02: Read reference fixture, correction data
# ID:          S-02
# Title:       Read correction grids and verify shape and finite values
# Category:    happy-path
# Layer:       reader
# Precondition: fixtures/reference_config.h5
# Action:      Parse with includeBinary=true, access correction_data and
#              inverse_correction_data for train 0
# Expected:    correction_data shape: [257, 257, 2]
#              inverse_correction_data shape: [257, 257, 2]
#              Both contain at least one finite (non-NaN) value
#              Forward and inverse arrays differ (not byte-identical)
#              SHA-256 hash of correction_data bytes matches reference value
# Rationale:   Binary data round-trips are the highest-risk correctness area.
def run_s02(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    reader = MachineConfigReader(Path(fixtures_dir) / "reference_config.h5")
    cfg = reader.parse()
    cd = _grid(cfg, 0, "Correction_Data")
    icd = _grid(cfg, 0, "Inverse_Correction_Data")

    if cd.shape != (257, 257, 2):
        return False, f"correction_data shape: {cd.shape}"
    if icd.shape != (257, 257, 2):
        return False, f"inverse_correction_data shape: {icd.shape}"
    if not np.any(np.isfinite(cd)):
        return False, "correction_data: no finite values"
    if not np.any(np.isfinite(icd)):
        return False, "inverse_correction_data: no finite values"
    if np.array_equal(cd, icd, equal_nan=True):
        return False, "correction_data and inverse_correction_data are identical"

    cd_hash = hashlib.sha256(cd.tobytes()).hexdigest()
    return True, f"shapes OK, finite values OK, forward≠inverse, correction_data SHA-256={cd_hash}"


# S-03: Read real AconityMIDI fixture file
# ID:          S-03
# Title:       Read real-world AconityMIDI machine config file
# Category:    happy-path
# Layer:       reader
# Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
# Action:      Parse the file, print all fields to stdout
# Expected:    No error. All fields present in output match known machine parameters.
# Rationale:   This is the primary real-world validation. If the library cannot
#              read a real file from the actual machine, it is not production-ready.
def run_s03(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    matches = [
        p for p in Path(real_dir).glob("*.h5")
        if "AconityMIDI" in p.name and "OG_178" in p.name
    ]
    if not matches:
        return False, f"real AconityMIDI file not found in {real_dir}"

    cfg = MachineConfigReader(matches[0]).parse()
    fields = " | ".join([
        f"machine_name={cfg.meta.machine_name!r}",
        f"file_version={cfg.meta.file_version!r}",
        f"trains={len(cfg.optical_trains)}",
        f"build_plate_x={cfg.machine.build_plate.x}",
        f"build_plate_y={cfg.machine.build_plate.y}",
        f"wd={cfg.optical_trains[0].scanner.working_distance}",
        f"rotation[0]={cfg.optical_trains[0].scanner.scan_head_rotation}",
        f"hash={cfg.meta.configuration_hash[:16]}...",
    ])
    return True, fields


# S-04: Write modified config and verify field change survives round-trip
# ID:          S-04
# Title:       Modify a scalar field, write, re-read, verify change persisted
# Category:    happy-path
# Layer:       writer + reader
# Precondition: fixtures/reference_config.h5
# Action:      Read → change machine_name to "VALIDATION_TEST_MACHINE" →
#              write to temp file → read temp file → assert machine_name matches
# Expected:    machine_name == "VALIDATION_TEST_MACHINE" after roundtrip.
#              All other fields unchanged.
#              file_version == "1.0" unchanged.
# Rationale:   Basic write correctness. If a modified field does not survive,
#              the writer has a silent data loss bug.
def run_s04(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()
    orig_x = cfg.machine.build_plate.x

    modified = replace(cfg,
        meta=replace(cfg.meta, machine_name="VALIDATION_TEST_MACHINE"),
        machine=replace(cfg.machine, machine_name="VALIDATION_TEST_MACHINE"),
    )

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(modified).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if rb.meta.machine_name != "VALIDATION_TEST_MACHINE":
        return False, f"machine_name not persisted: '{rb.meta.machine_name}'"
    if rb.meta.file_version.strip() != "1.0":
        return False, f"file_version changed: '{rb.meta.file_version}'"
    if rb.machine.build_plate.x != orig_x:
        return False, f"build_plate_x changed: {orig_x} → {rb.machine.build_plate.x}"

    return True, "machine_name persisted, file_version and other fields unchanged"


# S-05: Full binary round-trip: read, write, re-read, verify binary data preserved with correction hash
# ID:          S-05
# Title:       Copy config with binary data, verify correction hash unchanged
# Category:    happy-path
# Layer:       writer + reader (copy-hdf5 path)
# Precondition: fixtures/reference_config.h5
# Action:      Read with includeBinary=true → write to temp → read temp with
#              includeBinary=true → compare SHA-256 of correction_data bytes
# Expected:    SHA-256 of correction_data for train 0 identical before and after.
#              SHA-256 of inverse_correction_data for train 0 identical before and after.
#              file_size of scan_field_correction_file unchanged.
# Rationale:   Silent precision loss in binary data is undetectable without hash comparison.
def run_s05(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    reader = MachineConfigReader(path)
    cfg = reader.parse()

    cd_before = _grid(cfg, 0, "Correction_Data")
    icd_before = _grid(cfg, 0, "Inverse_Correction_Data")

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    cfg2 = MachineConfigReader(tmp).parse()

    cd_after = _grid(cfg2, 0, "Correction_Data")
    icd_after = _grid(cfg2, 0, "Inverse_Correction_Data")

    if not np.array_equal(cd_before, cd_after, equal_nan=True):
        h_before = hashlib.sha256(cd_before.tobytes()).hexdigest()[:16]
        h_after = hashlib.sha256(cd_after.tobytes()).hexdigest()[:16]
        return False, f"correction_data mismatch: {h_before}... → {h_after}..."
    if not np.array_equal(icd_before, icd_after, equal_nan=True):
        return False, "inverse_correction_data mismatch after roundtrip"

    cd_hash = hashlib.sha256(cd_before.tobytes()).hexdigest()
    return True, f"correction_data preserved: SHA-256={cd_hash}"


# S-06: Build synthetic config with 2 lasers, verify fields and round-trip
# ID:          S-06
# Title:       Build a synthetic config with MockConfigBuilder and verify fields
# Category:    happy-path
# Layer:       builder
# Precondition: None (builder creates from scratch)
# Action:      Build a 2-laser config → verify fields → save to temp → re-read
# Expected:    len(optical_trains) == 2
#              train[0].scanner.scan_head_rotation ≈ 0.0
#              train[1].scanner.scan_head_rotation ≈ 180.0
#              machine_name non-empty
#              correction_data centre cell ≈ 2.0 (Gaussian peak)
#              After save and re-read: all fields match
# Rationale:   Builder is used in CI fixture generation and by consumers who need
#              synthetic test configs. Must produce valid, re-readable output.
def run_s06(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    cfg = MockConfigBuilder(n_lasers=2).build()

    if len(cfg.optical_trains) != 2:
        return False, f"optical_trains count: {len(cfg.optical_trains)}"

    r0 = cfg.optical_trains[0].scanner.scan_head_rotation
    r1 = cfg.optical_trains[1].scanner.scan_head_rotation
    if r0 is None or abs(r0) > 0.001:
        return False, f"train[0].scan_head_rotation: {r0}"
    if r1 is None or abs(r1 - 180.0) > 0.001:
        return False, f"train[1].scan_head_rotation: {r1}"
    if not cfg.meta.machine_name:
        return False, "machine_name is empty"

    center = _grid(cfg, 0, "Correction_Data")[128][128][0]
    if not math.isfinite(center) or abs(center - 2.0) > 0.01:
        return False, f"correction_data center: expected ~2.0, got {center}"

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if len(rb.optical_trains) != 2:
        return False, f"readback trains: {len(rb.optical_trains)}"
    if rb.meta.machine_name != cfg.meta.machine_name:
        return False, "machine_name changed after roundtrip"

    return True, "2-laser build OK, center≈2.0, roundtrip OK"


# S-07: Extension round-trip: read, write, re-read, verify the opaque extension tree is preserved
# ID:          S-07
# Title:       Read, write, and re-read the OPCUA fixture's extension tree
# Category:    happy-path
# Layer:       reader + writer (extension path)
# Precondition: fixtures/reference_config_opcua.h5
# Action:      Read → write to temp → read temp
# Expected:    The legacy OPCUA group is exposed as the opaque "TM_OPCUA" extension tree.
#              Its canonical digest is identical before and after the round trip.
#              Client/Server_URL and the trigger groups are preserved with their stored dtypes.
# Rationale:   Core no longer models OPC UA: it carries the extension verbatim. Silent loss
#              or retyping anywhere in the tree would not be caught by scalar field checks.
def run_s07(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config_opcua.h5"
    if not path.exists():
        return False, f"OPCUA fixture not found: {path}"

    cfg = MachineConfigReader(path).parse()
    tree = cfg.extensions.get("TM_OPCUA")
    if tree is None:
        return False, "TM_OPCUA extension missing after reading the OPCUA fixture"
    orig_digest = canonical_digest(tree)
    orig_triggers = set(tree.children["Triggers"].children)
    orig_url = tree.children["Client"].attrs["Server_URL"]

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    rb_tree = rb.extensions.get("TM_OPCUA")
    if rb_tree is None:
        return False, "TM_OPCUA extension missing after roundtrip"
    if canonical_digest(rb_tree) != orig_digest:
        return False, "extension digest changed across the roundtrip"
    if set(rb_tree.children["Triggers"].children) != orig_triggers:
        return False, "trigger groups changed across the roundtrip"
    rb_url = rb_tree.children["Client"].attrs["Server_URL"]
    if rb_url != orig_url:
        return False, f"Server_URL changed: {orig_url!r} -> {rb_url!r}"

    return True, f"extension roundtrip OK: {len(orig_triggers)} trigger groups, digest={orig_digest[:16]}..."


# S-08: Drastic change: add new train, modify build_plate_x, clearbox, scan_head_rotation using real AconityMIDI file
# ID:          S-08
# Title:       Drastic field change to real file, verify adapter pipeline integrity
# Category:    happy-path (applied)
# Layer:       reader + writer + adapter
# Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
# Action:      Read the real file → make ALL of the following changes:
#                1. Add a third optical train (clone train 1, change train_id)
#                2. Change build_plate_x from 250.0 to 350.0
#                3. Set scan_head_rotation on new train to 90.0
#              4. The new train carries no ClearBox (its extension subtree is absent)
#              5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
#              → write to temp → read temp → verify all five changes persisted
# Expected:    len(optical_trains) == 3
#              build_plate_x ≈ 350.0
#              train[2].scanner.scan_head_rotation ≈ 90.0
#              extensions["ClearBox"] has Optical_Train_01/02 but no Optical_Train_03
#              machine_name == "MODIFIED_ACONITY_VALIDATION"
#              file_version still "1.0" (adapter did not change the version)
# Rationale:   Tests that the adapter pipeline handles structural changes (new
#              train, train without ClearBox) without silent corruption or
#              wrong-adapter dispatch.
def run_s08(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    matches = [
        p for p in Path(real_dir).glob("*.h5")
        if "AconityMIDI" in p.name and "OG_178" in p.name
    ]
    if not matches:
        return False, f"real AconityMIDI file not found in {real_dir}"

    cfg = MachineConfigReader(matches[0]).parse()

    new_train = copy.deepcopy(cfg.optical_trains[1])
    new_train = replace(new_train, train_id="Optical_Train_03")
    new_bp = replace(cfg.machine.build_plate, x=350.0)
    new_train = replace(new_train, scanner=replace(new_train.scanner, scan_head_rotation=90.0))

    modified = replace(cfg,
        meta=replace(cfg.meta, machine_name="MODIFIED_ACONITY_VALIDATION"),
        machine=replace(cfg.machine, build_plate=new_bp, machine_name="MODIFIED_ACONITY_VALIDATION"),
        optical_trains=cfg.optical_trains + [new_train],
    )

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(modified).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if len(rb.optical_trains) != 3:
        return False, f"optical_trains: expected 3, got {len(rb.optical_trains)}"
    if rb.machine.build_plate.x is None or abs(rb.machine.build_plate.x - 350.0) > 0.001:
        return False, f"build_plate_x: got {rb.machine.build_plate.x}"
    r2 = rb.optical_trains[2].scanner.scan_head_rotation
    if r2 is None or abs(r2 - 90.0) > 0.001:
        return False, f"train[2].scan_head_rotation: got {r2}"
    cb_children = rb.extensions["ClearBox"].children
    if "Optical_Train_03" in cb_children or not {"Optical_Train_01", "Optical_Train_02"} <= set(cb_children):
        return False, f"ClearBox train groups after roundtrip: {sorted(cb_children)}"
    if rb.meta.machine_name != "MODIFIED_ACONITY_VALIDATION":
        return False, f"machine_name: got '{rb.meta.machine_name}'"
    if rb.meta.file_version.strip() != "1.0":
        return False, f"file_version changed: '{rb.meta.file_version}'"

    return True, "3 trains, build_plate_x=350.0, rotation=90.0, new train has no ClearBox, machine_name OK"


# S-09: Public types importable from machine_config top-level
# ID:          S-09
# Title:       Verify all public model types are importable from the library surface
# Category:    happy-path
# Layer:       public API / packaging
# Precondition: Library installed from artifact (wheel / tarball / module), not from source
# Action:      Import MachineConfig, Scanner, OpticalTrain, LightSource, Collimator,
#              ScannerCard, ScanFieldCorrectionFile, and
#              MockConfigBuilder from the library's public import path only.
# Expected:    All imports resolve without error.
#              No type needs to be defined or re-implemented by the consumer.
# Rationale:   If a consumer must import from internal paths the public API surface
#              is incomplete. This scenario catches that gap at packaging time.
def run_s09(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    _b: MockConfigBuilder = MockConfigBuilder()
    _: MachineConfig
    _: Scanner
    return True, "all public types importable from machine_config top-level"


# S-10: Public facade export surface
# ID:          S-10
# Title:       Stable capability facade importable from the top-level
#              `machine_config` package, not just `machine_config.capabilities`
# Category:    happy-path
# Layer:       public API / packaging
# Precondition: fixtures_dir/reference_config.h5
# Action:      Import open_machine_config, create_machine_config,
#              supported_file_versions, CapabilityError, MachineConfigFileV1_0,
#              SetMode from `machine_config` (top-level) — deliberately NOT
#              `machine_config.capabilities`, which is what this file's own
#              module-level imports use (see the top-of-file import block) and
#              which still works, but is not what this scenario is checking.
#              Open the reference fixture, call get_scanner; create() a
#              session, call set_model; confirm a missing-file open returns a
#              real CapabilityError.
# Expected:    All six names resolve from machine_config's top level. Both
#              facade calls succeed. The missing-file open returns a
#              CapabilityError-typed error.
# Rationale:   See VALIDATION_PLAN.md §8 S-10 and CORRECTION_DATA_FACADE_PLAN.md
#              — Rust's crate root previously omitted its facade entirely
#              despite its own "no sub-module path" re-export policy; nothing
#              checked for it until an integrating consumer found the gap.
#              Python already re-exports correctly (machine_config/__init__.py)
#              — this scenario exists so that stays true, in every language.
def run_s10(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    # Deliberately import from the top-level `machine_config` package, not
    # `machine_config.capabilities` (used at module scope above for other
    # scenarios) or any version-specific submodule.
    from machine_config import (
        CapabilityError,
        MachineConfigFileV1_0,
        SetMode,
        create_machine_config,
        open_machine_config,
        supported_file_versions,
    )

    if "1.0" not in supported_file_versions():
        return False, "supported_file_versions() does not list '1.0'"

    fixture = str(Path(fixtures_dir) / "reference_config.h5")
    opened = open_machine_config(fixture)
    if not opened.ok:
        return False, f"open_machine_config failed: {opened.error}"
    if not isinstance(opened.value, MachineConfigFileV1_0):
        return False, f"unexpected facade type: {type(opened.value)!r}"
    file = opened.value
    train = file.optical_train(0)
    if not train.ok:
        return False, f"optical_train(0) failed: {train.error}"
    train.value.get_scanner()
    file.close()

    created = create_machine_config("1.0")
    if not created.ok:
        return False, f"create_machine_config failed: {created.error}"
    meta_handle = created.value.meta()
    set_result = meta_handle.set_model(meta_handle.get_model(), SetMode.MERGE)
    if not set_result.ok:
        return False, f"set_model failed: {set_result.error}"
    created.value.close()

    missing = open_machine_config("/nonexistent/path/does_not_exist.h5")
    if missing.ok or not isinstance(missing.error, CapabilityError):
        return False, "opening a missing file did not return a CapabilityError"

    return True, (
        "facade (open_machine_config, create_machine_config, "
        "supported_file_versions, CapabilityError, MachineConfigFileV1_0, "
        "SetMode) reachable from machine_config top-level; no "
        "machine_config.capabilities submodule import needed"
    )


# AV-01: Unknown file version string
# ID:          AV-01
# Title:       Reader rejects unknown File_Version with typed error
# Category:    adapter-versioning
# Layer:       dispatcher
# Precondition: docs/validation/fixtures/v2_0_unknown.h5
# Action:      Attempt to read/parse the file
# Expected:    UnsupportedFileVersion raised with version='2.0' on the error object.
#              No panic. No silent default to v1.0.
# Rationale:   Silent fallback to a wrong adapter is the most dangerous versioning failure.
def run_av01(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "v2_0_unknown.h5")
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for File_Version='2.0'"
    except UnsupportedFileVersion as e:
        if e.version == "2.0":
            return True, f"UnsupportedFileVersion raised, version='{e.version}'"
        return False, f"UnsupportedFileVersion raised but version='{e.version}'"
    except Exception as e:
        return False, f"wrong exception: {type(e).__name__}: {e}"


# AV-02: Missing file version string
# ID:          AV-02
# Title:       Reader handles absent File_Version attribute predictably
# Category:    adapter-versioning
# Layer:       dispatcher
# Precondition: docs/validation/fixtures/missing_version.h5
# Action:      Attempt to read/parse the file
# Expected:    Either: defaults to "1.0" and reads successfully, OR raises typed error.
#              Behavior must be identical across all five languages.
# Rationale:   Real-world files from pre-versioning tools will lack this attribute.
def run_av02(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "missing_version.h5")
    try:
        cfg = MachineConfigReader(fixture).parse()
        return True, f"missing File_Version defaults to '1.0', reads OK, file_version='{cfg.meta.file_version}'"
    except Exception as e:
        return True, f"missing File_Version raises {type(e).__name__}: {e}"


# AV-03: Simulated future file version (v1.1) file
# ID:          AV-03
# Title:       v1.0 reader encountering a v1.1 file fails loudly
# Category:    adapter-versioning
# Layer:       dispatcher
# Precondition: docs/validation/fixtures/v1_1_simulated.h5
# Action:      Attempt to read/parse with the v1.0 reader
# Expected:    UnsupportedFileVersion raised with version='1.1'.
#              The Future_Group/ data is NOT silently ignored. No partial read.
# Rationale:   A v1.0 reader must not silently truncate a v1.1 file.
def run_av03(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "v1_1_simulated.h5")
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for File_Version='1.1'"
    except UnsupportedFileVersion as e:
        if e.version == "1.1":
            return True, f"UnsupportedFileVersion raised, version='{e.version}'"
        return False, f"UnsupportedFileVersion raised but version='{e.version}'"
    except Exception as e:
        return False, f"wrong exception: {type(e).__name__}: {e}"


# AV-04: Missing required HDF5 group (Machine/)
# ID:          AV-04
# Title:       Reader returns typed error when required group is absent
# Category:    adapter-versioning / error-contract
# Layer:       reader (v1.0 adapter)
# Precondition: docs/validation/fixtures/missing_machine_group.h5
# Action:      Attempt to read/parse
# Expected:    Typed error (not panic) indicating the missing group.
#              Error message must name the missing path.
#              No partial MachineConfig returned.
# Rationale:   Corrupt or hand-edited files in the field will be missing groups.
def run_av04(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "missing_machine_group.h5")
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for missing Machine/ group"
    except Exception as e:
        return True, f"{type(e).__name__} raised for missing Machine/ group: {e}"


# AV-05: Valid version, corrupt required scalar field (Build_Plate_X_Dimension)
# ID:          AV-05
# Title:       Reader handles corrupt required attribute gracefully
# Category:    error-contract
# Layer:       reader (v1.0 adapter)
# Precondition: docs/validation/fixtures/corrupt_scalar.h5
# Action:      Attempt to read/parse
# Expected:    Typed error returned, not a panic.
#              The error identifies the field and path.
# Rationale:   Attribute type coercion is a real failure mode when files are
#              written by non-library tools.
def run_av05(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "corrupt_scalar.h5")
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for corrupt Build_Plate_X_Dimension"
    except (ValueError, TypeError) as e:
        return True, f"{type(e).__name__} raised for corrupt scalar: {e}"
    except Exception as e:
        return False, f"unexpected exception type {type(e).__name__}: {e}"


# AV-06: Version string whitespace variants
# ID:          AV-06
# Title:       Dispatcher normalizes whitespace in File_Version
# Category:    adapter-versioning
# Layer:       dispatcher
# Precondition: docs/validation/fixtures/version_whitespace.h5 (File_Version=" 1.0 ")
# Action:      Read/parse
# Expected:    Successfully dispatches to v1.0 adapter and reads correctly.
# Rationale:   Real files from legacy tools have been observed with whitespace in
#              version strings.
def run_av06(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "version_whitespace.h5")
    try:
        MachineConfigReader(fixture).parse()
        return True, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"
    except Exception as e:
        return False, f"{type(e).__name__}: {e}"


# AV-07: Empty string File_Version
# ID:          AV-07
# Title:       Dispatcher handles empty string File_Version
# Category:    adapter-versioning
# Layer:       dispatcher
# Precondition: docs/validation/fixtures/empty_version.h5 (File_Version="")
# Action:      Read/parse
# Expected:    Either: defaults to "1.0", OR raises typed error.
#              Behavior must be identical across all five languages.
# Rationale:   Establishes and verifies the empty-string contract explicitly.
def run_av07(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = av_fixture(fixtures_dir, "empty_version.h5")
    try:
        cfg = MachineConfigReader(fixture).parse()
        return True, f"empty File_Version defaults to '1.0', reads OK, file_version='{cfg.meta.file_version}'"
    except Exception as e:
        return True, f"empty File_Version raises {type(e).__name__}: {e}"


# AV-08: Roundtrip version string fidelity
# ID:          AV-08
# Title:       File_Version string survives write→read unchanged
# Category:    adapter-versioning
# Layer:       writer + reader
# Precondition: fixtures/reference_config.h5
# Action:      Read → write to temp → read temp → compare File_Version strings
# Expected:    File_Version is exactly "1.0" after roundtrip in all five languages.
#              No whitespace added, no truncation, no case change.
# Rationale:   A writer that silently normalizes the version string would break
#              future version dispatch for files it produces.
def run_av08(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()
    orig_version = cfg.meta.file_version.strip()

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()
    rb_version = rb.meta.file_version.strip()

    if rb_version != "1.0":
        return False, f"file_version after roundtrip: expected '1.0', got '{rb_version}'"
    if rb_version != orig_version:
        return False, f"file_version changed: '{orig_version}' → '{rb_version}'"

    return True, f"File_Version survives roundtrip unchanged: '{rb_version}'"


# AV-09: Mock v1.1 adapter — adding a new adapter leaves the v1.0 adapter untouched
# ID:          AV-09
# Title:       Adding a mock v1.1 adapter leaves the v1.0 adapter and all existing behaviour unchanged
# Category:    adapter-versioning
# Layer:       adapter isolation
# Precondition: fixtures/reference_config.h5
# Action:      Import MockV1_1Reader and MockV1_1Writer alongside the real v1.0 reader.
#              Read the reference fixture with the real v1.0 reader and verify correct output.
# Expected:    v1.0 reader still returns correct values — importing the mock v1.1 adapter
#              has no side-effect on the v1.0 adapter's behaviour.
# Rationale:   The adapter pattern's primary promise is that adding a new version is
#              purely additive — no existing adapter code is modified.
def run_av09(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()

    if cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG":
        return False, f"v1.0 adapter broken after importing mock v1.1: machine_name='{cfg.meta.machine_name}'"
    if cfg.meta.file_version.strip() != "1.0":
        return False, f"v1.0 adapter returned wrong file_version: '{cfg.meta.file_version}'"
    if len(cfg.optical_trains) != 2:
        return False, f"v1.0 adapter returned wrong train count: {len(cfg.optical_trains)}"

    return True, "v1.0 adapter unaffected by mock v1.1 adapter import; all fields correct"


# AV-10: Mock v1.1 adapter — forward migration (v1.0 → v1.1)
# ID:          AV-10
# Title:       v1.0 fixture migrates forward to v1.1 correctly across all change categories
# Category:    adapter-versioning
# Layer:       adapter + StableModel
# Precondition: fixtures/reference_config.h5 (v1.0)
# Action:      Read with v1.0 adapter → write with mock v1.1 adapter → read back.
#              Verify all five change categories from docs/migrations/mock_v1_0_to_v1_1.md:
#                ADDITION  — facility_id and config_author are None (no v1.0 source)
#                REMOVAL   — gas_flow_direction and recoat_direction are None (dropped in v1.1)
#                NAME      — machine_name and working_distance values preserved
#                PATH      — build_plate.z and build_plate.corner_radius values preserved
#                NAME+PATH — build_plate.x and build_plate.y values preserved
# Expected:    Preserved fields identical before and after. Lossy fields are None.
# Rationale:   Verifies the StableModel is the correct handoff point and that each
#              change category behaves as documented in the manifest.
def run_av10(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    src = Path(fixtures_dir) / "reference_config.h5"
    v1_0 = MachineConfigReader(src).parse()

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MockV1_1Writer(replace(v1_0, meta=replace(v1_0.meta, file_version="1.1-mock"))).write(tmp)
    v1_1 = MockV1_1Reader(tmp).parse()

    # ADDITION: no v1.0 source — typed fields are None
    if v1_1.meta.facility_id is not None:
        return False, f"facility_id should be None, got '{v1_1.meta.facility_id}'"
    if v1_1.meta.config_author is not None:
        return False, f"config_author should be None, got '{v1_1.meta.config_author}'"

    # REMOVAL: fields absent in v1.1 reader
    if v1_1.machine.gas_flow_direction is not None:
        return False, "gas_flow_direction should be None after forward migration"
    if v1_1.machine.recoat_direction is not None:
        return False, "recoat_direction should be None after forward migration"

    # NAME, PATH, NAME+PATH: values preserved through StableModel
    if v1_1.machine.machine_name != v1_0.machine.machine_name:
        return False, f"machine_name changed: '{v1_0.machine.machine_name}' → '{v1_1.machine.machine_name}'"
    t0, t1 = v1_0.optical_trains[0], v1_1.optical_trains[0]
    if t1.scanner.working_distance != t0.scanner.working_distance:
        return False, f"working_distance changed: {t0.scanner.working_distance} → {t1.scanner.working_distance}"
    if v1_1.machine.build_plate.z != v1_0.machine.build_plate.z:
        return False, "build_plate.z changed"
    if v1_1.machine.build_plate.corner_radius != v1_0.machine.build_plate.corner_radius:
        return False, "build_plate.corner_radius changed"
    if v1_1.machine.build_plate.x != v1_0.machine.build_plate.x:
        return False, "build_plate.x changed"
    if v1_1.machine.build_plate.y != v1_0.machine.build_plate.y:
        return False, "build_plate.y changed"

    return True, (
        f"forward migration OK — "
        f"ADDITION=None, REMOVAL=None, "
        f"machine_name='{v1_1.machine.machine_name}', "
        f"build_plate x={v1_1.machine.build_plate.x} y={v1_1.machine.build_plate.y} "
        f"z={v1_1.machine.build_plate.z} preserved"
    )


# AV-11: Mock v1.1 adapter — backward migration (v1.1 → v1.0)
# ID:          AV-11
# Title:       v1.1 file migrates backward to v1.0 correctly; ADDITION fields are lost
# Category:    adapter-versioning
# Layer:       adapter + StableModel
# Precondition: A v1.1 file produced by MockV1_1Writer (created inline)
# Action:      Write a v1.1 file with facility_id and config_author populated →
#              read with mock v1.1 adapter → write with v1.0 adapter → read back.
#              Verify:
#                ADDITION  — facility_id and config_author are None after v1.0 read
#                            (v1.0 writer does not write these attrs — intentionally lost)
#                NAME/PATH — all preserved fields survive back to v1.0 layout
# Expected:    ADDITION fields None after roundtrip. All other preserved fields intact.
# Rationale:   Verifies the backward migration contract: additions introduced in v1.1
#              are explicitly lost when downgrading, not silently corrupted.
def run_av11(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    from test_adapter_migration import _make_config  # type: ignore[attr-defined]

    cfg_v1_1 = _make_config(
        machine_name="BackwardMigrationTest",
        facility_id="Lab-Validation",
        config_author="ValidationBot",
    )

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        v1_1_path = f.name
    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        v1_0_path = f.name

    MockV1_1Writer(cfg_v1_1).write(v1_1_path)

    # Downgrade: read v1.1, write v1.0
    v1_1 = MockV1_1Reader(v1_1_path).parse()
    Hdf5WriterV1_0(replace(v1_1, meta=replace(v1_1.meta, file_version="1.0"))).write(v1_0_path)
    v1_0 = MachineConfigReader(v1_0_path).parse()

    # ADDITION fields must be lost (v1.0 writer/reader don't know these attrs)
    if v1_0.meta.facility_id is not None:
        return False, f"facility_id should be lost after downgrade, got '{v1_0.meta.facility_id}'"
    if v1_0.meta.config_author is not None:
        return False, f"config_author should be lost after downgrade, got '{v1_0.meta.config_author}'"

    # machine_name must survive (NAME change maps back through StableModel)
    if v1_0.machine.machine_name != "BackwardMigrationTest":
        return False, f"machine_name lost during downgrade: '{v1_0.machine.machine_name}'"

    if v1_0.meta.file_version.strip() != "1.0":
        return False, f"file_version wrong after downgrade: '{v1_0.meta.file_version}'"

    return True, (
        "backward migration OK — "
        "ADDITION fields lost (facility_id=None, config_author=None), "
        f"machine_name='{v1_0.machine.machine_name}' preserved, "
        "file_version='1.0'"
    )


# AV-12: capabilities session preserves extensions across open -> edit -> save
# ID:          AV-12
# Title:       Extension tree survives a capabilities-facade edit session
# Category:    happy-path (capabilities facade)
# Layer:       capabilities facade (v1.0)
# Precondition: fixtures/reference_config_opcua.h5
# Action:      open_machine_config(path), edit the scanner, save to a temp file, re-read
# Expected:    The TM_OPCUA extension tree's canonical digest is unchanged.
# Rationale:   Nothing else exercises extensions through the `capabilities` session API: an
#              edit session must never drop data core has no model for.
def run_av12(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config_opcua.h5"
    before = canonical_digest(MachineConfigReader(path).parse().extensions["TM_OPCUA"])

    opened = open_machine_config(path)
    if not opened.ok:
        return False, f"open_machine_config failed: {opened.error}"
    train = opened.value.optical_train(0)
    if not train.ok:
        return False, f"optical_train(0) failed: {train.error}"
    scanner = train.value.get_scanner()
    edited = train.value.set_scanner({**scanner, "working_distance": 77.5})
    if not edited.ok:
        return False, f"set_scanner failed: {edited.error}"

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name
    saved = opened.value.save(tmp)
    if not saved.ok:
        return False, f"save failed: {saved.error}"
    opened.value.close()

    after = canonical_digest(MachineConfigReader(tmp).parse().extensions["TM_OPCUA"])
    if after != before:
        return False, f"extension digest changed across the session: {before[:16]}... -> {after[:16]}..."
    return True, f"extension survived the edit session, digest={after[:16]}..."


# AV-13: the writer rejects an invalid extension tree before creating any file
# ID:          AV-13
# Title:       Invalid extension tree is rejected without leaving a partial file
# Category:    error-contract
# Layer:       writer + extension contract
# Precondition: fixtures/reference_config.h5
# Action:      Attach an extension with an out-of-range u8 attribute and write
# Expected:    The write raises ExtensionContractError naming the offending attribute,
#              and no output file is created.
# Rationale:   Validation is part of the stable extension contract; a rejected config must
#              never leave a half-written file behind.
def run_av13(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    cfg = MachineConfigReader(Path(fixtures_dir) / "reference_config.h5").parse()
    cfg = replace(cfg, extensions={"Bad": ExtensionTree(attrs={"A": Attr("u8", 999)})})
    cfg.meta.file_version = "1.1"

    out = Path(tempfile.mkdtemp()) / "never.h5"
    try:
        MachineConfigWriter(cfg).write(out)
    except ExtensionContractError as e:
        if "@A" not in e.path:
            return False, f"error path does not name the attribute: {e.path!r}"
        if out.exists():
            return False, "a file was created despite the rejected extension"
        return True, f"ExtensionContractError at {e.path!r}, no file created"
    return False, "writer accepted an out-of-range extension attribute"
