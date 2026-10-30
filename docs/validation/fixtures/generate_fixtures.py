"""Generate hand-crafted AV test fixtures from reference_config.h5.

Usage:
    python docs/validation/fixtures/generate_fixtures.py \
        --source fixtures/reference_config.h5 \
        --outdir docs/validation/fixtures/
"""
import argparse
import shutil
from pathlib import Path

import h5py


def _copy(src: Path, dst: Path) -> None:
    shutil.copy2(src, dst)


def generate(source: Path, outdir: Path) -> None:
    outdir.mkdir(parents=True, exist_ok=True)

    # AV-01: unknown version
    dst = outdir / "v2_0_unknown.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        f.attrs["File_Version"] = "2.0"

    # AV-02: missing version attribute
    dst = outdir / "missing_version.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        if "File_Version" in f.attrs:
            del f.attrs["File_Version"]

    # AV-03: simulated future version with extra group
    dst = outdir / "v1_1_simulated.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        f.attrs["File_Version"] = "1.1"
        f.require_group("Future_Group")

    # AV-04: required Machine/ group deleted
    dst = outdir / "missing_machine_group.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        del f["Machine"]

    # AV-05: corrupt required scalar attribute
    dst = outdir / "corrupt_scalar.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        f["Machine"].attrs["Build_Plate_X_Dimension"] = "not_a_number"

    # AV-06: whitespace around version string
    dst = outdir / "version_whitespace.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        f.attrs["File_Version"] = " 1.0 "

    # AV-07: empty version string
    dst = outdir / "empty_version.h5"
    _copy(source, dst)
    with h5py.File(dst, "a") as f:
        f.attrs["File_Version"] = ""

    for p in sorted(outdir.glob("*.h5")):
        print(f"OK  {p.name}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--outdir", required=True, type=Path)
    args = parser.parse_args()
    generate(args.source, args.outdir)
