"""Generate the Phase 1.7 golden fixtures.

Produces three files in fixtures/:
  reference_output.json   — canonical JSON from parsing the reference HDF5
  reference_output.sha256 — SHA-256 checksum of the JSON (enforced in CI)
  synthetic_2laser.h5     — small synthetic fixture for cross-language tests

Run from repo root:
    .venv/Scripts/python.exe tools/generate_fixtures.py    (Git Bash)
    .venv\\Scripts\\python.exe tools/generate_fixtures.py  (PowerShell)

REVIEW REQUIRED: compare reference_output.json against the checklist in
IMPLEMENTATION_PLAN.md section 1.7 before committing.
"""
from __future__ import annotations

import hashlib
import pathlib
import sys

REPO_ROOT = pathlib.Path(__file__).parent.parent
FIXTURES  = REPO_ROOT / "fixtures"

# Ensure the installed package is importable when running without activation.
sys.path.insert(0, str(REPO_ROOT / "python" / "src"))

from machine_config import MachineConfigReader  # noqa: E402
from machine_config.builder import MockConfigBuilder  # noqa: E402


def _separator(title: str) -> None:
    print(f"\n-- {title} " + "-" * max(0, 58 - len(title)))


def generate_reference_output() -> str:
    """Parse reference_config.h5 and write reference_output.json + .sha256."""
    h5_path = FIXTURES / "reference_config.h5"
    if not h5_path.exists():
        sys.exit(f"ERROR: {h5_path} not found. Cannot generate golden file.")

    _separator("Generating reference_output.json")
    reader = MachineConfigReader(h5_path)
    json_text = reader.to_json(indent=2)

    json_path = FIXTURES / "reference_output.json"
    json_path.write_text(json_text, encoding="utf-8")
    print(f"  Written: {json_path.relative_to(REPO_ROOT)}")

    digest = hashlib.sha256(json_text.encode("utf-8")).hexdigest()
    sha_path = FIXTURES / "reference_output.sha256"
    sha_path.write_text(digest + "\n", encoding="utf-8")
    print(f"  Written: {sha_path.relative_to(REPO_ROOT)}")
    print(f"  SHA-256: {digest}")

    return digest


def generate_synthetic_fixture() -> None:
    """Build a small synthetic 2-laser HDF5 for cross-language tests."""
    out_path = FIXTURES / "synthetic_2laser.h5"

    _separator("Generating synthetic_2laser.h5")
    MockConfigBuilder(
        n_lasers=2,
        machine_name="SyntheticMachine",
        manufacturer="TestCo",
        model="Synth2L",
        serial_number="SYNTH-002",
        build_plate_x=250.0,
        build_plate_y=250.0,
        build_plate_z=20.0,
        include_clearbox=True,
        file_version="1.0",
    ).save(out_path)
    print(f"  Written: {out_path.relative_to(REPO_ROOT)}")


def print_review_checklist() -> None:
    _separator("HUMAN REVIEW REQUIRED before committing")
    checklist = [
        "meta.machine_name  == 'TM-LPBF-02: AconityMIDI+_OG' (or matching root attr)",
        "meta.configuration_hash  is exactly 64 hex chars",
        "machine.build_plate_x = 250.0  build_plate_y = 250.0  build_plate_z = 20.0",
        "Two optical trains present",
        "Train 01: working_distance=670.0  offset_x=-87.5  offset_y=23.5  rotation=0.0",
        "Train 02: offset_x=86.074  offset_y=-21.695  rotation=180.0",
        "Train 01 thermal_lensing_passed = false",
        "Train 02 thermal_lensing_passed = true",
        "Both trains: clearbox present with scalar fields (no correction_data arrays — use include_binary=True to inspect)",
        "Train 01 scan_field_correction_file.file_size = 1138799  (no raw_bytes — use include_binary=True)",
        "Train 02 scan_field_correction_file.file_size = 1142763  (no raw_bytes — use include_binary=True)",
        "No OPCUA fields appear anywhere in the output",
    ]
    for item in checklist:
        print(f"  [ ] {item}")

    print()
    print("  Commit BOTH files together:")
    print("    fixtures/reference_output.json")
    print("    fixtures/reference_output.sha256")
    print()
    print("  Then re-run the test suite — all tests should pass.")


if __name__ == "__main__":
    generate_reference_output()
    generate_synthetic_fixture()
    print_review_checklist()
    print()
