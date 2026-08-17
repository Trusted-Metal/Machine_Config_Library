import sys
import tempfile
from dataclasses import replace as dc_replace
from pathlib import Path
from machine_config import MachineConfigReader

# mock adapter lives in the test suite, not the installed wheel
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent.parent.parent / "python" / "tests"))
from test_adapter_migration import MockV1_1Reader, MockV1_1Writer  # noqa: E402

"""
AV-10: Mock v1.1 adapter — forward migration (v1.0 → v1.1)
ID:          AV-10
Title:       v1.0 fixture migrates forward to v1.1 correctly across all change categories
Category:    adapter-versioning
Layer:       adapter + StableModel
Precondition: fixtures/reference_config.h5 (v1.0)
Action:      Read with v1.0 adapter → write with mock v1.1 adapter → read back.
             Verify all five change categories from docs/migrations/mock_v1_0_to_v1_1.md:
               ADDITION  — facility_id and config_author are None (no v1.0 source)
               REMOVAL   — gas_flow_direction and recoat_direction are None (dropped in v1.1)
               NAME      — machine_name and working_distance values preserved
               PATH      — build_plate.z and build_plate.corner_radius values preserved
               NAME+PATH — build_plate.x and build_plate.y values preserved
Expected:    Preserved fields identical before and after. Lossy fields are None.
Rationale:   Verifies the StableModel is the correct handoff point and that each
             change category behaves as documented in the manifest.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    src = Path(fixtures_dir) / "reference_config.h5"
    v1_0 = MachineConfigReader(src).parse()

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MockV1_1Writer(dc_replace(v1_0, meta=dc_replace(v1_0.meta, file_version="1.1"))).write(tmp)
    v1_1 = MockV1_1Reader(tmp).parse()

    # ADDITION: no v1.0 source — typed fields are None
    if v1_1.meta.facility_id is not None:
        return False, f"facility_id should be None, got '{v1_1.meta.facility_id}'"
    if v1_1.meta.config_author is not None:
        return False, f"config_author should be None, got '{v1_1.meta.config_author}'"

    # REMOVAL: fields absent in v1.1 reader
    if v1_1.machine.gas_flow_direction is not None:
        return False, f"gas_flow_direction should be None after forward migration"
    if v1_1.machine.recoat_direction is not None:
        return False, f"recoat_direction should be None after forward migration"

    # NAME, PATH, NAME+PATH: values preserved through StableModel
    if v1_1.machine.machine_name != v1_0.machine.machine_name:
        return False, f"machine_name changed: '{v1_0.machine.machine_name}' → '{v1_1.machine.machine_name}'"
    t0, t1 = v1_0.optical_trains[0], v1_1.optical_trains[0]
    if t1.scanner.working_distance != t0.scanner.working_distance:
        return False, f"working_distance changed: {t0.scanner.working_distance} → {t1.scanner.working_distance}"
    if v1_1.machine.build_plate.z != v1_0.machine.build_plate.z:
        return False, f"build_plate.z changed"
    if v1_1.machine.build_plate.corner_radius != v1_0.machine.build_plate.corner_radius:
        return False, f"build_plate.corner_radius changed"
    if v1_1.machine.build_plate.x != v1_0.machine.build_plate.x:
        return False, f"build_plate.x changed"
    if v1_1.machine.build_plate.y != v1_0.machine.build_plate.y:
        return False, f"build_plate.y changed"

    return True, (
        f"forward migration OK — "
        f"ADDITION=None, REMOVAL=None, "
        f"machine_name='{v1_1.machine.machine_name}', "
        f"build_plate x={v1_1.machine.build_plate.x} y={v1_1.machine.build_plate.y} "
        f"z={v1_1.machine.build_plate.z} preserved"
    )
