#!/usr/bin/env python3
"""
Cross-language correctness checker for Machine Config Library.

Runs in three phases, each reporting failures independently:

  Phase 1  Schema validation  — every language × every fixture validates against
                                schema/machine_config_v1.schema.json
  Phase 2  Read parity        — all active languages produce identical JSON for
                                every fixture (reference, reference_opcua,
                                synthetic_2laser)
  Phase 3  Write interop      — Python builder output is read identically by
                                Python and Rust; Python writer roundtrip
                                (reference → write → re-read) is verified the
                                same way.  Rust-writes → Python-reads will be
                                added when the Rust CLI gains a build command.

Usage:
  python tools/cross_check.py                       # all available languages
  python tools/cross_check.py --langs python,rust
  python tools/cross_check.py --skip-write-interop
  python tools/cross_check.py --verbose

Exit code: 0 if all active phases pass, 1 otherwise.

To add a new language, add one entry to RUNNERS — no other file needs changing.
"""

from __future__ import annotations

import argparse
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Callable

# Optional heavy deps — both available in CI but degrade gracefully locally.
try:
    from deepdiff import DeepDiff  # type: ignore
    _DEEPDIFF = True
except ImportError:
    _DEEPDIFF = False

try:
    import jsonschema  # type: ignore  # already a project dep via pyproject.toml
    _JSONSCHEMA = True
except ImportError:
    _JSONSCHEMA = False

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

REPO = Path(__file__).resolve().parent.parent
SCHEMA_FILE = REPO / "schema" / "machine_config_v1.schema.json"

FIXTURES: dict[str, Path] = {
    "reference":       REPO / "fixtures" / "reference_config.h5",
    "reference_opcua": REPO / "fixtures" / "reference_config_opcua.h5",
    "synthetic":       REPO / "fixtures" / "synthetic_2laser.h5",
}

_EXT = ".exe" if platform.system() == "Windows" else ""


def _rust_bin() -> Path:
    release = REPO / "rust" / "target" / "release" / f"machine-config-cli{_EXT}"
    debug   = REPO / "rust" / "target" / "debug"   / f"machine-config-cli{_EXT}"
    return release if release.exists() else debug


def _python_cli() -> str:
    """Find the machine-config CLI entry point.

    Prefers the script alongside sys.executable (works inside a venv) then
    falls back to PATH (works in CI after 'pip install').
    """
    local = Path(sys.executable).parent / f"machine-config{_EXT}"
    if local.exists():
        return str(local)
    found = shutil.which("machine-config")
    if found:
        return found
    raise FileNotFoundError(
        "machine-config CLI not found. Install with: pip install -e python/"
    )


# ---------------------------------------------------------------------------
# Language runners
# ---------------------------------------------------------------------------
# Each entry is a callable(fixture: Path) -> list[str] that returns the argv
# for the language's CLI.  Add nodejs/cpp here when they land.

