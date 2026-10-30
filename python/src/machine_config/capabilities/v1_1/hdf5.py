"""File_Version 1.1 HDF5 reader — on-disk layout, attribute names, and casting.

Public MachineConfigReader peeks File_Version then dispatches here.
Do not parse other on-disk versions with this module.

Deliberately independent of ``capabilities/v1_0/hdf5.py`` — every helper used
here (type-conversion, sub-group parsing) is defined fresh in this module,
even where the on-disk shape happens to match v1.0 today (Collimator,
ScannerCard, ScanFieldCorrectionFile, OPCUA's inner Client/Pipe/Triggers
shape). v1.0 could change or be removed later without affecting v1.1.
"""
from __future__ import annotations

import base64
import json
from pathlib import Path
from typing import Optional

import h5py
import numpy as np

from machine_config.extension_contract import tree_from_json, tree_to_json
from machine_config.extensions_hdf5 import read_extension_tree
from machine_config.models import (
    AxisConfig,
    BuildPlate,
    CalibrationPoint,
    Collimator,
    EquationConstant,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    PowerCharacterization,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)
from machine_config.capabilities.file_version import MissingRequiredGroup
from machine_config.schema import SCHEMA_VERSION

from . import layout

# Fixed-length compound-dataset convention for Power_Characterization's
# Derivation_Equation_Constants/Characterization_Points (see
# SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string convention") —
# defined fresh here rather than imported from v1_0, per this module's
# independence requirement.
EQUATION_CONSTANT_NAME_MAX_BYTES = 64
_EQUATION_CONSTANT_DTYPE = np.dtype(
    [
        ("name", h5py.string_dtype(encoding="utf-8", length=EQUATION_CONSTANT_NAME_MAX_BYTES)),
        ("value", "f8"),
    ]
)
_CALIBRATION_POINT_DTYPE = np.dtype(
    [("input_value", "f8"), ("output_value", "f8")]
)

_KNOWN_ROOT_KEYS: frozenset[str] = frozenset(
    {"machine_name", "manufacturer", "model", "serial_number",
     "File_Version", "Export_Date", "Configuration_Hash"}
)

def _hdf5_attrs_extra(attrs, known: frozenset[str]) -> dict:
    result = {}
    for k, v in attrs.items():
        if k in known:
            continue
        if isinstance(v, np.integer):
            result[k] = int(v)
        elif isinstance(v, np.floating):
            result[k] = float(v)
        elif isinstance(v, bytes):
            result[k] = v.decode()
        else:
            result[k] = str(v)
    return result


