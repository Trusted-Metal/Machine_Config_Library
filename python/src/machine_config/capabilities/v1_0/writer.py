"""File_Version 1.0 HDF5 writer — on-disk layout, attribute names, and casting.

Public MachineConfigWriter dispatches here after reading File_Version from the model.

Type conventions (mirror reader helpers in reverse):
  None  → ""   (empty string — matches what the machine software writes)
  bool  → 0 or 1  (HDF5 integer attribute)
  float → np.float64
  int   → Python int  (h5py defaults to int64)
  str   → Python str  (h5py writes variable-length UTF-8)
"""
from __future__ import annotations

from pathlib import Path
from typing import Optional

import h5py
import numpy as np

from machine_config.models import (
    AxisConfig,
    Collimator,
    LightSource,
    MachineConfig,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)

from machine_config import clearbox_legacy
from machine_config.power_characterization import backward_flat_fields_points

from machine_config.extension_contract import validate_tree
from machine_config.extensions_hdf5 import write_extension_tree
from . import layout


class Hdf5WriterV1_0:
    """Write a stable :class:`MachineConfig` as a File_Version 1.0 HDF5 file."""

    def __init__(self, config: MachineConfig) -> None:
        self.config = config

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def _validate_extensions(self) -> dict:
        """Runs before the output file is created, so a rejected config never leaves a partial file.

        Returns the ClearBox extension split into one v1.0 group per optical train id.
        """
        for name, tree in self.config.extensions.items():
            validate_tree(tree, name)
            if name not in (layout.LEGACY_OPCUA_EXTENSION_NAME, clearbox_legacy.EXTENSION_NAME):
                raise ValueError(
                    f'File_Version 1.0 has no Extensions group; cannot write extension "{name}"'
                )
        tree = self.config.extensions.get(clearbox_legacy.EXTENSION_NAME)
        groups = clearbox_legacy.to_v1_0_trains(tree) if tree is not None else {}
        known = {layout.train_id(i) for i in range(len(self.config.optical_trains))}
        for train_id in groups:
            if train_id not in known:
                raise ValueError(f'ClearBox: no optical train "{train_id}" in this configuration')
        return groups

    def write(self, path: str | Path) -> None:
        """Write *config* to an HDF5 file at *path* (creates or overwrites)."""
        clearbox_groups = self._validate_extensions()
        with h5py.File(Path(path), "w") as f:
            self._write_root_attrs(f)
            self._write_machine(f)
            self._write_optical_trains(f, clearbox_groups)
            # v1.0 stores the TM_OPCUA extension at the root as ``OPCUA/``.
            tree = self.config.extensions.get(layout.LEGACY_OPCUA_EXTENSION_NAME)
            if tree is not None:
                write_extension_tree(f, layout.ROOT_OPCUA, tree)

    # ------------------------------------------------------------------
    # Type-conversion helpers — exact inverses of reader helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _s(v) -> str:
        """``None`` → ``""``; anything else → ``str(v)``."""
        return str(v) if v is not None else ""

    @staticmethod
    def _f(v):
        """``None`` → ``""``; numeric → ``np.float64``."""
        return np.float64(v) if v is not None else ""

    @staticmethod
    def _i(v):
        """``None`` → ``""``; numeric → ``int``."""
        return int(v) if v is not None else ""

    @staticmethod
    def _b(v):
        """``None`` → ``""``; bool → ``0`` or ``1``."""
        return int(v) if v is not None else ""

    # ------------------------------------------------------------------
    # Root attributes → MachineConfigMeta
    # ------------------------------------------------------------------

    def _write_root_attrs(self, f: h5py.File) -> None:
        meta = self.config.meta
        f.attrs["machine_name"]       = meta.machine_name
        f.attrs["manufacturer"]       = meta.manufacturer
        f.attrs["model"]              = meta.model
        f.attrs["serial_number"]      = meta.serial_number
        f.attrs["File_Version"]       = meta.file_version
        f.attrs["Export_Date"]        = meta.export_date
        f.attrs["Configuration_Hash"] = meta.configuration_hash
        for k, v in meta.extra.items():
            f.attrs[k] = v

    # ------------------------------------------------------------------
    # Machine group → Machine + BuildPlate
    # ------------------------------------------------------------------

    def _write_machine(self, f: h5py.File) -> None:
        grp = f.require_group(layout.ROOT_MACHINE)
        ma  = self.config.machine
        bp  = ma.build_plate
        grp.attrs["ID"]           = self._s(ma.id)
        grp.attrs["Machine_Name"] = ma.machine_name
        grp.attrs["Manufacturer"] = ma.manufacturer
        grp.attrs["Model"]        = ma.model
        grp.attrs["Serial_Number"]= ma.serial_number
        grp.attrs["Build_Plate_X_Dimension"]      = self._f(bp.x)
        grp.attrs["Build_Plate_X_Dimension_unit"] = self._s(bp.x_unit)
        grp.attrs["Build_Plate_Y_Dimension"]      = self._f(bp.y)
        grp.attrs["Build_Plate_Y_Dimension_unit"] = self._s(bp.y_unit)
        grp.attrs["Build_Plate_Z_Dimension"]      = self._f(bp.z)
        grp.attrs["Build_Plate_Z_Dimension_unit"] = self._s(bp.z_unit)
        grp.attrs["Build_Plate_Corner_Radius"]      = self._f(bp.corner_radius)
        grp.attrs["Build_Plate_Corner_Radius_unit"] = self._s(bp.corner_radius_unit)
        grp.attrs["Gas_Flow_Direction"] = self._s(ma.gas_flow_direction)
        grp.attrs["Recoat_Direction"]   = self._s(ma.recoat_direction)
        grp.attrs["Recoater_Blade_Type"] = self._s(ma.recoater_blade_type)
        f.require_group(layout.ROOT_OPTICAL_TRAINS)

    # ------------------------------------------------------------------
    # Optical trains
    # ------------------------------------------------------------------

    def _write_optical_trains(self, f: h5py.File, clearbox_groups: dict) -> None:
        for i, train in enumerate(self.config.optical_trains):
            base = layout.train_path(i)
            grp  = f.require_group(base)
            self._write_train_attrs(grp, train)
            self._write_scanner(f.require_group(f"{base}/{layout.GROUP_SCANNER}"), train.scanner)
            self._write_light_source(f.require_group(f"{base}/{layout.GROUP_LIGHT_SOURCE}"), train.light_source)
            self._write_collimator(f.require_group(f"{base}/{layout.GROUP_COLLIMATOR}"), train.collimator)
            self._write_scanner_card(f.require_group(f"{base}/{layout.GROUP_SCANNER_CARD}"), train.scanner_card)
            clearbox = clearbox_groups.get(layout.train_id(i))
            if clearbox is not None:
                opt = f.require_group(f"{base}/{layout.GROUP_OPTIONAL_COMPONENTS}")
                write_extension_tree(opt, layout.GROUP_CLEARBOX, clearbox)
            if train.scan_field_correction_file is not None:
                self._write_sfcf(f, base, train.scan_field_correction_file)

    def _write_train_attrs(self, grp: h5py.Group, t: OpticalTrain) -> None:
        grp.attrs["ID"]                    = self._s(t.id)
        grp.attrs["Beam_Profile_Type"]     = self._s(t.beam_profile_type)
        grp.attrs["Beam_Waist_Definition"] = self._s(t.beam_waist_definition)
        grp.attrs["Beam_Waist_Major"]      = self._f(t.beam_waist_major)
        grp.attrs["Beam_Waist_Major_unit"] = self._s(t.beam_waist_major_unit)
        grp.attrs["Beam_Waist_Minor"]      = self._f(t.beam_waist_minor)
        grp.attrs["Beam_Waist_Minor_unit"] = self._s(t.beam_waist_minor_unit)
        grp.attrs["Beam_Waist_Offset_Z"]      = self._f(t.beam_waist_offset_z)
        grp.attrs["Beam_Waist_Offset_Z_unit"] = self._s(t.beam_waist_offset_z_unit)
        grp.attrs["Build_Plane_Offset_Major"]      = self._f(t.build_plane_offset_major)
        grp.attrs["Build_Plane_Offset_Major_unit"] = self._s(t.build_plane_offset_major_unit)
        grp.attrs["Build_Plane_Offset_Minor"]      = self._f(t.build_plane_offset_minor)
        grp.attrs["Build_Plane_Offset_Minor_unit"] = self._s(t.build_plane_offset_minor_unit)
        grp.attrs["Collimator_Focal_Length"]      = self._f(t.collimator_focal_length)
        grp.attrs["Collimator_Focal_Length_unit"] = self._s(t.collimator_focal_length_unit)
        grp.attrs["M2_Major"] = self._f(t.m2_major)
        grp.attrs["M2_Minor"] = self._f(t.m2_minor)
        grp.attrs["Major_Axis_Angle"]      = self._f(t.major_axis_angle)
        grp.attrs["Major_Axis_Angle_unit"] = self._s(t.major_axis_angle_unit)
        grp.attrs["Rayleigh_Length_Major"]      = self._f(t.rayleigh_length_major)
        grp.attrs["Rayleigh_Length_Major_unit"] = self._s(t.rayleigh_length_major_unit)
        grp.attrs["Rayleigh_Length_Minor"]      = self._f(t.rayleigh_length_minor)
        grp.attrs["Rayleigh_Length_Minor_unit"] = self._s(t.rayleigh_length_minor_unit)
        grp.attrs["Scanner_Number"]           = self._s(t.scanner_number)
        grp.attrs["Thermal_Lensing_Test_Passed"]        = self._b(t.thermal_lensing_passed)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift"]      = self._f(t.thermal_lensing_focal_plane_shift)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift_unit"] = self._s(t.thermal_lensing_focal_plane_shift_unit)
        grp.attrs["Thermal_Lensing_Threshold"]      = self._f(t.thermal_lensing_threshold)
        grp.attrs["Thermal_Lensing_Threshold_unit"] = self._s(t.thermal_lensing_threshold_unit)

    def _write_scanner(self, grp: h5py.Group, s: Scanner) -> None:
        grp.attrs["Manufacturer"]  = s.manufacturer
        grp.attrs["Model"]         = s.model
        grp.attrs["Serial_Number"] = s.serial_number
        grp.attrs["Working_Distance"]      = self._f(s.working_distance)
        grp.attrs["Working_Distance_unit"] = self._s(s.working_distance_unit)
        grp.attrs["Scan_Field_Size_X"]      = self._f(s.scan_field_x)
        grp.attrs["Scan_Field_Size_X_unit"] = self._s(s.scan_field_x_unit)
        grp.attrs["Scan_Field_Size_Y"]      = self._f(s.scan_field_y)
        grp.attrs["Scan_Field_Size_Y_unit"] = self._s(s.scan_field_y_unit)
        grp.attrs["Scan_Field_Size_Z"]      = self._f(s.scan_field_z)
        grp.attrs["Scan_Field_Size_Z_unit"] = self._s(s.scan_field_z_unit)
        grp.attrs["Scan_Head_Offset_X"]      = self._f(s.scan_head_offset_x)
        grp.attrs["Scan_Head_Offset_X_unit"] = self._s(s.scan_head_offset_x_unit)
        grp.attrs["Scan_Head_Offset_Y"]      = self._f(s.scan_head_offset_y)
        grp.attrs["Scan_Head_Offset_Y_unit"] = self._s(s.scan_head_offset_y_unit)
        grp.attrs["Scan_Head_Offset_Z"]      = self._f(s.scan_head_offset_z)
        grp.attrs["Scan_Head_Offset_Z_unit"] = self._s(s.scan_head_offset_z_unit)
        grp.attrs["Scan_Head_Rotation"]      = self._f(s.scan_head_rotation)
        grp.attrs["Scan_Head_Rotation_unit"] = self._s(s.scan_head_rotation_unit)
        grp.attrs["Axis_Configuration"]      = self._s(s.axis_configuration)
        # Written only when True — never written at all (not even as 0) when
        # False, unlike every other bool attribute (user-confirmed, 2026-08-21).
        if s.invert_actual_x:
            grp.attrs["Invert_Actual_X"] = 1
        if s.invert_actual_y:
            grp.attrs["Invert_Actual_Y"] = 1
        if s.invert_commanded_x:
            grp.attrs["Invert_Commanded_X"] = 1
        if s.invert_commanded_y:
            grp.attrs["Invert_Commanded_Y"] = 1
        self._write_axis(grp.require_group("X_Axis"), s.x_axis)
        self._write_axis(grp.require_group("Y_Axis"), s.y_axis)
        if s.z_axis is not None:
            self._write_axis(grp.require_group("Z_Axis"), s.z_axis)
        if s.focus is not None:
            self._write_axis(grp.require_group("Focus"), s.focus)

    def _write_axis(self, grp: h5py.Group, ax: AxisConfig) -> None:
        grp.attrs["Actual_Bit_Resolution"]      = self._i(ax.actual_bit_resolution)
        grp.attrs["Actual_Bit_Resolution_unit"] = self._s(ax.actual_bit_resolution_unit)
        grp.attrs["Commanded_Bit_Resolution"]      = self._i(ax.commanded_bit_resolution)
        grp.attrs["Commanded_Bit_Resolution_unit"] = self._s(ax.commanded_bit_resolution_unit)
        grp.attrs["Control_Type"]        = self._s(ax.control_type)
        grp.attrs["Range_Of_Motion"]      = self._f(ax.range_of_motion)
        grp.attrs["Range_Of_Motion_unit"] = self._s(ax.range_of_motion_unit)
        grp.attrs["Smoothing_Kernel"]     = self._s(ax.smoothing_kernel)
        grp.attrs["Smoothing_Parameters"] = self._f(ax.smoothing_parameters)
        grp.attrs["Tuning_Parameters"]    = self._s(ax.tuning_parameters)
        grp.attrs["Tuning_Type"]          = self._s(ax.tuning_type)

    def _write_light_source(self, grp: h5py.Group, ls: LightSource) -> None:
        grp.attrs["Manufacturer"]  = ls.manufacturer
        grp.attrs["Model"]         = ls.model
        grp.attrs["Serial_Number"] = ls.serial_number
        grp.attrs["Light_Wavelength"]      = self._f(ls.wavelength)
        grp.attrs["Light_Wavelength_unit"] = self._s(ls.wavelength_unit)
        grp.attrs["Power_Max_Nominal"]      = self._f(ls.power_max_nominal)
        grp.attrs["Power_Max_Nominal_unit"] = self._s(ls.power_max_nominal_unit)
        grp.attrs["Power_Max_Actual"]      = self._f(ls.power_max_actual)
        grp.attrs["Power_Max_Actual_unit"] = self._s(ls.power_max_actual_unit)
        grp.attrs["Power_Min_Actual"]      = self._f(ls.power_min_actual)
        grp.attrs["Power_Min_Actual_unit"] = self._s(ls.power_min_actual_unit)
        grp.attrs["Power_Min_Nominal"]      = self._f(ls.power_min_nominal)
        grp.attrs["Power_Min_Nominal_unit"] = self._s(ls.power_min_nominal_unit)
        # Power_Bit_Resolution is stored as String type in real HDF5 files
        grp.attrs["Power_Bit_Resolution"]      = self._s(ls.power_bit_resolution)
        grp.attrs["Power_Bit_Resolution_unit"] = self._s(ls.power_bit_resolution_unit)
        # power_characterization is the only StableModel representation of
        # this concept now (POWER_CHARACTERIZATION_UNIFICATION_PLAN.md) —
        # always derive the flat v1.0 shape from it, unconditionally.
        algorithm, params = backward_flat_fields_points(ls.power_characterization)
        grp.attrs["Watts_To_Volts_Algorithm"] = self._s(algorithm)
        grp.attrs["Watts_To_Volts_Params"]    = self._s(params)

    def _write_collimator(self, grp: h5py.Group, c: Collimator) -> None:
        grp.attrs["Manufacturer"]  = c.manufacturer
        grp.attrs["Model"]         = c.model
        grp.attrs["Serial_Number"] = c.serial_number
        grp.attrs["Focal_Length"]      = self._f(c.focal_length)
        grp.attrs["Focal_Length_unit"] = self._s(c.focal_length_unit)

    def _write_scanner_card(self, grp: h5py.Group, sc: ScannerCard) -> None:
        grp.attrs["Manufacturer"]  = sc.manufacturer
        grp.attrs["Model"]         = sc.model
        grp.attrs["Serial_Number"] = sc.serial_number
        grp.attrs["Communication_Protocol"] = self._s(sc.communication_protocol)
        grp.attrs["Sample_Period"]      = self._f(sc.sample_period)
        grp.attrs["Sample_Period_unit"] = self._s(sc.sample_period_unit)

    def _write_sfcf(
        self,
        f: h5py.File,
        train_path: str,
        sfcf: ScanFieldCorrectionFile,
    ) -> None:
        if sfcf.raw_bytes is not None:
            data = np.frombuffer(sfcf.raw_bytes, dtype=np.uint8)
        else:
            data = np.zeros(max(sfcf.file_size, 1), dtype=np.uint8)
        ds = f.create_dataset(f"{train_path}/{layout.DS_SCAN_FIELD_CORRECTION_FILE}", data=data)
        ds.attrs["document_name"]    = sfcf.document_name
        ds.attrs["document_id"]      = sfcf.document_id
        ds.attrs["file_size"]        = sfcf.file_size
        ds.attrs["valid_as_of_date"] = sfcf.valid_as_of_date
        ds.attrs["document_created_at"] = self._s(sfcf.document_created_at)
        ds.attrs["document_type"]       = self._s(sfcf.document_type)
        ds.attrs["original_uri"]        = self._s(sfcf.original_uri)

