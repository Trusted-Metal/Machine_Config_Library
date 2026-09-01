"""File_Version 1.1 on-disk HDF5 layout.

Group paths, train-id formatting, and dataset names for v1.1 live here — not
in the public reader/writer, and not shared with any other version's layout
module. Deliberately independent of ``capabilities/v1_0/layout.py``: nothing
here imports from or refers to v1.0, so v1.0 can change or be removed later
without affecting v1.1.
"""
from __future__ import annotations

FILE_VERSION = "1.1"

ROOT_MACHINE = "Machine"
ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains"
ROOT_EXTENSIONS = "Extensions"
GROUP_CLEARBOX = "Extensions/ClearBox"
ROOT_OPCUA = "Extensions/TM_OPCUA"
OPCUA_CLIENT = "Extensions/TM_OPCUA/Client"
OPCUA_PIPE = "Extensions/TM_OPCUA/Pipe"
OPCUA_TRIGGERS = "Extensions/TM_OPCUA/Triggers"

TRAIN_ID_PREFIX = "Optical_Train_"
GROUP_SCANNER = "Scanner"
GROUP_LIGHT_SOURCE = "Light_Source"
GROUP_COLLIMATOR = "Collimator"
GROUP_SCANNER_CARD = "Scanner_Card"
GROUP_POWER_CHARACTERIZATION = "Power_Characterization"
DS_DERIVATION_EQUATION_CONSTANTS = "Derivation_Equation_Constants"
DS_CHARACTERIZATION_POINTS = "Characterization_Points"
DS_CORRECTION_DATA = "Correction_Data"
DS_INVERSE_CORRECTION_DATA = "Inverse_Correction_Data"
DS_SCAN_FIELD_CORRECTION_FILE = "scan_field_correction_file"


def train_id(index: int) -> str:
    return f"{TRAIN_ID_PREFIX}{index + 1:02d}"


def train_ids(keys) -> list[str]:
    return sorted(k for k in keys if k.startswith(TRAIN_ID_PREFIX))


def train_path_by_id(tid: str) -> str:
    return f"{ROOT_OPTICAL_TRAINS}/{tid}"


def scanner_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_SCANNER}"


def light_source_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_LIGHT_SOURCE}"


def light_source_power_characterization_path_by_id(tid: str) -> str:
    return f"{light_source_path_by_id(tid)}/{GROUP_POWER_CHARACTERIZATION}"


def collimator_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_COLLIMATOR}"


def scanner_card_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_SCANNER_CARD}"


def clearbox_path_by_id(tid: str) -> str:
    return f"{GROUP_CLEARBOX}/{tid}"


def clearbox_power_characterization_path_by_id(tid: str) -> str:
    return f"{clearbox_path_by_id(tid)}/{GROUP_POWER_CHARACTERIZATION}"


def correction_data_path_by_id(tid: str) -> str:
    return f"{clearbox_path_by_id(tid)}/{DS_CORRECTION_DATA}"


def inverse_correction_data_path_by_id(tid: str) -> str:
    return f"{clearbox_path_by_id(tid)}/{DS_INVERSE_CORRECTION_DATA}"


def scan_field_correction_file_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{DS_SCAN_FIELD_CORRECTION_FILE}"
