import sys
from scenarios import (
    s01_read_scalars, s02_read_binary, s03_read_real,
    s04_write_modify, s05_binary_roundtrip, s06_builder,
    s07_opcua, s08_drastic_change, s09_type_exports,
    av01_unknown_version, av02_missing_version, av03_future_version,
    av04_missing_group, av05_corrupt_scalar, av06_whitespace_version,
    av07_empty_version, av08_version_fidelity,
    av09_mock_adapter_isolation, av10_forward_migration, av11_backward_migration,
)

SCENARIOS = [
    ("S-01",  s01_read_scalars.run),
    ("S-02",  s02_read_binary.run),
    ("S-03",  s03_read_real.run),
    ("S-04",  s04_write_modify.run),
    ("S-05",  s05_binary_roundtrip.run),
    ("S-06",  s06_builder.run),
    ("S-07",  s07_opcua.run),
    ("S-08",  s08_drastic_change.run),
    ("S-09",  s09_type_exports.run),
    ("AV-01", av01_unknown_version.run),
    ("AV-02", av02_missing_version.run),
    ("AV-03", av03_future_version.run),
    ("AV-04", av04_missing_group.run),
    ("AV-05", av05_corrupt_scalar.run),
    ("AV-06", av06_whitespace_version.run),
    ("AV-07", av07_empty_version.run),
    ("AV-08", av08_version_fidelity.run),
    ("AV-09", av09_mock_adapter_isolation.run),
    ("AV-10", av10_forward_migration.run),
    ("AV-11", av11_backward_migration.run),
]


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: python main.py <fixtures_dir> <real_dir>", file=sys.stderr)
        sys.exit(1)

    fixtures_dir = sys.argv[1]
    real_dir = sys.argv[2]
    passed = failed = 0

    for sid, run_fn in SCENARIOS:
        try:
            ok, msg = run_fn(fixtures_dir, real_dir)
        except Exception as e:
            ok, msg = False, f"EXCEPTION: {type(e).__name__}: {e}"
        print(f"{'[PASS]' if ok else '[FAIL]'} {sid}: {msg}")
        if ok:
            passed += 1
        else:
            failed += 1

    print(f"\n{passed + failed} scenarios: {passed} passed, {failed} failed")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
