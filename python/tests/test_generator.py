"""
Phase 0.7 — Jinja Adapter Generator Tests
Verifies:
  - generate_adapters.py runs cleanly on an empty spec directory
  - All four .j2 templates parse as valid Jinja2
  - Each template renders the DO NOT EDIT MANUALLY header and correct version strings
  - All three change types (field_add, field_rename, field_remove) render correctly
  - Templates render without error when changes list is empty
"""
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

# Sample spec covering all three change types
_SAMPLE_SPEC = {
    "from_version": "1.0",
    "to_version":   "1.1",
    "breaking":     False,
    "changes": [
        {"type": "field_add",    "path": "optical_trains[*]",         "field": "new_field_nm"},
        {"type": "field_rename", "path": "optical_trains[*].scanner", "old_field": "old_name", "new_field": "new_name"},
        {"type": "field_remove", "path": "optical_trains[*]",         "field": "deprecated_field"},
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

def test_generator_runs_with_empty_spec_dir():
    """Generator exits 0 and reports nothing to generate when spec dir is empty."""
    result = subprocess.run(
        [sys.executable, str(GENERATOR)],
        capture_output=True, text=True, cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"Generator failed:\n{result.stderr}"
    assert "No adapter specs found" in result.stdout


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