class Hdf5AdapterV1_1:
    """Parse a File_Version 1.1 machine-config HDF5 file into stable models."""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def parse(self) -> MachineConfig:
        with h5py.File(self.path, "r") as f:
            try:
                return self._parse(f)
            except KeyError as exc:
                raise MissingRequiredGroup(str(exc)) from exc

    def get_scan_field_correction_bytes(self, train_index: int) -> bytes:
        with h5py.File(self.path, "r") as f:
            tid = layout.train_id(train_index)
            return bytes(f[layout.scan_field_correction_file_path_by_id(tid)][:])

    def get_raw_group(self, hdf5_path: str) -> dict:
        with h5py.File(self.path, "r") as f:
            if hdf5_path not in f:
                return {}
            return dict(f[hdf5_path].attrs)

    def to_json(self, indent: int = 2, include_binary: bool = False) -> str:
        config = self.parse()
        return json.dumps(
            self._config_to_dict(config, include_binary=include_binary),
            indent=indent,
            default=str,
            ensure_ascii=False,
        )

    # ------------------------------------------------------------------
    # Attribute-reading helpers (all static; independent copies of the
    # equivalent v1.0 helpers — see module docstring)
    # ------------------------------------------------------------------

    @staticmethod
    def _read_str(attrs, key: str) -> Optional[str]:
        val = attrs.get(key)
        if val is None:
            return None
        s = str(val).strip()
        return s if s else None

    @staticmethod
    def _read_float(attrs, key: str) -> Optional[float]:
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, (str, bytes)) and str(val).strip() == "":
            return None
        return float(val)

    @staticmethod
    def _read_int(attrs, key: str) -> Optional[int]:
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, (str, bytes)) and str(val).strip() == "":
            return None
        return int(val)

    @staticmethod
    def _read_bool_from_int(attrs, key: str) -> Optional[bool]:
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, (str, bytes)) and str(val).strip() == "":
            return None
        i = int(val)
        if i == 0:
            return False
        if i == 1:
            return True
        raise ValueError(
            f"Attribute '{key}' has value {i!r}; expected 0 or 1 (Rule 8)."
        )

    @staticmethod
    def _read_str_locked(attrs, unit_key: str, expected_unit: str) -> Optional[str]:
        val = attrs.get(unit_key)
        if val is None:
            return None
        s = str(val).strip()
        if not s:
            return None
        if s != expected_unit:
            raise ValueError(
                f"Unit attribute '{unit_key}' has value {s!r}; "
                f"expected {expected_unit!r}.  Update the reader if the HDF5 "
                f"exporter changed units (Rule 8)."
            )
        return s

    # ------------------------------------------------------------------
    # Private parsing helpers
    # ------------------------------------------------------------------

    def _parse(self, f: h5py.File) -> MachineConfig:
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=str(f.attrs.get("machine_name", "")),
            manufacturer=str(f.attrs.get("manufacturer", "")),
            model=str(f.attrs.get("model", "")),
            serial_number=str(f.attrs.get("serial_number", "")),
            file_version=str(f.attrs.get("File_Version", "")).strip() or "1.1",
            export_date=str(f.attrs.get("Export_Date", "")),
            configuration_hash=str(f.attrs.get("Configuration_Hash", "")),
            extra=_hdf5_attrs_extra(f.attrs, _KNOWN_ROOT_KEYS),
        )

        ma = f[layout.ROOT_MACHINE].attrs
        build_plate = BuildPlate(
            x=self._read_float(ma, "Build_Plate_X_Dimension"),
            x_unit=self._read_str_locked(ma, "Build_Plate_X_Dimension_unit", "mm"),
            y=self._read_float(ma, "Build_Plate_Y_Dimension"),
            y_unit=self._read_str_locked(ma, "Build_Plate_Y_Dimension_unit", "mm"),
            z=self._read_float(ma, "Build_Plate_Z_Dimension"),
            z_unit=self._read_str_locked(ma, "Build_Plate_Z_Dimension_unit", "mm"),
            corner_radius=self._read_float(ma, "Build_Plate_Corner_Radius"),
            corner_radius_unit=self._read_str_locked(
                ma, "Build_Plate_Corner_Radius_unit", "mm"
            ),
        )
        machine = Machine(
            id=self._read_str(ma, "ID"),
            machine_name=str(ma.get("Machine_Name", "")),
            manufacturer=str(ma.get("Manufacturer", "")),
            model=str(ma.get("Model", "")),
            serial_number=str(ma.get("Serial_Number", "")),
            build_plate=build_plate,
            gas_flow_direction=self._read_str(ma, "Gas_Flow_Direction"),
            recoat_direction=self._read_str(ma, "Recoat_Direction"),
            recoater_blade_type=self._read_str(ma, "Recoater_Blade_Type"),
        )

        trains_grp = f[layout.ROOT_OPTICAL_TRAINS]
        train_ids = layout.train_ids(trains_grp.keys())
        optical_trains = [self._parse_train(f, tid) for tid in train_ids]

        extensions = self._parse_extensions(f)

        return MachineConfig(
            meta=meta,
            machine=machine,
            optical_trains=optical_trains,
            extensions=extensions,
        )

    def _parse_extensions(self, f: h5py.File) -> dict:
        """Every group under ``Extensions/``, verbatim (ClearBox included)."""
        out: dict = {}
        if layout.ROOT_EXTENSIONS not in f:
            return out
        root = f[layout.ROOT_EXTENSIONS]
        for name in root.keys():
            if not isinstance(root[name], h5py.Group):
                raise ValueError(f'HDF5: expected Group at "{layout.ROOT_EXTENSIONS}/{name}"')
            out[name] = read_extension_tree(root[name], name)
        return out

    def _parse_train(self, f: h5py.File, train_id: str) -> OpticalTrain:
        base_path = layout.train_path_by_id(train_id)
        a = f[base_path].attrs

        scanner = self._parse_scanner(f[layout.scanner_path_by_id(train_id)])
        light_source = self._parse_light_source(f[layout.light_source_path_by_id(train_id)])
        collimator = self._parse_collimator(f[layout.collimator_path_by_id(train_id)])
        scanner_card = self._parse_scanner_card(f[layout.scanner_card_path_by_id(train_id)])

        sfcf_path = layout.scan_field_correction_file_path_by_id(train_id)
        sfcf = self._parse_sfcf(f[sfcf_path]) if sfcf_path in f else None

        return OpticalTrain(
            train_id=train_id,
            id=self._read_str(a, "ID"),
            beam_profile_type=self._read_str(a, "Beam_Profile_Type"),
            beam_waist_definition=self._read_str(a, "Beam_Waist_Definition"),
            beam_waist_major=self._read_float(a, "Beam_Waist_Major"),
            beam_waist_major_unit=self._read_str_locked(a, "Beam_Waist_Major_unit", "μm"),
            beam_waist_minor=self._read_float(a, "Beam_Waist_Minor"),
            beam_waist_minor_unit=self._read_str_locked(a, "Beam_Waist_Minor_unit", "μm"),
            beam_waist_offset_z=self._read_float(a, "Beam_Waist_Offset_Z"),
            beam_waist_offset_z_unit=self._read_str_locked(
                a, "Beam_Waist_Offset_Z_unit", "mm"
            ),
            build_plane_offset_major=self._read_float(a, "Build_Plane_Offset_Major"),
            build_plane_offset_major_unit=self._read_str_locked(
                a, "Build_Plane_Offset_Major_unit", "mm"
            ),
            build_plane_offset_minor=self._read_float(a, "Build_Plane_Offset_Minor"),
            build_plane_offset_minor_unit=self._read_str_locked(
                a, "Build_Plane_Offset_Minor_unit", "mm"
            ),
            collimator_focal_length=self._read_float(a, "Collimator_Focal_Length"),
            collimator_focal_length_unit=self._read_str_locked(
                a, "Collimator_Focal_Length_unit", "mm"
            ),
            m2_major=self._read_float(a, "M2_Major"),
            m2_minor=self._read_float(a, "M2_Minor"),
            major_axis_angle=self._read_float(a, "Major_Axis_Angle"),
            major_axis_angle_unit=self._read_str_locked(
                a, "Major_Axis_Angle_unit", "degrees"
            ),
            rayleigh_length_major=self._read_float(a, "Rayleigh_Length_Major"),
            rayleigh_length_major_unit=self._read_str_locked(
                a, "Rayleigh_Length_Major_unit", "mm"
            ),
            rayleigh_length_minor=self._read_float(a, "Rayleigh_Length_Minor"),
            rayleigh_length_minor_unit=self._read_str_locked(
                a, "Rayleigh_Length_Minor_unit", "mm"
            ),
            scanner_number=self._read_str(a, "Scanner_Number"),
            thermal_lensing_passed=self._read_bool_from_int(
                a, "Thermal_Lensing_Test_Passed"
            ),
            thermal_lensing_focal_plane_shift=self._read_float(
                a, "Thermal_Lensing_Focal_Plane_Shift"
            ),
            thermal_lensing_focal_plane_shift_unit=self._read_str_locked(
                a, "Thermal_Lensing_Focal_Plane_Shift_unit", "mm"
            ),
            thermal_lensing_threshold=self._read_float(a, "Thermal_Lensing_Threshold"),
            thermal_lensing_threshold_unit=self._read_str_locked(
                a, "Thermal_Lensing_Threshold_unit", "mm"
            ),
            scanner=scanner,
            light_source=light_source,
            collimator=collimator,
            scanner_card=scanner_card,
            scan_field_correction_file=sfcf,
        )

    def _parse_axis(self, grp: h5py.Group) -> AxisConfig:
        """Change 5: Tuning_Parameters/Tuning_Type have no on-disk source in
        v1.1 — always None here, regardless of what's in the group.
        """
        a = grp.attrs
        return AxisConfig(
            actual_bit_resolution=self._read_int(a, "Actual_Bit_Resolution"),
            actual_bit_resolution_unit=self._read_str(a, "Actual_Bit_Resolution_unit"),
            commanded_bit_resolution=self._read_int(a, "Commanded_Bit_Resolution"),
            commanded_bit_resolution_unit=self._read_str(a, "Commanded_Bit_Resolution_unit"),
            control_type=self._read_str(a, "Control_Type"),
            range_of_motion=self._read_float(a, "Range_Of_Motion"),
            range_of_motion_unit=self._read_str(a, "Range_Of_Motion_unit"),
            smoothing_kernel=self._read_str(a, "Smoothing_Kernel"),
            smoothing_parameters=self._read_float(a, "Smoothing_Parameters"),
            tuning_parameters=None,
            tuning_type=None,
        )

    def _parse_scanner(self, grp: h5py.Group) -> Scanner:
        a = grp.attrs
        axis_cfg = self._read_str(a, "Axis_Configuration")
        x_axis = self._parse_axis(grp["X_Axis"])
        y_axis = self._parse_axis(grp["Y_Axis"])
        z_axis = self._parse_axis(grp["Z_Axis"]) if "Z_Axis" in grp else None
        focus = self._parse_axis(grp["Focus"]) if "Focus" in grp else None
        return Scanner(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=self._read_str(a, "Serial_Number") or "",
            working_distance=self._read_float(a, "Working_Distance"),
            working_distance_unit=self._read_str_locked(
                a, "Working_Distance_unit", "mm"
            ),
            scan_field_x=self._read_float(a, "Scan_Field_Size_X"),
            scan_field_x_unit=self._read_str_locked(a, "Scan_Field_Size_X_unit", "mm"),
            scan_field_y=self._read_float(a, "Scan_Field_Size_Y"),
            scan_field_y_unit=self._read_str_locked(a, "Scan_Field_Size_Y_unit", "mm"),
            scan_field_z=self._read_float(a, "Scan_Field_Size_Z"),
            scan_field_z_unit=self._read_str_locked(a, "Scan_Field_Size_Z_unit", "mm"),
            scan_head_offset_x=self._read_float(a, "Scan_Head_Offset_X"),
            scan_head_offset_x_unit=self._read_str_locked(
                a, "Scan_Head_Offset_X_unit", "mm"
            ),
            scan_head_offset_y=self._read_float(a, "Scan_Head_Offset_Y"),
            scan_head_offset_y_unit=self._read_str_locked(
                a, "Scan_Head_Offset_Y_unit", "mm"
            ),
            scan_head_offset_z=self._read_float(a, "Scan_Head_Offset_Z"),
            scan_head_offset_z_unit=self._read_str_locked(
                a, "Scan_Head_Offset_Z_unit", "mm"
            ),
            scan_head_rotation=self._read_float(a, "Scan_Head_Rotation"),
            scan_head_rotation_unit=self._read_str_locked(
                a, "Scan_Head_Rotation_unit", "degrees"
            ),
            axis_configuration=axis_cfg,
            x_axis=x_axis,
            y_axis=y_axis,
            z_axis=z_axis,
            focus=focus,
            invert_actual_x=self._read_bool_from_int(a, "Invert_Actual_X") or False,
            invert_actual_y=self._read_bool_from_int(a, "Invert_Actual_Y") or False,
            invert_commanded_x=self._read_bool_from_int(a, "Invert_Commanded_X") or False,
            invert_commanded_y=self._read_bool_from_int(a, "Invert_Commanded_Y") or False,
        )

    def _parse_power_characterization(self, grp: h5py.Group) -> PowerCharacterization:
        a = grp.attrs
        derivation_equation_constants: list[EquationConstant] = []
        if layout.DS_DERIVATION_EQUATION_CONSTANTS in grp:
            rows = grp[layout.DS_DERIVATION_EQUATION_CONSTANTS][()]
            derivation_equation_constants = [
                EquationConstant(name=row["name"].decode("utf-8"), value=float(row["value"]))
                for row in rows
            ]
        characterization_points: list[CalibrationPoint] = []
        if layout.DS_CHARACTERIZATION_POINTS in grp:
            rows = grp[layout.DS_CHARACTERIZATION_POINTS][()]
            characterization_points = [
                CalibrationPoint(input_value=float(row["input_value"]), output_value=float(row["output_value"]))
                for row in rows
            ]
        return PowerCharacterization(
            algorithm_type=self._read_str(a, "Algorithm_Type"),
            algorithm_equation=self._read_str(a, "Algorithm_Equation"),
            input_type=self._read_str(a, "Input_Type"),
            units_derived_quantity=self._read_str(a, "Units_Derived_Quantity"),
            derivation_equation_constants=derivation_equation_constants,
            characterization_points=characterization_points,
        )

    def _parse_light_source(self, grp: h5py.Group) -> LightSource:
        a = grp.attrs
        pc_grp = grp.get(layout.GROUP_POWER_CHARACTERIZATION)
        power_characterization = (
            self._parse_power_characterization(pc_grp) if pc_grp is not None else None
        )
        return LightSource(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            wavelength=self._read_float(a, "Light_Wavelength"),
            wavelength_unit=self._read_str_locked(a, "Light_Wavelength_unit", "nm"),
            power_max_nominal=self._read_float(a, "Power_Max_Nominal"),
            power_max_nominal_unit=self._read_str_locked(
                a, "Power_Max_Nominal_unit", "W"
            ),
            power_max_actual=self._read_float(a, "Power_Max_Actual"),
            power_max_actual_unit=self._read_str_locked(
                a, "Power_Max_Actual_unit", "W"
            ),
            power_min_actual=self._read_float(a, "Power_Min_Actual"),
            power_min_actual_unit=self._read_str_locked(
                a, "Power_Min_Actual_unit", "W"
            ),
            power_min_nominal=self._read_float(a, "Power_Min_Nominal"),
            power_min_nominal_unit=self._read_str_locked(
                a, "Power_Min_Nominal_unit", "W"
            ),
            power_bit_resolution=self._read_float(a, "Power_Bit_Resolution"),
            power_bit_resolution_unit=self._read_str_locked(
                a, "Power_Bit_Resolution_unit", "bits"
            ),
            power_characterization=power_characterization,
        )

    def _parse_collimator(self, grp: h5py.Group) -> Collimator:
        a = grp.attrs
        return Collimator(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            focal_length=self._read_float(a, "Focal_Length"),
            focal_length_unit=self._read_str_locked(a, "Focal_Length_unit", "mm"),
        )

    def _parse_scanner_card(self, grp: h5py.Group) -> ScannerCard:
        a = grp.attrs
        return ScannerCard(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            communication_protocol=self._read_str(a, "Communication_Protocol"),
            sample_period=self._read_float(a, "Sample_Period"),
            sample_period_unit=self._read_str_locked(a, "Sample_Period_unit", "μs"),
        )

    def _parse_sfcf(self, ds: h5py.Dataset) -> ScanFieldCorrectionFile:
        a = ds.attrs
        return ScanFieldCorrectionFile(
            document_name=str(a.get("document_name", "")),
            document_id=str(a.get("document_id", "")),
            file_size=int(a.get("file_size", 0)),
            valid_as_of_date=str(a.get("valid_as_of_date", "")),
            document_created_at=self._read_str(a, "document_created_at"),
            document_type=self._read_str(a, "document_type"),
            original_uri=self._read_str(a, "original_uri"),
            raw_bytes=bytes(ds[()]),
        )

    # ------------------------------------------------------------------
    # JSON serialisation helpers — independent of v1.0's, since v1.1's
    # ClearBox/LightSource shape differs (firmware_version, no tuning on
    # AxisConfig). Both this file and v1_0/hdf5.py hydrate
    # power_characterization identically but via their own separate,
    # deliberately-uncoupled dict-helper methods — same "no coupling
    # between version dict-helpers" convention this file already follows
    # for everything else.
    # ------------------------------------------------------------------

    def _config_to_dict(self, config: MachineConfig, include_binary: bool = False) -> dict:
        result = {
            "meta": {
                "schema_version": config.meta.schema_version,
                "machine_name": config.meta.machine_name,
                "manufacturer": config.meta.manufacturer,
                "model": config.meta.model,
                "serial_number": config.meta.serial_number,
                "file_version": config.meta.file_version,
                "export_date": config.meta.export_date,
                "configuration_hash": config.meta.configuration_hash,
                "extra": config.meta.extra,
            },
            "machine": {
                "id": config.machine.id,
                "machine_name": config.machine.machine_name,
                "manufacturer": config.machine.manufacturer,
                "model": config.machine.model,
                "serial_number": config.machine.serial_number,
                "build_plate_x": config.machine.build_plate.x,
                "build_plate_x_unit": config.machine.build_plate.x_unit,
                "build_plate_y": config.machine.build_plate.y,
                "build_plate_y_unit": config.machine.build_plate.y_unit,
                "build_plate_z": config.machine.build_plate.z,
                "build_plate_z_unit": config.machine.build_plate.z_unit,
                "build_plate_radius": config.machine.build_plate.corner_radius,
                "build_plate_radius_unit": config.machine.build_plate.corner_radius_unit,
                "gas_flow_direction": config.machine.gas_flow_direction,
                "recoat_direction": config.machine.recoat_direction,
                "recoater_blade_type": config.machine.recoater_blade_type,
            },
            "optical_trains": [
                self._train_to_dict(t, include_binary=include_binary)
                for t in config.optical_trains
            ],
        }
        if config.extensions:
            result["extensions"] = {
                name: tree_to_json(tree) for name, tree in config.extensions.items()
            }
        return result

    def _train_to_dict(self, train: OpticalTrain, include_binary: bool = False) -> dict:
        return {
            "train_id": train.train_id,
            "id": train.id,
            "beam_profile_type": train.beam_profile_type,
            "beam_waist_definition": train.beam_waist_definition,
            "beam_waist_major": train.beam_waist_major,
            "beam_waist_major_unit": train.beam_waist_major_unit,
            "beam_waist_minor": train.beam_waist_minor,
            "beam_waist_minor_unit": train.beam_waist_minor_unit,
            "beam_waist_offset_z": train.beam_waist_offset_z,
            "beam_waist_offset_z_unit": train.beam_waist_offset_z_unit,
            "build_plane_offset_major": train.build_plane_offset_major,
            "build_plane_offset_major_unit": train.build_plane_offset_major_unit,
            "build_plane_offset_minor": train.build_plane_offset_minor,
            "build_plane_offset_minor_unit": train.build_plane_offset_minor_unit,
            "collimator_focal_length": train.collimator_focal_length,
            "collimator_focal_length_unit": train.collimator_focal_length_unit,
            "m2_major": train.m2_major,
            "m2_minor": train.m2_minor,
            "major_axis_angle": train.major_axis_angle,
            "major_axis_angle_unit": train.major_axis_angle_unit,
            "rayleigh_length_major": train.rayleigh_length_major,
            "rayleigh_length_major_unit": train.rayleigh_length_major_unit,
            "rayleigh_length_minor": train.rayleigh_length_minor,
            "rayleigh_length_minor_unit": train.rayleigh_length_minor_unit,
            "scanner_number": train.scanner_number,
            "thermal_lensing_passed": train.thermal_lensing_passed,
            "thermal_lensing_focal_plane_shift": train.thermal_lensing_focal_plane_shift,
            "thermal_lensing_focal_plane_shift_unit": train.thermal_lensing_focal_plane_shift_unit,
            "thermal_lensing_threshold": train.thermal_lensing_threshold,
            "thermal_lensing_threshold_unit": train.thermal_lensing_threshold_unit,
            "scanner": self._scanner_to_dict(train.scanner),
            "light_source": self._light_source_to_dict(train.light_source),
            "collimator": self._collimator_to_dict(train.collimator),
            "scanner_card": self._scanner_card_to_dict(train.scanner_card),
            "scan_field_correction_file": self._sfcf_to_dict(
                train.scan_field_correction_file, include_binary=include_binary
            ),
        }

    @staticmethod
    def _axis_to_dict(ax: Optional[AxisConfig]) -> Optional[dict]:
        if ax is None:
            return None
        return {
            "actual_bit_resolution": ax.actual_bit_resolution,
            "actual_bit_resolution_unit": ax.actual_bit_resolution_unit,
            "commanded_bit_resolution": ax.commanded_bit_resolution,
            "commanded_bit_resolution_unit": ax.commanded_bit_resolution_unit,
            "control_type": ax.control_type,
            "range_of_motion": ax.range_of_motion,
            "range_of_motion_unit": ax.range_of_motion_unit,
            "smoothing_kernel": ax.smoothing_kernel,
            "smoothing_parameters": ax.smoothing_parameters,
            "tuning_parameters": ax.tuning_parameters,
            "tuning_type": ax.tuning_type,
        }

    def _scanner_to_dict(self, s: Scanner) -> dict:
        d = {
            "manufacturer": s.manufacturer,
            "model": s.model,
            "serial_number": s.serial_number,
            "working_distance": s.working_distance,
            "working_distance_unit": s.working_distance_unit,
            "scan_field_x": s.scan_field_x,
            "scan_field_x_unit": s.scan_field_x_unit,
            "scan_field_y": s.scan_field_y,
            "scan_field_y_unit": s.scan_field_y_unit,
            "scan_field_z": s.scan_field_z,
            "scan_field_z_unit": s.scan_field_z_unit,
            "scan_head_offset_x": s.scan_head_offset_x,
            "scan_head_offset_x_unit": s.scan_head_offset_x_unit,
            "scan_head_offset_y": s.scan_head_offset_y,
            "scan_head_offset_y_unit": s.scan_head_offset_y_unit,
            "scan_head_offset_z": s.scan_head_offset_z,
            "scan_head_offset_z_unit": s.scan_head_offset_z_unit,
            "scan_head_rotation": s.scan_head_rotation,
            "scan_head_rotation_unit": s.scan_head_rotation_unit,
            "axis_configuration": s.axis_configuration,
            "x_axis": self._axis_to_dict(s.x_axis),
            "y_axis": self._axis_to_dict(s.y_axis),
            "z_axis": self._axis_to_dict(s.z_axis),
            "focus": self._axis_to_dict(s.focus),
        }
        if s.invert_actual_x:
            d["invert_actual_x"] = True
        if s.invert_actual_y:
            d["invert_actual_y"] = True
        if s.invert_commanded_x:
            d["invert_commanded_x"] = True
        if s.invert_commanded_y:
            d["invert_commanded_y"] = True
        return d

    @staticmethod
    def _power_characterization_to_dict(pc: Optional[PowerCharacterization]) -> Optional[dict]:
        if pc is None:
            return None
        return {
            "algorithm_type": pc.algorithm_type,
            "algorithm_equation": pc.algorithm_equation,
            "input_type": pc.input_type,
            "units_derived_quantity": pc.units_derived_quantity,
            "derivation_equation_constants": [
                {"name": c.name, "value": c.value} for c in pc.derivation_equation_constants
            ],
            "characterization_points": [
                {"input_value": p.input_value, "output_value": p.output_value}
                for p in pc.characterization_points
            ],
        }

    @staticmethod
    def _power_characterization_from_dict(d: Optional[dict]) -> Optional[PowerCharacterization]:
        if d is None:
            return None
        return PowerCharacterization(
            algorithm_type=d.get("algorithm_type"),
            algorithm_equation=d.get("algorithm_equation"),
            input_type=d.get("input_type"),
            units_derived_quantity=d.get("units_derived_quantity"),
            derivation_equation_constants=[
                EquationConstant(name=c["name"], value=c["value"])
                for c in d.get("derivation_equation_constants", [])
            ],
            characterization_points=[
                CalibrationPoint(input_value=p["input_value"], output_value=p["output_value"])
                for p in d.get("characterization_points", [])
            ],
        )

    def _light_source_to_dict(self, ls: LightSource) -> dict:
        d = {
            "manufacturer": ls.manufacturer,
            "model": ls.model,
            "serial_number": ls.serial_number,
            "wavelength": ls.wavelength,
            "wavelength_unit": ls.wavelength_unit,
            "power_max_nominal": ls.power_max_nominal,
            "power_max_nominal_unit": ls.power_max_nominal_unit,
            "power_max_actual": ls.power_max_actual,
            "power_max_actual_unit": ls.power_max_actual_unit,
            "power_min_actual": ls.power_min_actual,
            "power_min_actual_unit": ls.power_min_actual_unit,
            "power_min_nominal": ls.power_min_nominal,
            "power_min_nominal_unit": ls.power_min_nominal_unit,
            "power_bit_resolution": ls.power_bit_resolution,
            "power_bit_resolution_unit": ls.power_bit_resolution_unit,
        }
        if ls.power_characterization is not None:
            d["power_characterization"] = self._power_characterization_to_dict(
                ls.power_characterization
            )
        return d

    def _collimator_to_dict(self, c: Collimator) -> dict:
        return {
            "manufacturer": c.manufacturer,
            "model": c.model,
            "serial_number": c.serial_number,
            "focal_length": c.focal_length,
            "focal_length_unit": c.focal_length_unit,
        }

    def _scanner_card_to_dict(self, sc: ScannerCard) -> dict:
        return {
            "manufacturer": sc.manufacturer,
            "model": sc.model,
            "serial_number": sc.serial_number,
            "communication_protocol": sc.communication_protocol,
            "sample_period": sc.sample_period,
            "sample_period_unit": sc.sample_period_unit,
        }

    def _sfcf_to_dict(
        self,
        sfcf: Optional[ScanFieldCorrectionFile],
        include_binary: bool = False,
    ) -> Optional[dict]:
        if sfcf is None:
            return None
        d: dict = {
            "document_name": sfcf.document_name,
            "document_id": sfcf.document_id,
            "file_size": sfcf.file_size,
            "valid_as_of_date": sfcf.valid_as_of_date,
            "document_created_at": sfcf.document_created_at,
            "document_type": sfcf.document_type,
            "original_uri": sfcf.original_uri,
        }
        if include_binary:
            d["raw_bytes"] = (
                base64.b64encode(sfcf.raw_bytes).decode("ascii")
                if sfcf.raw_bytes is not None
                else None
            )
        return d

