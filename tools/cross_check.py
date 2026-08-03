#!/usr/bin/env python3
"""
Cross-language correctness checker for Machine Config Library.

Runs in three phases, each reporting failures independently:

  Phase 1  Schema validation  — every language × every fixture validates against
                                schema/machine_config_v1.schema.json
  Phase 2  Read parity        — all active languages produce identical JSON for
                                every fixture (reference, reference_opcua,
                                synthetic_2laser)
  Phase 3  Write interop      — Every active writer language serialises the
                                reference fixture from canonical JSON to HDF5.
                                All active readers must agree (parity) and the
                                Python reader must reproduce the canonical JSON
                                (fidelity).  Add a language to WRITERS when its
                                write-hdf5 CLI subcommand is implemented.

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
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable

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
    "nodejs": lambda fix: ["node", str(REPO / "nodejs/dist/cli.js"), "export-json", str(fix)],
    # "cpp":    lambda fix: [str(REPO / f"cpp/build/machine_config_cli{_EXT}"), "export-json", str(fix)],
}

# Argv *prefix* for subcommands other than export-json (e.g. correction-hash).
# A prefix, not a single binary path, because Node.js needs ["node", "<cli.js>"].
BINARIES: dict[str, Callable[[], list[str]]] = {
    "python": lambda: [_python_cli()],
    "rust":   lambda: [str(_rust_bin())],
    "nodejs": lambda: ["node", str(REPO / "nodejs/dist/cli.js")],
}

# Each entry is a callable(json_path: Path, h5_path: Path) -> list[str] that
# returns the argv for the language's write-from-JSON CLI command.
# Input:  canonical JSON file (output of `export-json` on the reference fixture)
# Output: .h5 file verified by all reader languages in phase_write_interop().
# Add nodejs/cpp here when their writers land.
WRITERS: dict[str, Callable[[Path, Path], list[str]]] = {
    "python": lambda j, h: [_python_cli(), "write", str(j), "--output", str(h)],
    "rust":   lambda j, h: [str(_rust_bin()), "write-hdf5", str(j), str(h)],
    "nodejs": lambda j, h: ["node", str(REPO / "nodejs/dist/cli.js"), "write-hdf5", str(j), str(h)],
    # "cpp": lambda j, h: [str(REPO / f"cpp/build/machine_config_cli{_EXT}"), "write-hdf5", str(j), str(h)],
}

# Each entry is a callable(input: Path, output: Path) -> list[str] that returns
# the argv for the language's copy-hdf5 CLI subcommand (read with full binary
# data, write verbatim — used in Phase 3.5 binary round-trip).
COPIERS: dict[str, Callable[[Path, Path], list[str]]] = {
    "python": lambda i, o: [_python_cli(), "copy-hdf5", str(i), "--output", str(o)],
    "rust":   lambda i, o: [str(_rust_bin()), "copy-hdf5", str(i), str(o)],
    "nodejs": lambda i, o: ["node", str(REPO / "nodejs/dist/cli.js"), "copy-hdf5", str(i), str(o)],
    # "cpp":    lambda i, o: [str(REPO / f"cpp/build/machine_config_cli{_EXT}"), "copy-hdf5", str(i), str(o)],
    # "go":     lambda i, o: [str(REPO / f"go/machine-config-cli{_EXT}"), "copy-hdf5", str(i), str(o)],
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
    # PYTHONUTF8=1 forces the subprocess's stdout to UTF-8 on Windows (default
    # is the console code page, e.g. CP1252, which cannot encode μ, etc.).
    env = {**os.environ, "PYTHONUTF8": "1"}
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", env=env)
    if r.returncode != 0:
        raise RuntimeError(
            f"{Path(cmd[0]).name} exited {r.returncode}\nstderr: {r.stderr.strip()}"
        )
    return json.loads(r.stdout)


def _run_text(cmd: list[str]) -> str:
    """Run a CLI command; return stripped stdout text. Raises on error."""
    env = {**os.environ, "PYTHONUTF8": "1"}
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", env=env)
    if r.returncode != 0:
        raise RuntimeError(
            f"{Path(cmd[0]).name} exited {r.returncode}\nstderr: {r.stderr.strip()}"
        )
    return r.stdout.strip()


def _normalize_numbers(obj: Any) -> Any:
    """Recursively convert whole-number floats to int.

    JSON has no int/float distinction.  Python's json.loads maps ``670`` to
    ``int`` and ``670.0`` to ``float``; JavaScript's JSON.stringify always
    emits ``670`` for a whole-number float.  Normalising before comparison
    prevents spurious mismatches without hiding real differences (value
    changes, string-vs-number, null-vs-number, missing fields).
    """
    if isinstance(obj, float) and obj.is_integer():
        return int(obj)
    if isinstance(obj, dict):
        return {k: _normalize_numbers(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [_normalize_numbers(v) for v in obj]
    return obj


def _diff(a: dict, b: dict) -> list[str]:
    """Return human-readable differences; empty list means equal.

    ``ignore_numeric_type_changes=True`` is passed to DeepDiff so that
    ``670`` (int, from JavaScript) and ``670.0`` (float, from Python/Rust)
    compare equal.  JSON has no int/float distinction; real type errors
    (e.g. a string where a number is expected) are still caught by Phase 1
    schema validation before _diff is ever called.
    """
    if _DEEPDIFF:
        d = DeepDiff(
            a, b,
            significant_digits=8,
            ignore_order=False,
            ignore_numeric_type_changes=True,
        )
        return [d.pretty()] if d else []
    # Fallback: normalise whole-number floats so 670.0 and 670 compare equal.
    sa = json.dumps(_normalize_numbers(a), sort_keys=True)
    sb = json.dumps(_normalize_numbers(b), sort_keys=True)
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


def _ref_lang(langs: list[str]) -> str:
    """Return Python as the fixed parity anchor.

    Falls back to langs[0] with a warning only when Python is absent, so
    a local run with --langs rust,nodejs still produces useful output instead
    of crashing.  In CI, Python is always present.
    """
    if "python" in langs:
        return "python"
    print(f"{WARN} Python not in active set — using {langs[0]!r} as parity anchor "
          "(two languages with the same bug can both agree; results may be unreliable)")
    return langs[0]


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

    ref_lang = _ref_lang(langs)
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
    """Every active writer produces HDF5 that all active readers parse identically
    (parity) and that matches the canonical JSON from the reference fixture (fidelity).

    For each writer language in WRITERS ∩ langs:
      1. Write an HDF5 from the canonical reference JSON via that language's CLI.
      2. Read it back with every active reader language — all outputs must agree.
      3. Python's output must match the canonical JSON (fidelity).

    Phase 3a (MockConfigBuilder 1-laser synthetic) was removed: its read-parity
    coverage is fully superseded by Phase 2, which validates all active languages
    against fixtures/synthetic_2laser.h5 (a committed builder output).
    All writes now go through the CLI so every language is tested uniformly.
    """
    print("\n=== Phase 3: Write Interoperability ===")

    writer_langs = [l for l in langs if l in WRITERS]
    if not writer_langs:
        print(f"{SKIP} No writer-capable language in active set "
              f"(supported: {', '.join(WRITERS)}).")
        return True
    if "python" not in langs:
        print(f"{SKIP} Python required as the fidelity reference reader.")
        return True

    failures: list[str] = []

    # Canonical JSON — produced once by the Python reference implementation.
    try:
        canonical = _run(RUNNERS["python"](FIXTURES["reference"]))
    except Exception as exc:
        print(f"{FAIL} Could not read reference fixture with Python: {exc}")
        return False

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as jf:
        json_tmp = Path(jf.name)
    json_tmp.write_text(json.dumps(canonical, ensure_ascii=False), encoding="utf-8")

    try:
        for writer_lang in writer_langs:
            with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as hf:
                h5_tmp = Path(hf.name)
            try:
                # Step 1: write HDF5 via this language's CLI.
                env = {**os.environ, "PYTHONUTF8": "1"}
                r = subprocess.run(
                    WRITERS[writer_lang](json_tmp, h5_tmp),
                    capture_output=True, text=True, encoding="utf-8", env=env,
                )
                if r.returncode != 0:
                    raise RuntimeError(
                        f"{writer_lang} writer exited {r.returncode}\n"
                        f"stderr: {r.stderr.strip()}"
                    )

                # Step 2: read back with all active reader languages.
                reader_outputs: dict[str, dict] = {}
                for reader_lang in langs:
                    try:
                        reader_outputs[reader_lang] = _run(RUNNERS[reader_lang](h5_tmp))
                    except Exception as exc:
                        tag = f"{writer_lang}-writes → {reader_lang}-reads"
                        print(f"{FAIL} {tag}: {exc}")
                        failures.append(tag)

                # Parity: every reader must agree with the Python anchor.
                ref_reader = _ref_lang(langs)
                if ref_reader in reader_outputs:
                    for reader_lang, data in reader_outputs.items():
                        if reader_lang == ref_reader:
                            continue
                        tag = f"{writer_lang}-writes → {ref_reader} vs {reader_lang} parity"
                        diffs = _diff(reader_outputs[ref_reader], data)
                        if diffs:
                            print(f"{FAIL} {tag}:")
                            for line in diffs:
                                print(f"  {line}")
                            failures.append(tag)
                        elif verbose:
                            print(f"{PASS} {tag}")

                # Fidelity: Python's reading must match the canonical JSON.
                if "python" in reader_outputs:
                    tag = f"{writer_lang}-writes → fidelity"
                    fid_diffs = _diff(canonical, reader_outputs["python"])
                    if fid_diffs:
                        print(f"{FAIL} {tag}:")
                        for line in fid_diffs:
                            print(f"  {line}")
                        failures.append(tag)
                    elif verbose:
                        print(f"{PASS} {tag}")

            except Exception as exc:
                tag = f"{writer_lang}-writer"
                print(f"{FAIL} {tag}: {exc}")
                failures.append(tag)
            finally:
                h5_tmp.unlink(missing_ok=True)
    finally:
        json_tmp.unlink(missing_ok=True)

    if not failures:
        n_parity = len(writer_langs) * max(0, len(langs) - 1)
        n_fidelity = len(writer_langs)
        print(
            f"{PASS} Write interoperability: {len(writer_langs)} writer(s) × "
            f"{len(langs)} reader(s) — {n_parity} parity + {n_fidelity} fidelity checks passed."
        )
    return not failures


# ---------------------------------------------------------------------------
# Phase 4 — Correction data hash parity
# ---------------------------------------------------------------------------

def _correction_hash_cmd(
    lang: str, path: Path, train: int = 0, inverse: bool = False
) -> list[str]:
    """Build the correction-hash CLI invocation for the given language."""
    cmd = [*BINARIES[lang](), "correction-hash", str(path), "--train", str(train)]
    if inverse:
        cmd.append("--inverse")
    return cmd


def phase_correction_hash(langs: list[str], verbose: bool) -> bool:
    """Python and Rust must produce identical SHA-256 hashes for correction grids.

    Values are hashed as flat little-endian float64 bytes, so the digest is
    independent of platform byte order and directly comparable across languages.
    """
    print("\n=== Phase 4: Correction Data Hashes ===")
    if len(langs) < 2:
        print(f"{SKIP} Need ≥2 languages for hash parity check.")
        return True

    ref_lang = _ref_lang(langs)
    failures: list[str] = []

    for name, path in FIXTURES.items():
        for inverse in [False, True]:
            kind = "inverse_correction" if inverse else "correction"
            hashes: dict[str, str] = {}
            for lang in langs:
                try:
                    hashes[lang] = _run_text(
                        _correction_hash_cmd(lang, path, inverse=inverse)
                    )
                except Exception as exc:
                    tag = f"{lang}/{name}/{kind}"
                    print(f"{FAIL} {tag}: {exc}")
                    failures.append(tag)

            if ref_lang not in hashes:
                continue

            for lang, h in hashes.items():
                if lang == ref_lang:
                    continue
                tag = f"{ref_lang} vs {lang} / {name} {kind}"
                if h != hashes[ref_lang]:
                    print(f"{FAIL} {tag}:")
                    print(f"  {ref_lang}: {hashes[ref_lang]}")
                    print(f"  {lang}:   {h}")
                    failures.append(tag)
                elif verbose:
                    print(f"{PASS} {tag}")

    if not failures:
        combos = len(langs) * (len(langs) - 1) // 2 * len(FIXTURES) * 2
        print(f"{PASS} Correction data hashes match ({combos} comparisons).")
    return not failures


# ---------------------------------------------------------------------------
# Phase 3.5 — Binary copy round-trip
# ---------------------------------------------------------------------------

def phase_binary_copy(langs: list[str], verbose: bool) -> bool:
    """Each writer copies reference_config.h5 via copy-hdf5 (full binary round-trip).

    Two checks per writer × direction (forward + inverse correction grid):
      Parity   — all reader languages produce the same correction hash for the copy.
      Fidelity — Python’s hash of the copy matches Python’s hash of the original fixture.
    """
    print("\n=== Phase 3.5: Binary Copy Round-trip ===")

    copy_langs = [l for l in langs if l in COPIERS]
    if not copy_langs:
        print(f"{SKIP} No copy-capable languages in active set.")
        return True

    # Baseline: Python’s correction hashes of the original reference fixture.
    baselines: dict[bool, str] = {}
    for inverse in [False, True]:
        try:
            baselines[inverse] = _run_text(
                _correction_hash_cmd("python", FIXTURES["reference"], inverse=inverse)
            )
        except Exception as exc:
            print(f"{FAIL} Could not establish baseline hash from original fixture: {exc}")
            return False

    failures: list[str] = []

    for copy_lang in copy_langs:
        with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as hf:
            copy_path = Path(hf.name)
        try:
            env = {**os.environ, "PYTHONUTF8": "1"}
            r = subprocess.run(
                COPIERS[copy_lang](FIXTURES["reference"], copy_path),
                capture_output=True, text=True, encoding="utf-8", env=env,
            )
            if r.returncode != 0:
                tag = f"{copy_lang}-copy"
                print(f"{FAIL} {tag}: copy-hdf5 exited {r.returncode}\n"
                      f"  stderr: {r.stderr.strip()}")
                failures.append(tag)
                continue

            for inverse in [False, True]:
                kind = "inverse" if inverse else "forward"

                # Gather hashes from all reader languages.
                hashes: dict[str, str] = {}
                for reader_lang in langs:
                    try:
                        hashes[reader_lang] = _run_text(
                            _correction_hash_cmd(reader_lang, copy_path, inverse=inverse)
                        )
                    except Exception as exc:
                        tag = f"{copy_lang}-copy / {reader_lang}-reads / {kind}"
                        print(f"{FAIL} {tag}: {exc}")
                        failures.append(tag)

                # Parity: all readers must agree with the Python anchor.
                ref_reader = _ref_lang(langs)
                if ref_reader in hashes:
                    for reader_lang, h in hashes.items():
                        if reader_lang == ref_reader:
                            continue
                        tag = f"{copy_lang}-copy / {ref_reader} vs {reader_lang} / {kind} parity"
                        if h != hashes[ref_reader]:
                            print(f"{FAIL} {tag}:")
                            print(f"  {ref_reader}: {hashes[ref_reader]}")
                            print(f"  {reader_lang}: {h}")
                            failures.append(tag)
                        elif verbose:
                            print(f"{PASS} {tag}")

                # Fidelity: Python’s hash of the copy must match Python’s hash
                # of the original fixture.
                if "python" in hashes:
                    tag = f"{copy_lang}-copy / python fidelity / {kind}"
                    if hashes["python"] != baselines[inverse]:
                        print(f"{FAIL} {tag}:")
                        print(f"  copy:     {hashes['python']}")
                        print(f"  original: {baselines[inverse]}")
                        failures.append(tag)
                    elif verbose:
                        print(f"{PASS} {tag}")

        except Exception as exc:
            tag = f"{copy_lang}-copy"
            print(f"{FAIL} {tag}: {exc}")
            failures.append(tag)
        finally:
            copy_path.unlink(missing_ok=True)

    if not failures:
        checks = len(copy_langs) * (max(0, len(langs) - 1) + 1) * 2
        print(
            f"{PASS} Binary copy round-trip: {len(copy_langs)} writer(s) × "
            f"{len(langs)} reader(s) × 2 directions — {checks} checks passed."
        )
    return not failures


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
        "--skip-schema",
        action="store_true",
        help="Skip Phase 1 (schema validation).",
    )
    p.add_argument(
        "--skip-read-parity",
        action="store_true",
        help="Skip Phase 2 (read parity).",
    )
    p.add_argument(
        "--skip-correction-hash",
        action="store_true",
        help="Skip Phase 4 (correction data hash parity).",
    )
    p.add_argument(
        "--skip-binary-copy",
        action="store_true",
        help="Skip Phase 3.5 (binary copy round-trip).",
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

    # None = skipped, True = passed, False = failed
    results: list[tuple[str, bool | None]] = [
        ("Phase 1 (Schema)",          None if args.skip_schema          else phase_schema(reachable, schema, args.verbose)),
        ("Phase 2 (Read Parity)",     None if args.skip_read_parity     else phase_read_parity(reachable, args.verbose)),
        ("Phase 3 (Write Interop)",   None if args.skip_write_interop   else phase_write_interop(reachable, args.verbose)),
        ("Phase 3.5 (Binary Copy)",   None if args.skip_binary_copy     else phase_binary_copy(reachable, args.verbose)),
        ("Phase 4 (Correction Hash)", None if args.skip_correction_hash else phase_correction_hash(reachable, args.verbose)),
    ]

    print()
    skipped = [name for name, r in results if r is None]
    failed  = [name for name, r in results if r is False]
    active  = sum(1 for _, r in results if r is not None)

    if skipped:
        print(f"{SKIP} Skipped: {', '.join(skipped)}")
    if failed:
        print(f"{len(failed)} phase(s) failed: {', '.join(failed)}")
        sys.exit(1)
    else:
        print(f"All {active} active phase(s) passed.")
        sys.exit(0)


if __name__ == "__main__":
    main()