RUNNERS: dict[str, Callable[[Path], list[str]]] = {
    "python": lambda fix: [_python_cli(), "export-json", str(fix)],
    "rust":   lambda fix: [str(_rust_bin()), "export-json", str(fix)],
    # "nodejs": lambda fix: ["node", str(REPO / "nodejs/dist/cli.js"), "export-json", str(fix)],
    # "cpp":    lambda fix: [str(REPO / f"cpp/build/machine_config_cli{_EXT}"), "export-json", str(fix)],
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

FAIL = "[FAIL]"
PASS = "[PASS]"
SKIP = "[SKIP]"
WARN = "[WARN]"


def _run(cmd: list[str]) -> dict:
    """Run a CLI command; return its stdout parsed as JSON. Raises on error."""
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8")
    if r.returncode != 0:
        raise RuntimeError(
            f"{Path(cmd[0]).name} exited {r.returncode}\nstderr: {r.stderr.strip()}"
        )
    return json.loads(r.stdout)


def _diff(a: dict, b: dict) -> list[str]:
    """Return human-readable differences; empty list means equal."""
    if _DEEPDIFF:
        d = DeepDiff(a, b, significant_digits=8, ignore_order=False)
        return [d.pretty()] if d else []
    sa = json.dumps(a, sort_keys=True)
    sb = json.dumps(b, sort_keys=True)
    if sa != sb:
        return ["outputs differ (install deepdiff for field-level detail: pip install deepdiff)"]
    return []


def _validate_schema(data: dict, schema: dict) -> list[str]:
    if not _JSONSCHEMA:
        return []
    try:
        jsonschema.validate(data, schema)
        return []
    except jsonschema.ValidationError as exc:
        return [exc.message]


# ---------------------------------------------------------------------------
# Phase 1 — Schema validation
# ---------------------------------------------------------------------------

def phase_schema(langs: list[str], schema: dict, verbose: bool) -> bool:
    """Every language × every fixture must validate against the JSON Schema."""
    print("\n=== Phase 1: Schema Validation ===")
    if not _JSONSCHEMA:
        print(f"{SKIP} jsonschema not installed — pip install jsonschema")
        return True

    failures: list[str] = []
    for lang in langs:
        for name, path in FIXTURES.items():
            tag = f"{lang}/{name}"
            try:
                data = _run(RUNNERS[lang](path))
            except Exception as exc:
                print(f"{FAIL} {tag}: runner error: {exc}")
                failures.append(tag)
                continue
            errs = _validate_schema(data, schema)
            if errs:
                print(f"{FAIL} {tag}: {errs[0]}")
                failures.append(tag)
            elif verbose:
                print(f"{PASS} {tag}")

    if not failures:
        n = len(langs) * len(FIXTURES)
        print(f"{PASS} {n} combinations validate against schema.")
    return not failures


# ---------------------------------------------------------------------------
# Phase 2 — Read parity
# ---------------------------------------------------------------------------

def phase_read_parity(langs: list[str], verbose: bool) -> bool:
    """All languages must produce identical JSON for every fixture."""
    print("\n=== Phase 2: Read Parity ===")
    if len(langs) < 2:
        print(f"{SKIP} Need ≥2 languages for parity check.")
        return True

    ref_lang = langs[0]
    failures: list[str] = []

    for name, path in FIXTURES.items():
        outputs: dict[str, dict] = {}
        for lang in langs:
            try:
                outputs[lang] = _run(RUNNERS[lang](path))
            except Exception as exc:
                print(f"{FAIL} {lang}/{name}: {exc}")
                failures.append(f"{lang}/{name}")

        if ref_lang not in outputs:
            continue  # reference language failed for this fixture; skip comparisons

        for lang, data in outputs.items():
            if lang == ref_lang:
                continue
            tag = f"{ref_lang} vs {lang} / {name}"
            diffs = _diff(outputs[ref_lang], data)
            if diffs:
                print(f"{FAIL} {tag}:")
                for line in diffs:
                    print(f"  {line}")
                failures.append(tag)
            elif verbose:
                print(f"{PASS} {tag}")

    if not failures:
        combos = len(langs) * (len(langs) - 1) // 2 * len(FIXTURES)
        print(f"{PASS} {len(langs)} languages agree on all {len(FIXTURES)} fixtures ({combos} comparisons).")
    return not failures


# ---------------------------------------------------------------------------
# Phase 3 — Write interoperability
# ---------------------------------------------------------------------------

def phase_write_interop(langs: list[str], verbose: bool) -> bool:
    """Python-written HDF5 must be read identically by Python and Rust."""
    print("\n=== Phase 3: Write Interoperability ===")
    if "python" not in langs or "rust" not in langs:
        print(f"{SKIP} Requires both python and rust.")
        return True

    try:
        from machine_config.builder import MockConfigBuilder  # type: ignore
        from machine_config.reader import MachineConfigReader  # type: ignore
        from machine_config.writer import MachineConfigWriter  # type: ignore
    except ImportError as exc:
        print(f"{SKIP} machine_config not importable: {exc}")
        return True

    failures: list[str] = []

    # 3a — Python builder (1-laser synthetic) → compare Python vs Rust readers
    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        mock_path = Path(f.name)
    try:
        MockConfigBuilder(n_lasers=1).save(str(mock_path))
        tag = "python-builder (1-laser) → rust reader"
        py_out   = _run(RUNNERS["python"](mock_path))
        rust_out = _run(RUNNERS["rust"](mock_path))
        diffs = _diff(py_out, rust_out)
        if diffs:
            print(f"{FAIL} {tag}:")
            for line in diffs:
                print(f"  {line}")
            failures.append(tag)
        elif verbose:
            print(f"{PASS} {tag}")
    except Exception as exc:
        print(f"{FAIL} python-builder: {exc}")
        failures.append("python-builder")
    finally:
        mock_path.unlink(missing_ok=True)

    # 3b — Python writer roundtrip (reference → write → re-read)
    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        rt_path = Path(f.name)
    try:
        reader = MachineConfigReader(str(FIXTURES["reference"]))
        config = reader.parse()
        MachineConfigWriter(config).write(str(rt_path))
        tag = "python-writer roundtrip → rust reader"
        py_out   = _run(RUNNERS["python"](rt_path))
        rust_out = _run(RUNNERS["rust"](rt_path))
        diffs = _diff(py_out, rust_out)
        if diffs:
            print(f"{FAIL} {tag}:")
            for line in diffs:
                print(f"  {line}")
            failures.append(tag)
        elif verbose:
            print(f"{PASS} {tag}")
    except Exception as exc:
        print(f"{FAIL} python-writer roundtrip: {exc}")
        failures.append("python-writer roundtrip")
    finally:
        rt_path.unlink(missing_ok=True)

    if not failures:
        print(f"{PASS} Write interoperability checks passed.")
    return not failures


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def _parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "--langs",
        default=",".join(RUNNERS),
        help="Comma-separated list of languages to check (default: all in RUNNERS).",
    )
    p.add_argument(
        "--skip-write-interop",
        action="store_true",
        help="Skip Phase 3 (write interoperability).",
    )
    p.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Print PASS lines in addition to FAIL/SKIP lines.",
    )
    return p.parse_args()


