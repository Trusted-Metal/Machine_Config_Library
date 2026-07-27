"""Phase 1.5 — HDF5 writer: serialises a MachineConfig back to a machine-config .h5 file.

The writer is machine-agnostic. It produces an HDF5 file whose structure and attribute names
follow the MachineConfig schema — any machine whose software can read that schema can consume
the output. The reference implementation is validated against AconityMIDI fixtures, but no
vendor-specific logic is encoded here.

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

from .models import (
    AxisConfig,
    ClearBox,
    Collimator,
    LightSource,
    MachineConfig,
    OpcuaConfig,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)


class MachineConfigWriter:
    """Write a :class:`MachineConfig` to a machine-config HDF5 file."""

    def __init__(self, config: MachineConfig) -> None:
        self.config = config

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def write(self, path: str | Path) -> None:
        """Write *config* to an HDF5 file at *path* (creates or overwrites)."""
        with h5py.File(Path(path), "w") as f:
            self._write_root_attrs(f)
            self._write_machine(f)
            self._write_optical_trains(f)
            if self.config.opcua is not None:
                self._write_opcua(f, self.config.opcua)

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
        grp = f.require_group("Machine")
        ma  = self.config.machine
        bp  = ma.build_plate
        grp.attrs["ID"]           = self._s(ma.id)
        grp.attrs["Machine_Name"] = ma.machine_name
        grp.attrs["Manufacturer"] = ma.manufacturer
        grp.attrs["Model"]        = ma.model
        grp.attrs["Serial_Number"]= ma.serial_number
        grp.attrs["Build_Plate_X_Dimension"]      = self._f(bp.x)
        grp.attrs["Build_Plate_X_Dimension_unit"] = bp.x_unit or "mm"
        grp.attrs["Build_Plate_Y_Dimension"]      = self._f(bp.y)
        grp.attrs["Build_Plate_Y_Dimension_unit"] = bp.y_unit or "mm"
        grp.attrs["Build_Plate_Z_Dimension"]      = self._f(bp.z)
        grp.attrs["Build_Plate_Z_Dimension_unit"] = bp.z_unit or "mm"
        grp.attrs["Build_Plate_Corner_Radius"]      = self._f(bp.corner_radius)
        grp.attrs["Build_Plate_Corner_Radius_unit"] = bp.corner_radius_unit or "mm"
        grp.attrs["Gas_Flow_Direction"] = self._s(ma.gas_flow_direction)
        grp.attrs["Recoat_Direction"]   = self._s(ma.recoat_direction)
        f.require_group("Machine/Optical_Trains")

    # ------------------------------------------------------------------
    # Optical trains
    # ------------------------------------------------------------------

    def _write_optical_trains(self, f: h5py.File) -> None:
        for i, train in enumerate(self.config.optical_trains):
            tid  = f"Optical_Train_{i + 1:02d}"
            base = f"Machine/Optical_Trains/{tid}"
            grp  = f.require_group(base)
            self._write_train_attrs(grp, train)
            self._write_scanner(f.require_group(f"{base}/Scanner"), train.scanner)
            self._write_light_source(f.require_group(f"{base}/Light_Source"), train.light_source)
            self._write_collimator(f.require_group(f"{base}/Collimator"), train.collimator)
            self._write_scanner_card(f.require_group(f"{base}/Scanner_Card"), train.scanner_card)
            if train.clearbox is not None:
                opt = f.require_group(f"{base}/Optional_Components")
                self._write_clearbox(opt.require_group("ClearBox"), train.clearbox)
            if train.scan_field_correction_file is not None:
                self._write_sfcf(f, base, train.scan_field_correction_file)

    def _write_train_attrs(self, grp: h5py.Group, t: OpticalTrain) -> None:
        grp.attrs["ID"]                    = self._s(t.id)
        grp.attrs["Beam_Profile_Type"]     = self._s(t.beam_profile_type)
        grp.attrs["Beam_Waist_Definition"] = self._s(t.beam_waist_definition)
        grp.attrs["Beam_Waist_Major"]      = self._f(t.beam_waist_major)
        grp.attrs["Beam_Waist_Major_unit"] = t.beam_waist_major_unit or "μm"
        grp.attrs["Beam_Waist_Minor"]      = self._f(t.beam_waist_minor)
        grp.attrs["Beam_Waist_Minor_unit"] = t.beam_waist_minor_unit or "μm"
        grp.attrs["Beam_Waist_Offset_Z"]      = self._f(t.beam_waist_offset_z)
        grp.attrs["Beam_Waist_Offset_Z_unit"] = t.beam_waist_offset_z_unit or "mm"
        grp.attrs["Build_Plane_Offset_Major"]      = self._f(t.build_plane_offset_major)
        grp.attrs["Build_Plane_Offset_Major_unit"] = t.build_plane_offset_major_unit or "mm"
        grp.attrs["Build_Plane_Offset_Minor"]      = self._f(t.build_plane_offset_minor)
        grp.attrs["Build_Plane_Offset_Minor_unit"] = t.build_plane_offset_minor_unit or "mm"
        grp.attrs["Collimator_Focal_Length"]      = self._f(t.collimator_focal_length)
        grp.attrs["Collimator_Focal_Length_unit"] = t.collimator_focal_length_unit or "mm"
        grp.attrs["M2_Major"] = self._f(t.m2_major)
        grp.attrs["M2_Minor"] = self._f(t.m2_minor)
        grp.attrs["Major_Axis_Angle"]      = self._f(t.major_axis_angle)
        grp.attrs["Major_Axis_Angle_unit"] = t.major_axis_angle_unit or "degrees"
        grp.attrs["Rayleigh_Length_Major"]      = self._f(t.rayleigh_length_major)
        grp.attrs["Rayleigh_Length_Major_unit"] = t.rayleigh_length_major_unit or "mm"
        grp.attrs["Rayleigh_Length_Minor"]      = self._f(t.rayleigh_length_minor)
        grp.attrs["Rayleigh_Length_Minor_unit"] = t.rayleigh_length_minor_unit or "mm"
        grp.attrs["Scanner_Number"]           = self._s(t.scanner_number)
        grp.attrs["Thermal_Lensing_Test_Passed"]        = self._b(t.thermal_lensing_passed)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift"]      = self._f(t.thermal_lensing_focal_plane_shift)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift_unit"] = t.thermal_lensing_focal_plane_shift_unit or "mm"
        grp.attrs["Thermal_Lensing_Threshold"]      = self._f(t.thermal_lensing_threshold)
        grp.attrs["Thermal_Lensing_Threshold_unit"] = t.thermal_lensing_threshold_unit or "mm"

    def _write_scanner(self, grp: h5py.Group, s: Scanner) -> None:
        grp.attrs["Manufacturer"]  = s.manufacturer
        grp.attrs["Model"]         = s.model
        grp.attrs["Serial_Number"] = s.serial_number
        grp.attrs["Working_Distance"]      = self._f(s.working_distance)
        grp.attrs["Working_Distance_unit"] = s.working_distance_unit or "mm"
        grp.attrs["Scan_Field_Size_X"]      = self._f(s.scan_field_x)
        grp.attrs["Scan_Field_Size_X_unit"] = s.scan_field_x_unit or "mm"
        grp.attrs["Scan_Field_Size_Y"]      = self._f(s.scan_field_y)
        grp.attrs["Scan_Field_Size_Y_unit"] = s.scan_field_y_unit or "mm"
        grp.attrs["Scan_Field_Size_Z"]      = self._f(s.scan_field_z)
        grp.attrs["Scan_Field_Size_Z_unit"] = s.scan_field_z_unit or "mm"
        grp.attrs["Scan_Head_Offset_X"]      = self._f(s.scan_head_offset_x)
        grp.attrs["Scan_Head_Offset_X_unit"] = s.scan_head_offset_x_unit or "mm"
        grp.attrs["Scan_Head_Offset_Y"]      = self._f(s.scan_head_offset_y)
        grp.attrs["Scan_Head_Offset_Y_unit"] = s.scan_head_offset_y_unit or "mm"
        grp.attrs["Scan_Head_Offset_Z"]      = self._f(s.scan_head_offset_z)
        grp.attrs["Scan_Head_Offset_Z_unit"] = s.scan_head_offset_z_unit or "mm"
        grp.attrs["Scan_Head_Rotation"]      = self._f(s.scan_head_rotation)
        grp.attrs["Scan_Head_Rotation_unit"] = s.scan_head_rotation_unit or "degrees"
        grp.attrs["Axis_Configuration"]      = self._s(s.axis_configuration)
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
        grp.attrs["Light_Wavelength_unit"] = ls.wavelength_unit or "nm"
        grp.attrs["Power_Max_Nominal"]      = self._f(ls.power_max_nominal)
        grp.attrs["Power_Max_Nominal_unit"] = ls.power_max_nominal_unit or "W"
        grp.attrs["Power_Max_Actual"]      = self._f(ls.power_max_actual)
        grp.attrs["Power_Max_Actual_unit"] = ls.power_max_actual_unit or "W"
        grp.attrs["Power_Min_Actual"]      = self._f(ls.power_min_actual)
        grp.attrs["Power_Min_Actual_unit"] = ls.power_min_actual_unit or "W"
        grp.attrs["Power_Min_Nominal"]      = self._f(ls.power_min_nominal)
        grp.attrs["Power_Min_Nominal_unit"] = ls.power_min_nominal_unit or "W"
        # Power_Bit_Resolution is stored as String type in real HDF5 files
        grp.attrs["Power_Bit_Resolution"]      = self._s(ls.power_bit_resolution)
        grp.attrs["Power_Bit_Resolution_unit"] = ls.power_bit_resolution_unit or "bits"
        grp.attrs["Watts_To_Volts_Algorithm"] = self._s(ls.watts_to_volts_algorithm)
        grp.attrs["Watts_To_Volts_Params"]    = self._s(ls.watts_to_volts_params)

    def _write_collimator(self, grp: h5py.Group, c: Collimator) -> None:
        grp.attrs["Manufacturer"]  = c.manufacturer
        grp.attrs["Model"]         = c.model
        grp.attrs["Serial_Number"] = c.serial_number
        grp.attrs["Focal_Length"]      = self._f(c.focal_length)
        grp.attrs["Focal_Length_unit"] = c.focal_length_unit or "mm"

    def _write_scanner_card(self, grp: h5py.Group, sc: ScannerCard) -> None:
        grp.attrs["Manufacturer"]  = sc.manufacturer
        grp.attrs["Model"]         = sc.model
        grp.attrs["Serial_Number"] = sc.serial_number
        grp.attrs["Communication_Protocol"] = self._s(sc.communication_protocol)
        grp.attrs["Sample_Period"]      = self._f(sc.sample_period)
        grp.attrs["Sample_Period_unit"] = sc.sample_period_unit or "μs"

    @staticmethod
    def _correction_list_to_array(data: Optional[list], shape: tuple = (257, 257, 2)) -> np.ndarray:
        """Convert nested Python list (None = NaN) back to float64 ndarray."""
        if data is None:
            return np.zeros(shape, dtype=np.float64)
        obj = np.array(data, dtype=object)
        result = np.empty(obj.shape, dtype=np.float64)
        result.flat[:] = [
            float("nan") if v is None else float(v) for v in obj.flat
        ]
        return result

    def _write_clearbox(self, grp: h5py.Group, cb: ClearBox) -> None:
        grp.attrs["Ip_Address"]           = cb.ip_address
        grp.attrs["Serial_Number"]        = self._s(cb.serial_number)
        grp.attrs["Data_Port"]            = self._i(cb.data_port)
        grp.attrs["Server_Port"]          = self._i(cb.server_port)
        grp.attrs["Actual_Timing_Offset"]    = self._i(cb.actual_timing_offset)
        grp.attrs["Commanded_Timing_Offset"] = self._i(cb.commanded_timing_offset)
        grp.attrs["Manufacturer"]         = self._s(cb.manufacturer)
        grp.attrs["Model"]                = self._s(cb.model)
        grp.attrs["Output_Path"]          = self._s(cb.output_path)
        grp.attrs["Selected_Camera"]      = self._s(cb.selected_camera)
        grp.attrs["Custom_Video_Format"]  = self._s(cb.custom_video_format)
        grp.attrs["Video_Output"]         = self._s(cb.video_output)
        grp.attrs["Show_Console"]         = self._b(cb.show_console)
        grp.attrs["Software_Trigger_Delay"]    = self._i(cb.software_trigger_delay)
        grp.attrs["Volts_To_Watts_Algorithm"]  = self._s(cb.volts_to_watts_algorithm)
        grp.attrs["Volts_To_Watts_Params"]     = self._s(cb.volts_to_watts_params)
        grp.attrs["Correction_Grid_Domain_Shape"]  = self._s(cb.correction_grid_domain_shape)
        grp.attrs["Inverse_Grid_Domain_Shape"]     = self._s(cb.inverse_grid_domain_shape)
        grp.create_dataset(
            "Correction_Data",
            data=self._correction_list_to_array(cb.correction_data),
        )
        cds = grp["Correction_Data"]
        cds.attrs["dimensions"] = "H,W,D"
        cds.attrs["dtype"]      = "float64"
        cds.attrs["shape"]      = f"{cds.shape[0]}x{cds.shape[1]}x{cds.shape[2]}"
        grp.create_dataset(
            "Inverse_Correction_Data",
            data=self._correction_list_to_array(cb.inverse_correction_data),
        )
        ids = grp["Inverse_Correction_Data"]
        ids.attrs["dimensions"] = "H,W,D"
        ids.attrs["dtype"]      = "float64"
        ids.attrs["shape"]      = f"{ids.shape[0]}x{ids.shape[1]}x{ids.shape[2]}"

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
        ds = f.create_dataset(f"{train_path}/scan_field_correction_file", data=data)
        ds.attrs["document_name"]    = sfcf.document_name
        ds.attrs["document_id"]      = sfcf.document_id
        ds.attrs["file_size"]        = sfcf.file_size
        ds.attrs["valid_as_of_date"] = sfcf.valid_as_of_date
        ds.attrs["document_created_at"] = self._s(sfcf.document_created_at)
        ds.attrs["document_type"]       = self._s(sfcf.document_type)
        ds.attrs["original_uri"]        = self._s(sfcf.original_uri)

    def _write_opcua(self, f: h5py.File, opcua: OpcuaConfig) -> None:
        # ---- Client --------------------------------------------------------------
        cg = f.require_group("OPCUA/Client")
        c  = opcua.client
        cg.attrs["Server_URL"]        = c.server_url
        cg.attrs["Auth_Mode"]         = c.auth_mode
        cg.attrs["Security_Mode"]     = c.security_mode
        cg.attrs["Security_Policy"]   = c.security_policy
        cg.attrs["BFS_Max_Depth"]     = int(c.bfs_max_depth)
        cg.attrs["Publish_Interval"]  = int(c.publish_interval)
        cg.attrs["Sampling_Interval"] = int(c.sampling_interval)
        cg.attrs["Session_Timeout"]   = int(c.session_timeout)
        for k, v in c.extra.items():
            cg.attrs[k] = v

        # ---- Pipe ----------------------------------------------------------------
        pg = f.require_group("OPCUA/Pipe")
        p  = opcua.pipe
        pg.attrs["Pipe_Enabled"] = 1 if p.pipe_enabled else 0
        pg.attrs["Buffer_Size"]  = int(p.buffer_size)
        for k, v in p.extra.items():
            pg.attrs[k] = v

        # ---- Triggers ------------------------------------------------------------
        tg = f.require_group("OPCUA/Triggers")
        if opcua.triggers_enabled is not None:
            tg.attrs["Triggers_Enabled"] = np.float64(1.0 if opcua.triggers_enabled else 0.0)

        for name, trigger in opcua.triggers.items():
            sg = tg.require_group(name)
            sg.attrs["ID"]           = self._s(trigger.id)
            sg.attrs["Signal"]       = self._s(trigger.signal)
            sg.attrs["Subsystem"]    = self._s(trigger.subsystem)
            sg.attrs["Rule_Enabled"] = self._b(trigger.rule_enabled)
            sg.attrs["Start_Value"]  = self._s(trigger.start_value)
            sg.attrs["Stop_Value"]   = self._s(trigger.stop_value)
            for k, v in trigger.extra.items():
                sg.attrs[k] = v

