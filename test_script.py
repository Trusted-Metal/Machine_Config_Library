import json
import pathlib

import h5py
import numpy as np

from machine_config import MachineConfigReader, MachineConfigWriter, config_from_dict

DOWNLOADS = pathlib.Path(r"C:\Users\ChrisParham\Downloads")

# ---------------------------------------------------------------------------
# H5 structural diff
# ---------------------------------------------------------------------------

# Datasets whose raw bytes the writer intentionally does not reproduce.
# (Empty — scan_field_correction_file bytes are now fully preserved.)
_SKIP_DATASET_VALUES: set[str] = set()

# String attributes where the reader intentionally strips leading/trailing
# whitespace (normalisation of data-entry artifacts in the source HDF5).
# Values are compared after stripping on both sides.
_STRIP_STRING_ATTRS = {"Serial_Number"}


def _native(v):
    """Convert numpy scalar → Python native for value comparison."""
    if isinstance(v, np.integer):
        return int(v)
    if isinstance(v, np.floating):
        return float(v)
    if isinstance(v, bytes):
        return v.decode()
    return v


def diff_h5(orig_path, rt_path, label: str) -> list[str]:
    """
    Recursively compare two HDF5 files.

    Returns a list of difference strings (empty = identical within model scope).
    Intentionally skips:
      - HDF5 dtype differences (int32 vs int64 — value-equivalent is fine)
      - Raw bytes of datasets in _SKIP_DATASET_VALUES
    """
    diffs = []

    def visit(orig_obj, rt_obj, path: str):
        # ---- attributes -------------------------------------------------------
        orig_keys = set(orig_obj.attrs.keys())
        rt_keys   = set(rt_obj.attrs.keys())

        for k in sorted(orig_keys - rt_keys):
            diffs.append(f"  MISSING attr  {path}::{k}  (in original, absent in roundtrip)")
        for k in sorted(rt_keys - orig_keys):
            diffs.append(f"  EXTRA   attr  {path}::{k}  (absent in original, added in roundtrip)")
        for k in sorted(orig_keys & rt_keys):
            ov = _native(orig_obj.attrs[k])
            rv = _native(rt_obj.attrs[k])
            # For known string attrs the reader normalises by stripping whitespace.
            if k in _STRIP_STRING_ATTRS and isinstance(ov, str) and isinstance(rv, str):
                ov, rv = ov.strip(), rv.strip()
            if ov != rv:
                diffs.append(f"  VALUE   attr  {path}::{k}  original={ov!r}  roundtrip={rv!r}")

        # ---- subgroups --------------------------------------------------------
        if isinstance(orig_obj, (h5py.File, h5py.Group)):
            orig_groups   = {k for k, v in orig_obj.items() if isinstance(v, h5py.Group)}
            rt_groups     = {k for k, v in rt_obj.items()   if isinstance(v, h5py.Group)}
            orig_datasets = {k for k, v in orig_obj.items() if isinstance(v, h5py.Dataset)}
            rt_datasets   = {k for k, v in rt_obj.items()   if isinstance(v, h5py.Dataset)}

            for k in sorted(orig_groups - rt_groups):
                diffs.append(f"  MISSING group  {path}/{k}")
            for k in sorted(rt_groups - orig_groups):
                diffs.append(f"  EXTRA   group  {path}/{k}")
            for k in sorted(orig_groups & rt_groups):
                visit(orig_obj[k], rt_obj[k], f"{path}/{k}")

            for k in sorted(orig_datasets - rt_datasets):
                diffs.append(f"  MISSING dataset  {path}/{k}")
            for k in sorted(rt_datasets - orig_datasets):
                diffs.append(f"  EXTRA   dataset  {path}/{k}")
            for k in sorted(orig_datasets & rt_datasets):
                visit(orig_obj[k], rt_obj[k], f"{path}/{k}")

        # ---- dataset values ---------------------------------------------------
        elif isinstance(orig_obj, h5py.Dataset):
            ds_name = path.split("/")[-1]
            if ds_name in _SKIP_DATASET_VALUES:
                return  # known placeholder — skip value comparison
            oa = orig_obj[()]
            ra = rt_obj[()]
            if oa.shape != ra.shape:
                diffs.append(f"  SHAPE  dataset  {path}  original={oa.shape}  roundtrip={ra.shape}")
            else:
                # NaN-aware comparison
                both_nan  = np.isnan(oa) & np.isnan(ra)
                mismatch  = ~both_nan & (oa != ra)
                n = int(mismatch.sum())
                if n:
                    diffs.append(f"  VALUES dataset  {path}  {n} element(s) differ")

    with h5py.File(orig_path, "r") as fo, h5py.File(rt_path, "r") as fr:
        visit(fo, fr, label)

    return diffs


# ---------------------------------------------------------------------------
# Roundtrip helper
# ---------------------------------------------------------------------------

def roundtrip(src_h5: str, label: str) -> None:
    json_path = DOWNLOADS / f"{label}.json"
    rt_h5_path = DOWNLOADS / f"{label}_roundtrip.h5"

    # JSON roundtrip (canonical model check)
    original_reader = MachineConfigReader(src_h5)
    original_json   = original_reader.to_json(indent=2)
    json_path.write_text(original_json, encoding="utf-8")

    config = config_from_dict(json.loads(original_json))
    MachineConfigWriter(config).write(rt_h5_path)

    rt_json = MachineConfigReader(rt_h5_path).to_json(indent=2)

    if original_json == rt_json:
        print(f"[{label}] JSON  ✓  canonical model matches.")
    else:
        orig_lines = original_json.splitlines()
        rt_lines   = rt_json.splitlines()
        for i, (a, b) in enumerate(zip(orig_lines, rt_lines), 1):
            if a != b:
                print(f"[{label}] JSON  ✗  first drift at line {i}:")
                print(f"  original:   {a!r}")
                print(f"  roundtrip:  {b!r}")
                break
        raise AssertionError(f"[{label}] JSON roundtrip produced different output.")

    # HDF5 structural diff
    diffs = diff_h5(src_h5, rt_h5_path, label)
    if not diffs:
        print(f"[{label}] HDF5  ✓  all groups, attrs, and datasets match.\n")
    else:
        print(f"[{label}] HDF5  — {len(diffs)} difference(s):")
        for d in diffs:
            print(d)
        print()


# ---------------------------------------------------------------------------
# Run both fixtures
# ---------------------------------------------------------------------------

roundtrip("fixtures/reference_config.h5",       "reference_config")
roundtrip("fixtures/reference_config_opcua.h5",  "reference_config_opcua")