# ---------------------------------------------------------------------------
# JSON → MachineConfig deserialiser (inverse of Hdf5AdapterV1_1._config_to_dict)
# ---------------------------------------------------------------------------

def config_from_dict(d: dict) -> MachineConfig:
    meta_d = d["meta"]
    m = d["machine"]

    meta = MachineConfigMeta(
        schema_version=meta_d.get("schema_version", "v1"),
        machine_name=meta_d["machine_name"],
        manufacturer=meta_d["manufacturer"],
        model=meta_d["model"],
        serial_number=meta_d["serial_number"],
        file_version=meta_d["file_version"],
        export_date=meta_d["export_date"],
        configuration_hash=meta_d["configuration_hash"],
        extra=meta_d.get("extra", {}),
    )

    build_plate = BuildPlate(
        x=m.get("build_plate_x"),
        x_unit=m.get("build_plate_x_unit"),
        y=m.get("build_plate_y"),
        y_unit=m.get("build_plate_y_unit"),
        z=m.get("build_plate_z"),
        z_unit=m.get("build_plate_z_unit"),
        corner_radius=m.get("build_plate_radius"),
        corner_radius_unit=m.get("build_plate_radius_unit"),
    )

    machine = Machine(
        id=m.get("id"),
        machine_name=m["machine_name"],
        manufacturer=m["manufacturer"],
        model=m["model"],
        serial_number=m["serial_number"],
        build_plate=build_plate,
        gas_flow_direction=m.get("gas_flow_direction"),
        recoat_direction=m.get("recoat_direction"),
        recoater_blade_type=m.get("recoater_blade_type"),
    )

    optical_trains = [_train_from_dict(t) for t in d["optical_trains"]]
    extensions = {name: tree_from_json(j) for name, j in d.get("extensions", {}).items()}
    return MachineConfig(
        meta=meta, machine=machine, optical_trains=optical_trains, extensions=extensions
    )


