"""
Phase 0.7 — Jinja Adapter Generator Tests
Verifies:
  - generate_adapters.py runs cleanly and creates one file per language
  - All four .j2 templates parse as valid Jinja2
  - Each template renders the DO NOT EDIT MANUALLY header and correct version strings
  - All five change types render correctly
  - Templates render without error when changes list is empty
  - Generator is idempotent: running it twice produces identical output
"""
import hashlib
import subprocess
import sys
import pathlib
import pytest
from jinja2 import Environment, FileSystemLoader

REPO_ROOT    = pathlib.Path(__file__).resolve().parent.parent.parent
TEMPLATE_DIR = REPO_ROOT / "tools" / "templates"
GENERATOR    = REPO_ROOT / "tools" / "generate_adapters.py"

TEMPLATES = [
    "adapter_python.py.j2",
    "adapter_typescript.ts.j2",
    "adapter_rust.rs.j2",
    "adapter_cpp.hpp.j2",
]

# Sample spec covering all five implemented change types
_SAMPLE_SPEC = {
    "from_version": "1.0",
    "to_version":   "1.1",
    "breaking":     False,
    "changes": [
        {"type": "field_add",         "path":      "optical_trains[*]",          "field":     "new_field_nm"},
        {"type": "field_rename",      "path":      "optical_trains[*].scanner",  "old_field": "old_name",       "new_field": "new_name"},
        {"type": "field_remove",      "path":      "optical_trains[*]",          "field":     "deprecated_field"},
        {"type": "field_move",        "from_path": "optical_trains[*]",          "to_path":   "optical_trains[*].scanner", "field":     "move_src_field"},
        {"type": "field_move_rename", "from_path": "optical_trains[*]",          "to_path":   "optical_trains[*].scanner", "old_field": "move_rename_src", "new_field": "move_rename_dst"},
    ],
}

# Spec with no field changes (pure version bump)
_SAMPLE_SPEC_EMPTY = {
    "from_version": "1.1",
    "to_version":   "1.2",
    "breaking":     False,
    "changes": [],
}


@pytest.fixture(scope="module")
def jinja_env():
    """Jinja2 environment matching the generator's settings exactly."""
    return Environment(
        loader=FileSystemLoader(str(TEMPLATE_DIR)),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
    )


# ---------------------------------------------------------------------------
# Generator script
# ---------------------------------------------------------------------------

def test_generator_runs_and_creates_outputs():
    """Generator exits 0 and writes one file per language for every spec found."""
    result = subprocess.run(
        [sys.executable, str(GENERATOR)],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"Generator failed:\n{result.stderr}"
    expected = [
        REPO_ROOT / "python/src/machine_config/adapters/test_v0_9_to_v1_0.py",
        REPO_ROOT / "nodejs/src/adapters/test_v0_9_to_v1_0.ts",
        REPO_ROOT / "rust/src/adapters/test_v0_9_to_v1_0.rs",
        REPO_ROOT / "cpp/include/machine_config/adapters/test_v0_9_to_v1_0.hpp",
    ]
    for path in expected:
        assert path.exists(), f"Generator did not create {path}"


def test_generator_is_idempotent():
    """Running the generator twice produces byte-for-byte identical output."""
    generated = [
        REPO_ROOT / "python/src/machine_config/adapters/test_v0_9_to_v1_0.py",
        REPO_ROOT / "nodejs/src/adapters/test_v0_9_to_v1_0.ts",
        REPO_ROOT / "rust/src/adapters/test_v0_9_to_v1_0.rs",
        REPO_ROOT / "cpp/include/machine_config/adapters/test_v0_9_to_v1_0.hpp",
    ]

    def sha256(path: pathlib.Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()

    before = {p: sha256(p) for p in generated}
    subprocess.run([sys.executable, str(GENERATOR)], check=True, capture_output=True, cwd=str(REPO_ROOT))
    for path in generated:
        assert sha256(path) == before[path], f"Generator not idempotent: {path.name} changed on second run"


# ---------------------------------------------------------------------------
# Template validity
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_template_is_valid_jinja2(jinja_env, tmpl):
    """Each .j2 file loads without a Jinja2 parse error."""
    assert jinja_env.get_template(tmpl) is not None


# ---------------------------------------------------------------------------
# Required header and metadata
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_contains_do_not_edit_header(jinja_env, tmpl):
    """Every rendered adapter must contain the DO NOT EDIT MANUALLY marker."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "DO NOT EDIT MANUALLY" in rendered, f"{tmpl}: missing header"


@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_contains_version_strings(jinja_env, tmpl):
    """Rendered output embeds both the from_version and to_version values."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "1.0" in rendered, f"{tmpl}: missing from_version"
    assert "1.1" in rendered, f"{tmpl}: missing to_version"


# ---------------------------------------------------------------------------
# Change type rendering
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_field_add(jinja_env, tmpl):
    """field_add change produces a comment and the new field name in output."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "field_add" in rendered, f"{tmpl}: missing field_add"
    assert "new_field_nm" in rendered, f"{tmpl}: missing field name"


@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_field_rename(jinja_env, tmpl):
    """field_rename change produces both old and new field names in output."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "field_rename" in rendered, f"{tmpl}: missing field_rename"
    assert "old_name" in rendered, f"{tmpl}: missing old_field"
    assert "new_name" in rendered, f"{tmpl}: missing new_field"


@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_field_remove(jinja_env, tmpl):
    """field_remove change produces a comment and the removed field name in output."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "field_remove" in rendered, f"{tmpl}: missing field_remove"
    assert "deprecated_field" in rendered, f"{tmpl}: missing field name"


# ---------------------------------------------------------------------------
# Empty changes list
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_empty_changes_is_non_empty(jinja_env, tmpl):
    """Template renders to non-empty output even when no changes are defined."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC_EMPTY, name="v1_1_to_v1_2")
    assert "DO NOT EDIT MANUALLY" in rendered
    assert rendered.strip(), f"{tmpl}: empty output for empty changes"


@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_field_move(jinja_env, tmpl):
    """field_move change produces comment and field name in output."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "field_move" in rendered, f"{tmpl}: missing field_move"
    assert "move_src_field" in rendered, f"{tmpl}: missing field name"


@pytest.mark.parametrize("tmpl", TEMPLATES)
def test_rendered_field_move_rename(jinja_env, tmpl):
    """field_move_rename change produces both old and new field names in output."""
    rendered = jinja_env.get_template(tmpl).render(spec=_SAMPLE_SPEC, name="v1_0_to_v1_1")
    assert "field_move_rename" in rendered, f"{tmpl}: missing field_move_rename"
    assert "move_rename_src" in rendered, f"{tmpl}: missing old_field"
    assert "move_rename_dst" in rendered, f"{tmpl}: missing new_field"
