import sys
from scenarios import (
    run_s01, run_s02, run_s03, run_s04, run_s05, run_s06, run_s07, run_s08, run_s09, run_s10,
    run_av01, run_av02, run_av03, run_av04, run_av05, run_av06, run_av07, run_av08,
    run_av09, run_av10, run_av11, run_av12, run_av13,
)

SCENARIOS = [
    ("S-01",  run_s01),
    ("S-02",  run_s02),
    ("S-03",  run_s03),
    ("S-04",  run_s04),
    ("S-05",  run_s05),
    ("S-06",  run_s06),
    ("S-07",  run_s07),
    ("S-08",  run_s08),
    ("S-09",  run_s09),
    ("S-10",  run_s10),
    ("AV-01", run_av01),
    ("AV-02", run_av02),
    ("AV-03", run_av03),
    ("AV-04", run_av04),
    ("AV-05", run_av05),
    ("AV-06", run_av06),
    ("AV-07", run_av07),
    ("AV-08", run_av08),
    ("AV-09", run_av09),
    ("AV-10", run_av10),
    ("AV-11", run_av11),
    ("AV-12", run_av12),
    ("AV-13", run_av13),
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