def _axis_from_dict(d: Optional[dict]) -> Optional[AxisConfig]:
    if d is None:
        return None
    return AxisConfig(
        actual_bit_resolution=d.get("actual_bit_resolution"),
        actual_bit_resolution_unit=d.get("actual_bit_resolution_unit"),
        commanded_bit_resolution=d.get("commanded_bit_resolution"),
        commanded_bit_resolution_unit=d.get("commanded_bit_resolution_unit"),
        control_type=d.get("control_type"),
        range_of_motion=d.get("range_of_motion"),
        range_of_motion_unit=d.get("range_of_motion_unit"),
        smoothing_kernel=d.get("smoothing_kernel"),
        smoothing_parameters=d.get("smoothing_parameters"),
        tuning_parameters=d.get("tuning_parameters"),
        tuning_type=d.get("tuning_type"),
    )


def _train_from_dict(t: dict) -> OpticalTrain:
    s = t["scanner"]
    ls = t["light_source"]
    col = t["collimator"]
    sc = t["scanner_card"]
    sfcf_d = t.get("scan_field_correction_file")

    x_axis_d = s.get("x_axis") or {}
    y_axis_d = s.get("y_axis") or {}

    scanner = Scanner(
        manufacturer=s.get("manufacturer", ""),
        model=s.get("model", ""),
        serial_number=s.get("serial_number", ""),
        working_distance=s.get("working_distance"),
        working_distance_unit=s.get("working_distance_unit"),
        scan_field_x=s.get("scan_field_x"),
        scan_field_x_unit=s.get("scan_field_x_unit"),
        scan_field_y=s.get("scan_field_y"),
        scan_field_y_unit=s.get("scan_field_y_unit"),
        scan_field_z=s.get("scan_field_z"),
        scan_field_z_unit=s.get("scan_field_z_unit"),
        scan_head_offset_x=s.get("scan_head_offset_x"),
        scan_head_offset_x_unit=s.get("scan_head_offset_x_unit"),
        scan_head_offset_y=s.get("scan_head_offset_y"),
        scan_head_offset_y_unit=s.get("scan_head_offset_y_unit"),
        scan_head_offset_z=s.get("scan_head_offset_z"),
        scan_head_offset_z_unit=s.get("scan_head_offset_z_unit"),
        scan_head_rotation=s.get("scan_head_rotation"),
        scan_head_rotation_unit=s.get("scan_head_rotation_unit"),
        axis_configuration=s.get("axis_configuration"),
        x_axis=_axis_from_dict(x_axis_d),
        y_axis=_axis_from_dict(y_axis_d),
        z_axis=_axis_from_dict(s.get("z_axis")),
        focus=_axis_from_dict(s.get("focus")),
        invert_actual_x=s.get("invert_actual_x", False),
        invert_actual_y=s.get("invert_actual_y", False),
        invert_commanded_x=s.get("invert_commanded_x", False),
        invert_commanded_y=s.get("invert_commanded_y", False),
    )

    light_source = LightSource(
        manufacturer=ls.get("manufacturer", ""),
        model=ls.get("model", ""),
        serial_number=ls.get("serial_number", ""),
        wavelength=ls.get("wavelength"),
        wavelength_unit=ls.get("wavelength_unit"),
        power_max_nominal=ls.get("power_max_nominal"),
        power_max_nominal_unit=ls.get("power_max_nominal_unit"),
        power_max_actual=ls.get("power_max_actual"),
        power_max_actual_unit=ls.get("power_max_actual_unit"),
        power_min_actual=ls.get("power_min_actual"),
        power_min_actual_unit=ls.get("power_min_actual_unit"),
        power_min_nominal=ls.get("power_min_nominal"),
        power_min_nominal_unit=ls.get("power_min_nominal_unit"),
        power_bit_resolution=ls.get("power_bit_resolution"),
        power_bit_resolution_unit=ls.get("power_bit_resolution_unit"),
        power_characterization=Hdf5AdapterV1_1._power_characterization_from_dict(
            ls.get("power_characterization")
        ),
    )

    collimator = Collimator(
        manufacturer=col.get("manufacturer", ""),
        model=col.get("model", ""),
        serial_number=col.get("serial_number", ""),
        focal_length=col.get("focal_length"),
        focal_length_unit=col.get("focal_length_unit"),
    )

    scanner_card = ScannerCard(
        manufacturer=sc.get("manufacturer", ""),
        model=sc.get("model", ""),
        serial_number=sc.get("serial_number", ""),
        communication_protocol=sc.get("communication_protocol"),
        sample_period=sc.get("sample_period"),
        sample_period_unit=sc.get("sample_period_unit"),
    )

    sfcf: Optional[ScanFieldCorrectionFile] = None
    if sfcf_d is not None:
        raw_b64 = sfcf_d.get("raw_bytes")
        sfcf = ScanFieldCorrectionFile(
            document_name=sfcf_d["document_name"],
            document_id=sfcf_d["document_id"],
            file_size=sfcf_d["file_size"],
            valid_as_of_date=sfcf_d["valid_as_of_date"],
            document_created_at=sfcf_d.get("document_created_at"),
            document_type=sfcf_d.get("document_type"),
            original_uri=sfcf_d.get("original_uri"),
            raw_bytes=base64.b64decode(raw_b64) if raw_b64 is not None else None,
        )

    return OpticalTrain(
        train_id=t["train_id"],
        id=t.get("id"),
        beam_profile_type=t.get("beam_profile_type"),
        beam_waist_definition=t.get("beam_waist_definition"),
        beam_waist_major=t.get("beam_waist_major"),
        beam_waist_major_unit=t.get("beam_waist_major_unit"),
        beam_waist_minor=t.get("beam_waist_minor"),
        beam_waist_minor_unit=t.get("beam_waist_minor_unit"),
        beam_waist_offset_z=t.get("beam_waist_offset_z"),
        beam_waist_offset_z_unit=t.get("beam_waist_offset_z_unit"),
        build_plane_offset_major=t.get("build_plane_offset_major"),
        build_plane_offset_major_unit=t.get("build_plane_offset_major_unit"),
        build_plane_offset_minor=t.get("build_plane_offset_minor"),
        build_plane_offset_minor_unit=t.get("build_plane_offset_minor_unit"),
        collimator_focal_length=t.get("collimator_focal_length"),
        collimator_focal_length_unit=t.get("collimator_focal_length_unit"),
        m2_major=t.get("m2_major"),
        m2_minor=t.get("m2_minor"),
        major_axis_angle=t.get("major_axis_angle"),
        major_axis_angle_unit=t.get("major_axis_angle_unit"),
        rayleigh_length_major=t.get("rayleigh_length_major"),
        rayleigh_length_major_unit=t.get("rayleigh_length_major_unit"),
        rayleigh_length_minor=t.get("rayleigh_length_minor"),
        rayleigh_length_minor_unit=t.get("rayleigh_length_minor_unit"),
        scanner_number=t.get("scanner_number"),
        thermal_lensing_passed=t.get("thermal_lensing_passed"),
        thermal_lensing_focal_plane_shift=t.get("thermal_lensing_focal_plane_shift"),
        thermal_lensing_focal_plane_shift_unit=t.get("thermal_lensing_focal_plane_shift_unit"),
        thermal_lensing_threshold=t.get("thermal_lensing_threshold"),
        thermal_lensing_threshold_unit=t.get("thermal_lensing_threshold_unit"),
        scanner=scanner,
        light_source=light_source,
        collimator=collimator,
        scanner_card=scanner_card,
        scan_field_correction_file=sfcf,
    )


# ---------------------------------------------------------------------------
# Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrate_v1_to_v1_1/
# migrate_v1_1_to_v1 from here — the coefficients/points shape-conversion
# functions they used now live in machine_config.power_characterization
# (imported by capabilities/v1_0/writer.py and this module's own writer.py),
# called as a write-time fallback, not a standalone migration step. See that
# module's docstring for the full design.
# ---------------------------------------------------------------------------
