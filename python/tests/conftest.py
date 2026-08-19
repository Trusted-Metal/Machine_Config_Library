"""Shared pytest fixtures for the machine_config test suite."""
from pathlib import Path

import pytest

from machine_config.reader import MachineConfigReader

# ---------------------------------------------------------------------------
# Fixture paths
# ---------------------------------------------------------------------------
_REPO_ROOT = Path(__file__).parent.parent.parent
REFERENCE_H5 = _REPO_ROOT / "fixtures" / "reference_config.h5"
REFERENCE_OPCUA_H5 = _REPO_ROOT / "fixtures" / "reference_config_opcua.h5"
OPCUA_MISSING_REQUIRED_H5 = (
    _REPO_ROOT / "docs" / "validation" / "fixtures" / "opcua_missing_required.h5"
)


# ---------------------------------------------------------------------------
# Reader / config fixtures  (session-scoped — parse once for the whole run)
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def reference_reader() -> MachineConfigReader:
    return MachineConfigReader(REFERENCE_H5)


@pytest.fixture(scope="session")
def reference_config(reference_reader: MachineConfigReader):
    return reference_reader.parse()


@pytest.fixture(scope="session")
def opcua_reader() -> MachineConfigReader:
    return MachineConfigReader(REFERENCE_OPCUA_H5)


@pytest.fixture(scope="session")
def opcua_missing_required_reader() -> MachineConfigReader:
    return MachineConfigReader(OPCUA_MISSING_REQUIRED_H5)
