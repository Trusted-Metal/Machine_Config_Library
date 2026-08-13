"""
Phase 0.4 — Schema Self-Tests
Verifies that schema/machine_config_v1.schema.json is well-formed and that
its constraints are correctly specified. No reader implementation required.
"""
import copy
import json
import pathlib
import pytest
import jsonschema
from jsonschema import Draft202012Validator

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
REPO_ROOT = pathlib.Path(__file__).parent.parent.parent
SCHEMA_PATH = REPO_ROOT / "schema" / "machine_config_v1.schema.json"
GOLDEN_PATH = REPO_ROOT / "fixtures" / "reference_output.json"

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------
@pytest.fixture(scope="module")
def schema():
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


# Minimal document that must pass validation — used as base for rejection tests.
_MINIMAL_TRAIN = {
    "train_id": "Optical_Train_01",
    "scanner":      {"manufacturer": "Aconity3D", "model": "AconityScan",          "serial_number": "01522024181"},
    "light_source": {"manufacturer": "nLIGHT",    "model": "NL-FLS-R11-1.00-1070-230", "serial_number": "31070"},
    "collimator":   {"manufacturer": "IPG",        "model": "D50_F120_WC",          "serial_number": "CO3271210"},
    "scanner_card": {"manufacturer": "Raylase",    "model": "SP-ICE-3",             "serial_number": "SP314244"},
}

_MINIMAL_CONFIG = {
    "meta": {
        "machine_name":       "TM-LPBF-02: AconityMIDI+_OG",
        "manufacturer":       "Aconity3D",
        "model":              "AconityMIDI+",
        "serial_number":      "500300_1",
        "file_version":       "1.0",
        "export_date":        "2026-07-23T13:02:12.269Z",
        "configuration_hash": "a" * 64,
    },
    "machine": {},
    "optical_trains": [copy.deepcopy(_MINIMAL_TRAIN)],
}


def _valid():
    """Return a fresh deep copy of the minimal valid config."""
    return copy.deepcopy(_MINIMAL_CONFIG)


def _validate(instance, schema):
    validator = Draft202012Validator(schema)
    return list(validator.iter_errors(instance))


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_schema_parses_as_valid_json():
    """Schema file is readable and parses without error."""
    raw = SCHEMA_PATH.read_text(encoding="utf-8")
    parsed = json.loads(raw)
    assert isinstance(parsed, dict)


def test_schema_is_valid_draft_2020_12(schema):
    """Schema passes jsonschema meta-validation for draft 2020-12."""
    Draft202012Validator.check_schema(schema)


@pytest.mark.skipif(
    not GOLDEN_PATH.exists(),
    reason="reference_output.json not yet generated (Phase 1.7)"
)
def test_golden_output_satisfies_schema(schema):
    """Golden fixture validates against the schema (run after Phase 1.7)."""
    instance = json.loads(GOLDEN_PATH.read_text(encoding="utf-8"))
    errors = _validate(instance, schema)
    assert errors == [], "\n".join(str(e) for e in errors)


def test_minimal_valid_config_passes(schema):
    """Sanity check: the minimal base config used in rejection tests is itself valid."""
    errors = _validate(_valid(), schema)
    assert errors == [], "\n".join(str(e) for e in errors)


def test_missing_meta_is_rejected(schema):
    doc = _valid()
    del doc["meta"]
    assert _validate(doc, schema), "Expected validation errors for missing 'meta'"


def test_missing_machine_is_rejected(schema):
    doc = _valid()
    del doc["machine"]
    assert _validate(doc, schema), "Expected validation errors for missing 'machine'"


def test_missing_optical_trains_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"]
    assert _validate(doc, schema), "Expected validation errors for missing 'optical_trains'"


def test_empty_optical_trains_is_rejected(schema):
    doc = _valid()
    doc["optical_trains"] = []
    assert _validate(doc, schema), "Expected validation errors for empty 'optical_trains'"


def test_seven_optical_trains_is_rejected(schema):
    doc = _valid()
    doc["optical_trains"] = [copy.deepcopy(_MINIMAL_TRAIN) for _ in range(7)]
    for i, t in enumerate(doc["optical_trains"]):
        t["train_id"] = f"Optical_Train_0{i+1}"
    assert _validate(doc, schema), "Expected validation errors for 7 optical trains (maxItems: 6)"


def test_configuration_hash_too_short_is_rejected(schema):
    doc = _valid()
    doc["meta"]["configuration_hash"] = "a" * 63
    assert _validate(doc, schema), "Expected rejection for hash shorter than 64 chars"


def test_configuration_hash_too_long_is_rejected(schema):
    doc = _valid()
    doc["meta"]["configuration_hash"] = "a" * 65
    assert _validate(doc, schema), "Expected rejection for hash longer than 64 chars"


def test_optical_train_missing_train_id_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"][0]["train_id"]
    assert _validate(doc, schema), "Expected rejection for optical train missing 'train_id'"


def test_optical_train_missing_scanner_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"][0]["scanner"]
    assert _validate(doc, schema), "Expected rejection for optical train missing 'scanner'"


def test_optical_train_missing_light_source_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"][0]["light_source"]
    assert _validate(doc, schema), "Expected rejection for optical train missing 'light_source'"


def test_optical_train_missing_collimator_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"][0]["collimator"]
    assert _validate(doc, schema), "Expected rejection for optical train missing 'collimator'"


def test_optical_train_missing_scanner_card_is_rejected(schema):
    doc = _valid()
    del doc["optical_trains"][0]["scanner_card"]
    assert _validate(doc, schema), "Expected rejection for optical train missing 'scanner_card'"


def test_bundled_schema_matches_canonical():
    canonical = pathlib.Path(__file__).parents[2] / "schema" / "machine_config_v1.schema.json"
    if not canonical.exists():
        pytest.skip("canonical schema not present — not running from monorepo")
    bundled = pathlib.Path(__file__).parents[1] / "src" / "machine_config" / "machine_config_v1.schema.json"
    assert json.loads(canonical.read_text(encoding="utf-8")) == json.loads(bundled.read_text(encoding="utf-8"))
