"""File_Version 1.0 on-disk HDF5 layout.

Group paths, train-id formatting, and dataset names for v1.0 live here — not
in the public reader. A later File_Version adapter gets its own layout module.
"""
from __future__ import annotations

FILE_VERSION = "1.0"

ROOT_MACHINE = "Machine"
ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains"
ROOT_OPCUA = "OPCUA"
OPCUA_CLIENT = "OPCUA/Client"
OPCUA_PIPE = "OPCUA/Pipe"
OPCUA_TRIGGERS = "OPCUA/Triggers"

TRAIN_ID_PREFIX = "Optical_Train_"
GROUP_SCANNER = "Scanner"
GROUP_LIGHT_SOURCE = "Light_Source"
GROUP_COLLIMATOR = "Collimator"
GROUP_SCANNER_CARD = "Scanner_Card"
GROUP_OPTIONAL_COMPONENTS = "Optional_Components"
GROUP_CLEARBOX = "ClearBox"
DS_CORRECTION_DATA = "Correction_Data"
DS_INVERSE_CORRECTION_DATA = "Inverse_Correction_Data"
DS_SCAN_FIELD_CORRECTION_FILE = "scan_field_correction_file"


def train_id(index: int) -> str:
    return f"{TRAIN_ID_PREFIX}{index + 1:02d}"


def train_ids(keys) -> list[str]:
    return sorted(k for k in keys if k.startswith(TRAIN_ID_PREFIX))


def train_path(index: int) -> str:
    return train_path_by_id(train_id(index))


def train_path_by_id(tid: str) -> str:
    return f"{ROOT_OPTICAL_TRAINS}/{tid}"


def scanner_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_SCANNER}"


def light_source_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_LIGHT_SOURCE}"


def collimator_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_COLLIMATOR}"


def scanner_card_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{GROUP_SCANNER_CARD}"


def clearbox_path(index: int) -> str:
    return clearbox_path_by_id(train_id(index))


def clearbox_path_by_id(tid: str) -> str:
    return (
        f"{train_path_by_id(tid)}/{GROUP_OPTIONAL_COMPONENTS}/{GROUP_CLEARBOX}"
    )


def correction_data_path(index: int) -> str:
    return f"{clearbox_path(index)}/{DS_CORRECTION_DATA}"


def inverse_correction_data_path(index: int) -> str:
    return f"{clearbox_path(index)}/{DS_INVERSE_CORRECTION_DATA}"


def scan_field_correction_file_path(index: int) -> str:
    return scan_field_correction_file_path_by_id(train_id(index))


def scan_field_correction_file_path_by_id(tid: str) -> str:
    return f"{train_path_by_id(tid)}/{DS_SCAN_FIELD_CORRECTION_FILE}"
