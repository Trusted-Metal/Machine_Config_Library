import sys
import tempfile
from dataclasses import replace as dc_replace
from pathlib import Path
from machine_config import MachineConfigReader
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0

# mock adapter lives in the test suite, not the installed wheel
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent.parent.parent / "python" / "tests"))
from test_adapter_migration import MockV1_1Reader, MockV1_1Writer  # noqa: E402

"""
AV-11: Mock v1.1 adapter — backward migration (v1.1 → v1.0)
ID:          AV-11
Title:       v1.1 file migrates backward to v1.0 correctly; ADDITION fields are lost
Category:    adapter-versioning
Layer:       adapter + StableModel
Precondition: A v1.1 file produced by MockV1_1Writer (created inline)
Action:      Write a v1.1 file with facility_id and config_author populated →
             read with mock v1.1 adapter → write with v1.0 adapter → read back.
             Verify:
               ADDITION  — facility_id and config_author are None after v1.0 read
                           (v1.0 writer does not write these attrs — intentionally lost)
               NAME/PATH — all preserved fields survive back to v1.0 layout
Expected:    ADDITION fields None after roundtrip. All other preserved fields intact.
Rationale:   Verifies the backward migration contract: additions introduced in v1.1
             are explicitly lost when downgrading, not silently corrupted.
"""

_SCHEMA_VERSION = "https://github.com/AconityLabs/machine-config-schema/v1"

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    # Build a v1.1 file with ADDITION fields populated
    from machine_config.models import MachineConfig, MachineConfigMeta
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
    Hdf5WriterV1_0(dc_replace(v1_1, meta=dc_replace(v1_1.meta, file_version="1.0"))).write(v1_0_path)
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