def main() -> None:
    args = _parse_args()
    requested = [l.strip() for l in args.langs.split(",") if l.strip()]
    unknown = [l for l in requested if l not in RUNNERS]
    active  = [l for l in requested if l in RUNNERS]

    if unknown:
        print(f"{WARN} Unknown/not-yet-implemented languages skipped: {', '.join(unknown)}")

    # Filter to languages whose binary is actually present
    reachable: list[str] = []
    for lang in active:
        if lang == "python":
            reachable.append(lang)
            continue
        try:
            _run(RUNNERS[lang](FIXTURES["reference"]))
            reachable.append(lang)
        except FileNotFoundError:
            print(f"{WARN} {lang} binary not found — build it first, then re-run.")
        except Exception:
            reachable.append(lang)  # binary exists; failure is content, not missing binary

    if not reachable:
        print("No reachable languages. Nothing to check.")
        sys.exit(0)

    print(f"Active languages: {', '.join(reachable)}")

    schema = json.loads(SCHEMA_FILE.read_text())

    results = [
        phase_schema(reachable, schema, args.verbose),
        phase_read_parity(reachable, args.verbose),
    ]
    if not args.skip_write_interop:
        results.append(phase_write_interop(reachable, args.verbose))

    print()
    if all(results):
        print("All checks passed.")
        sys.exit(0)
    else:
        n_failed = sum(1 for r in results if not r)
        print(f"{n_failed} phase(s) failed.")
        sys.exit(1)


if __name__ == "__main__":
    main()
