"""Structural guard: no File_Version adapter may depend on another's.

Each `capabilities/vX_Y/` adapter (and the test-only mock v1.1 adapter in
`test_adapter_migration.py`, which models what a real adapter looks like)
must be fully self-contained. If v1.1 depends on v1.0's code, v1.0 can never
change or be removed later without checking v1.1 — and every later version
compounds the risk. See V1_1_IMPLEMENTATION_PLAN.md's "second guardrail" for
the full reasoning; this test exists so that violation can't land silently
again the way it did once already this session.

The only legitimate place two versions are referenced together is the three
shared dispatch registries (`reader.py`, `writer.py`,
`capabilities/__init__.py`) — this test does not scan those, only the
version-specific adapter packages and the mock.
"""
from __future__ import annotations

import ast
import re
from pathlib import Path

_REPO_ROOT = Path(__file__).parent.parent.parent
_CAPABILITIES_DIR = _REPO_ROOT / "python" / "src" / "machine_config" / "capabilities"
_MOCK_FILE = _REPO_ROOT / "python" / "tests" / "test_adapter_migration.py"

_VERSION_DIR_RE = re.compile(r"^v(\d+)_(\d+)$")
_VERSION_TOKEN_RE = re.compile(r"\bv(\d+)_(\d+)\b")


def _version_dirs() -> list[Path]:
    return sorted(
        p for p in _CAPABILITIES_DIR.iterdir()
        if p.is_dir() and _VERSION_DIR_RE.match(p.name)
    )


def _import_tokens(py_file: Path) -> list[str]:
    """All dotted names/module strings referenced by import statements."""
    tree = ast.parse(py_file.read_text(encoding="utf-8"), filename=str(py_file))
    tokens: list[str] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            tokens.extend(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                tokens.append(node.module)
            tokens.extend(alias.name for alias in node.names)
    return tokens


def _violations_in_file(py_file: Path, own_version: str) -> list[str]:
    violations = []
    for token in _import_tokens(py_file):
        for match in _VERSION_TOKEN_RE.finditer(token):
            found_version = f"v{match.group(1)}_{match.group(2)}"
            if found_version != own_version:
                violations.append(
                    f"{py_file.relative_to(_REPO_ROOT)}: imports '{token}' — "
                    f"references {found_version} from within {own_version}"
                )
    return violations


def test_no_version_adapter_depends_on_another():
    """Nothing under capabilities/vX_Y/ may reference capabilities/vZ_W/ for
    a different (Z, W) — each version's adapter must be independently
    hand-authored and self-contained.
    """
    version_dirs = _version_dirs()
    assert len(version_dirs) >= 1, "expected at least one capabilities/vX_Y directory"

    violations: list[str] = []
    for vdir in version_dirs:
        own_version = vdir.name  # e.g. "v1_0"
        for py_file in vdir.rglob("*.py"):
            violations.extend(_violations_in_file(py_file, own_version))

    assert not violations, (
        "Version adapters must not depend on each other:\n" + "\n".join(violations)
    )


def test_mock_v1_1_adapter_does_not_depend_on_v1_0():
    """The test-only mock v1.1 adapter in test_adapter_migration.py must
    also be self-contained — it models what a real adapter looks like
    (see docs/migrations/mock_v1_0_to_v1_1.md), so it should be held to the
    same standard, not just production code.
    """
    assert _MOCK_FILE.exists(), f"expected {_MOCK_FILE} to exist"
    violations = _violations_in_file(_MOCK_FILE, own_version="v1_1")
    assert not violations, (
        "Mock v1.1 adapter must not depend on v1.0:\n" + "\n".join(violations)
    )
