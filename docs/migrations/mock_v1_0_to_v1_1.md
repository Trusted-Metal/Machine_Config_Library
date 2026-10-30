# Migration Manifest: v1.0 → mock v1.1

> **This manifest documents the mock v1.1 layout used in `python/tests/test_adapter_migration.py`.**
> The changes are synthetic and exist solely to verify the adapter migration architecture.
> They do not represent a planned schema change.
>
> The real v1.1 has since shipped — see [`v1_0_to_v1_1.md`](v1_0_to_v1_1.md) in this
> folder, following the same format.

---

## Overview

| | Value |
|---|---|
| Source version | `1.0` |
| Target version | `1.1-mock` (test artifact only — the `-mock` suffix is deliberate so this can never be mistaken for, or collide with, a real `File_Version`) |
| StableModel changes | **None** — all changes are HDF5 layout only |
| Layout constants | `python/tests/test_adapter_migration.py` → `MockV1_1Layout` |
| Reader | `python/tests/test_adapter_migration.py` → `MockV1_1Reader` |
| Writer | `python/tests/test_adapter_migration.py` → `MockV1_1Writer` |

---

## Change Manifest

| # | Category | Field | v1.0 Group | v1.0 Key | v1.1 Group | v1.1 Key | StableModel Field | Lossy |
|---|---|---|---|---|---|---|---|---|
| 1 | Addition | Facility ID | — | — | root | `Facility_ID` | `meta.facility_id` (`Optional[str]`) | No |
| 2 | Addition | Config Author | — | — | root | `Config_Author` | `meta.config_author` (`Optional[str]`) | No |
| 3 | Removal | Gas Flow Direction | `Machine/` | `Gas_Flow_Direction` | — | — | `machine.gas_flow_direction → None` | **Yes** |
| 4 | Removal | Recoat Direction | `Machine/` | `Recoat_Direction` | — | — | `machine.recoat_direction → None` | **Yes** |
| 5 | Name | Machine Name | `Machine/` | `Machine_Name` | `Machine/` | `Machine_Label` | `machine.machine_name` | No |
| 6 | Name | Working Distance | `…/Scanner/` | `Working_Distance` | `…/Scanner/` | `Focal_Distance` | `scanner.working_distance` | No |
| 7 | Path | Build Plate Z | `Machine/` | `Build_Plate_Z_Dimension` | `Machine/Dimensions/` | `Build_Plate_Z_Dimension` | `build_plate.z` | No |
| 8 | Path | Corner Radius | `Machine/` | `Build_Plate_Corner_Radius` | `Machine/Dimensions/` | `Build_Plate_Corner_Radius` | `build_plate.corner_radius` | No |
| 9 | Name+Path | Build Plate X | `Machine/` | `Build_Plate_X_Dimension` | `Machine/Dimensions/` | `Width` | `build_plate.x` | No |
| 10 | Name+Path | Build Plate Y | `Machine/` | `Build_Plate_Y_Dimension` | `Machine/Dimensions/` | `Height` | `build_plate.y` | No |

### Unit attributes (unchanged)

These attrs remain in `Machine/` in both versions with the same keys:

`Build_Plate_X_Dimension_unit`, `Build_Plate_Y_Dimension_unit`,
`Build_Plate_Z_Dimension_unit`, `Build_Plate_Corner_Radius_unit`

---

## Migration behaviour

### v1.0 → v1.1 (forward)

| Change | Behaviour |
|---|---|
| Addition fields | `meta.facility_id` and `meta.config_author` are `None` (no v1.0 source); the v1.1 writer still writes the HDF5 attrs as empty strings so the file is schema-complete |
| Removal fields | Dropped — `gas_flow_direction` and `recoat_direction` are **lost** |
| Name/Path/Name+Path fields | Value preserved through StableModel intermediary |

| Change | Behaviour |
|---|---|
| Addition fields | **Lost** — `meta.facility_id` / `meta.config_author` are `None` after v1.0 read-back (v1.0 writer/reader don't know these fields) |
| Removal fields | Remain `None` — v1.0 writer writes `""`, v1.0 reader reads it back as `None` |
| Name/Path/Name+Path fields | Value preserved through StableModel intermediary |

---

## Behavioral verification

All 8 tests pass as part of the standard suite (`python -m pytest python/tests/`):

| Test | What it proves |
|---|---|
| `test_v1_1_read` | All 5 change categories read correctly from a mock v1.1 file |
| `test_v1_1_roundtrip` | All categories survive write → read → write → read |
| `test_v1_to_v1_1` | Real v1.0 fixture migrates forward correctly |
| `test_v1_1_to_v1` | Mock v1.1 migrates backward correctly; ADDITION fields are lost (v1.0 writer doesn't write them) |
| `test_v1_unaffected` | Existing v1.0 adapter path is untouched |
| `test_dispatcher_v1_to_v1_1` | Full public API (`MachineConfigReader`/`Writer`) for v1.1 read/write |
| `test_dispatcher_v1_1_to_v1` | Full public API for v1.0 → v1.1 via monkeypatched dispatcher |
| `test_mock_adapters_satisfy_protocol` | `MockV1_1Reader` and `MockV1_1Writer` satisfy `ReaderAdapter`/`WriterAdapter` |

---

## Visual inspection

```powershell
# From repo root, venv active
.\.venv\Scripts\python.exe scratch/inspect_migration.py
```

Produces `scratch/migrated_mock_v1_1.h5`. Open alongside `fixtures/reference_config.h5`
in an HDF5 viewer to visually confirm the structural differences in the table above.
