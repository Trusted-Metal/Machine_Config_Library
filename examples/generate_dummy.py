"""Generate examples/dummy_2train.h5 via MockConfigBuilder (2 optical trains).

Run from the repo root:

    python examples/generate_dummy.py
"""
from __future__ import annotations

from pathlib import Path

from machine_config import MockConfigBuilder

REPO_ROOT = Path(__file__).resolve().parents[1]
OUT = Path(__file__).resolve().parent / "dummy_2train.h5"


def main() -> None:
    builder = MockConfigBuilder(
        n_lasers=2,
        include_clearbox=True,
        file_version="1.0",
        machine_name="ExampleDummy-2Train",
        manufacturer="ExampleCo",
        model="DummyMIDI+",
        serial_number="EX-DUMMY-001",
    )
    builder.save(OUT)
    print(f"Wrote {OUT.relative_to(REPO_ROOT)}  ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
