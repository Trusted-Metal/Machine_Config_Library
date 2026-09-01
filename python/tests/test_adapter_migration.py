"""Adapter migration tests.

Proves the StableModel contract holds across adapter version boundaries.
MockV1_1 introduces all five change categories relative to v1.0:

  Addition  (×2): root attrs Facility_ID, Config_Author → meta.extra
  Removal   (×2): Machine/ drops Gas_Flow_Direction, Recoat_Direction → None
  Name      (×2): Machine/ "Machine_Name"   → "Machine_Label"
                  Scanner/ "Working_Distance" → "Focal_Distance"
  Path      (×2): Build_Plate_Corner_Radius, Build_Plate_Z_Dimension
                  move from Machine/ to Machine/Dimensions/
  Name+path (×2): Build_Plate_X_Dimension → Machine/Dimensions/ "Width"
                  Build_Plate_Y_Dimension → Machine/Dimensions/ "Height"

Tests
-----
  test_v1_1_read              mock v1.1 file → StableModel — all categories asserted
  test_v1_1_roundtrip         mock v1.1 → StableModel → mock v1.1 → StableModel
  test_v1_to_v1_1             real v1.0 fixture → StableModel → mock v1.1 layout
  test_v1_1_to_v1             mock v1.1 file → StableModel → v1.0 layout
  test_v1_unaffected          existing v1.0 read/write unchanged (no mock involved)
  test_dispatcher_v1_to_v1_1  full public API via monkeypatched _ADAPTERS (v1.1 → v1.0)
  test_dispatcher_v1_1_to_v1  full public API via monkeypatched _ADAPTERS (v1.0 → v1.1)
"""
from __future__ import annotations

from dataclasses import replace as dc_replace
from pathlib import Path

import h5py
import numpy as np
import pytest

import machine_config.reader as _reader_mod
import machine_config.writer as _writer_mod
from machine_config import MachineConfigReader, MachineConfigWriter
from machine_config.reader import ReaderAdapter
from machine_config.writer import WriterAdapter
from machine_config.models import (
    AxisConfig,
    BuildPlate,
    CalibrationPoint,
    ClearBox,
    Collimator,
    EquationConstant,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    OptionalComponents,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
    SynchronousSensor,
    nan_array_to_nested,
    nested_to_array,
)
from machine_config.schema import SCHEMA_VERSION

_REPO_ROOT = Path(__file__).parent.parent.parent
_REFERENCE_H5 = _REPO_ROOT / "fixtures" / "reference_config.h5"

# ---------------------------------------------------------------------------
# Type-conversion helpers and compound dtypes — this mock is deliberately
# self-contained (no import from capabilities.v1_0): it exists purely to
# verify the adapter-migration *architecture* (dispatcher, ReaderAdapter/
# WriterAdapter protocols), and depending on a real version's adapter for
# "unchanged" pieces would defeat that — a real File_Version could then
# never change or be removed without checking every mock (and, by the same
# logic, every later real version) that quietly leaned on it.
# ---------------------------------------------------------------------------

EQUATION_CONSTANT_NAME_MAX_BYTES = 64
_EQUATION_CONSTANT_DTYPE = np.dtype(
    [
        ("name", h5py.string_dtype(encoding="utf-8", length=EQUATION_CONSTANT_NAME_MAX_BYTES)),
        ("value", "f8"),
    ]
)
_CALIBRATION_POINT_DTYPE = np.dtype([("input_value", "f8"), ("output_value", "f8")])


def _read_str(attrs, key: str):
    val = attrs.get(key)
    if val is None:
        return None
    s = str(val).strip()
    return s if s else None


def _read_float(attrs, key: str):
    val = attrs.get(key)
    if val is None:
        return None
    if isinstance(val, (str, bytes)) and str(val).strip() == "":
        return None
    return float(val)


def _read_int(attrs, key: str):
    val = attrs.get(key)
    if val is None:
        return None
    if isinstance(val, (str, bytes)) and str(val).strip() == "":
        return None
    return int(val)


def _read_bool_from_int(attrs, key: str):
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
    raise ValueError(f"Attribute '{key}' has value {i!r}; expected 0 or 1 (Rule 8).")


def _read_str_locked(attrs, unit_key: str, expected_unit: str):
    val = attrs.get(unit_key)
    if val is None:
        return None
    s = str(val).strip()
    if not s:
        return None
    if s != expected_unit:
        raise ValueError(
            f"Unit attribute '{unit_key}' has value {s!r}; expected {expected_unit!r}."
        )
    return s


def _s(v) -> str:
    return str(v) if v is not None else ""


def _f(v):
    return np.float64(v) if v is not None else ""


def _i(v):
    return int(v) if v is not None else ""


def _b(v):
    return int(v) if v is not None else ""


# ---------------------------------------------------------------------------
# MockV1_1Layout — on-disk constants that differ from v1.0
# ---------------------------------------------------------------------------

class MockV1_1Layout:
    FILE_VERSION = "1.1-mock"

    # ADDITION (×2): new root-level HDF5 attrs that map to typed StableModel fields
    ATTR_FACILITY_ID   = "Facility_ID"
    ATTR_CONFIG_AUTHOR = "Config_Author"

    # NAME CHANGE (×2): same group, renamed attr key
    ATTR_MACHINE_LABEL  = "Machine_Label"   # was Machine/ "Machine_Name"
    ATTR_FOCAL_DISTANCE = "Focal_Distance"  # was Scanner/ "Working_Distance"

    # PATH CHANGE (×2): Build_Plate_Corner_Radius and Build_Plate_Z_Dimension
    # move from Machine/ to Machine/Dimensions/ with the same attr names.
    #
    # NAME+PATH CHANGE (×2): x and y move to Machine/Dimensions/ with new names.
    SUBGROUP_DIMENSIONS = "Machine/Dimensions"
    ATTR_BP_WIDTH  = "Width"   # was Machine/ "Build_Plate_X_Dimension"
    ATTR_BP_HEIGHT = "Height"  # was Machine/ "Build_Plate_Y_Dimension"

    # REMOVAL (×2): Machine/ no longer contains these attrs
    # "Gas_Flow_Direction", "Recoat_Direction"

    # Unchanged group paths
    ROOT_MACHINE        = "Machine"
    ROOT_OPTICAL_TRAINS = "Machine/Optical_Trains"
    TRAIN_ID_PREFIX     = "Optical_Train_"
    GROUP_SCANNER       = "Scanner"
    GROUP_LIGHT_SOURCE  = "Light_Source"
    GROUP_COLLIMATOR    = "Collimator"
    GROUP_SCANNER_CARD  = "Scanner_Card"
    GROUP_OPTIONAL_COMPONENTS      = "Optional_Components"
    GROUP_CLEARBOX                 = "ClearBox"
    DS_SCAN_FIELD_CORRECTION_FILE  = "scan_field_correction_file"


L = MockV1_1Layout  # shorthand used throughout this module


# ---------------------------------------------------------------------------
# MockV1_1Reader
# ---------------------------------------------------------------------------

class MockV1_1Reader:
    """Read a mock v1.1 HDF5 file and return a stable MachineConfig.

    Fully self-contained — does not import or call into
    ``capabilities.v1_0``, even for subcomponents (Light_Source, Collimator,
    Scanner_Card, ClearBox, sfcf) whose shape happens to be unchanged from
    v1.0 today. See the module-level note above this class for why.
    """

    def __init__(self, path) -> None:
        self.path = Path(path)

    def parse(self) -> MachineConfig:
        with h5py.File(self.path, "r") as f:
            return self._parse(f)

    def _parse(self, f: h5py.File) -> MachineConfig:
        # ADDITION (×2): typed fields read from dedicated HDF5 attrs; empty string → None
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=str(f.attrs.get("machine_name", "")),
            manufacturer=str(f.attrs.get("manufacturer", "")),
            model=str(f.attrs.get("model", "")),
            serial_number=str(f.attrs.get("serial_number", "")),
            file_version=str(f.attrs.get("File_Version", "")).strip() or "1.1-mock",
            export_date=str(f.attrs.get("Export_Date", "")),
            configuration_hash=str(f.attrs.get("Configuration_Hash", "")),
            facility_id=str(f.attrs.get(L.ATTR_FACILITY_ID, "")).strip() or None,
            config_author=str(f.attrs.get(L.ATTR_CONFIG_AUTHOR, "")).strip() or None,
        )

        ma   = f[L.ROOT_MACHINE].attrs
        dims = f[L.SUBGROUP_DIMENSIONS].attrs

        build_plate = BuildPlate(
            x=_read_float(dims, L.ATTR_BP_WIDTH),                  # name+path change
            x_unit=_read_str(ma, "Build_Plate_X_Dimension_unit"),
            y=_read_float(dims, L.ATTR_BP_HEIGHT),                  # name+path change
            y_unit=_read_str(ma, "Build_Plate_Y_Dimension_unit"),
            z=_read_float(dims, "Build_Plate_Z_Dimension"),         # path change
            z_unit=_read_str(ma, "Build_Plate_Z_Dimension_unit"),
            corner_radius=_read_float(dims, "Build_Plate_Corner_Radius"),  # path change
            corner_radius_unit=_read_str(ma, "Build_Plate_Corner_Radius_unit"),
        )

        machine = Machine(
            id=_read_str(ma, "ID"),
            machine_name=str(ma.get(L.ATTR_MACHINE_LABEL, "")),  # name change
            manufacturer=str(ma.get("Manufacturer", "")),
            model=str(ma.get("Model", "")),
            serial_number=str(ma.get("Serial_Number", "")),
            build_plate=build_plate,
            gas_flow_direction=None,  # removed in v1.1
            recoat_direction=None,    # removed in v1.1
        )

        trains_grp = f[L.ROOT_OPTICAL_TRAINS]
        train_keys = sorted(k for k in trains_grp.keys() if k.startswith(L.TRAIN_ID_PREFIX))
        optical_trains = [self._parse_train(f, tid) for tid in train_keys]

        return MachineConfig(
            meta=meta,
            machine=machine,
            optical_trains=optical_trains,
            opcua=None,  # OPCUA layout unchanged; not needed for migration test scope
        )

    def _parse_train(self, f: h5py.File, tid: str) -> OpticalTrain:
        base = f"{L.ROOT_OPTICAL_TRAINS}/{tid}"
        a    = f[base].attrs

        light_source = self._parse_light_source(f[f"{base}/{L.GROUP_LIGHT_SOURCE}"])
        collimator   = self._parse_collimator(f[f"{base}/{L.GROUP_COLLIMATOR}"])
        scanner_card = self._parse_scanner_card(f[f"{base}/{L.GROUP_SCANNER_CARD}"])
        scanner      = self._parse_scanner(f[f"{base}/{L.GROUP_SCANNER}"])

        cb_path   = f"{base}/{L.GROUP_OPTIONAL_COMPONENTS}/{L.GROUP_CLEARBOX}"
        clearbox  = self._parse_clearbox(f[cb_path]) if cb_path in f else None
        sfcf_path = f"{base}/{L.DS_SCAN_FIELD_CORRECTION_FILE}"
        sfcf      = self._parse_sfcf(f[sfcf_path]) if sfcf_path in f else None

        return OpticalTrain(
            train_id=tid,
            id=_read_str(a, "ID"),
            beam_profile_type=_read_str(a, "Beam_Profile_Type"),
            beam_waist_definition=_read_str(a, "Beam_Waist_Definition"),
            beam_waist_major=_read_float(a, "Beam_Waist_Major"),
            beam_waist_major_unit=_read_str_locked(a, "Beam_Waist_Major_unit", "μm"),
            beam_waist_minor=_read_float(a, "Beam_Waist_Minor"),
            beam_waist_minor_unit=_read_str_locked(a, "Beam_Waist_Minor_unit", "μm"),
            beam_waist_offset_z=_read_float(a, "Beam_Waist_Offset_Z"),
            beam_waist_offset_z_unit=_read_str_locked(a, "Beam_Waist_Offset_Z_unit", "mm"),
            build_plane_offset_major=_read_float(a, "Build_Plane_Offset_Major"),
            build_plane_offset_major_unit=_read_str_locked(a, "Build_Plane_Offset_Major_unit", "mm"),
            build_plane_offset_minor=_read_float(a, "Build_Plane_Offset_Minor"),
            build_plane_offset_minor_unit=_read_str_locked(a, "Build_Plane_Offset_Minor_unit", "mm"),
            collimator_focal_length=_read_float(a, "Collimator_Focal_Length"),
            collimator_focal_length_unit=_read_str_locked(a, "Collimator_Focal_Length_unit", "mm"),
            m2_major=_read_float(a, "M2_Major"),
            m2_minor=_read_float(a, "M2_Minor"),
            major_axis_angle=_read_float(a, "Major_Axis_Angle"),
            major_axis_angle_unit=_read_str_locked(a, "Major_Axis_Angle_unit", "degrees"),
            rayleigh_length_major=_read_float(a, "Rayleigh_Length_Major"),
            rayleigh_length_major_unit=_read_str_locked(a, "Rayleigh_Length_Major_unit", "mm"),
            rayleigh_length_minor=_read_float(a, "Rayleigh_Length_Minor"),
            rayleigh_length_minor_unit=_read_str_locked(a, "Rayleigh_Length_Minor_unit", "mm"),
            scanner_number=_read_str(a, "Scanner_Number"),
            thermal_lensing_passed=_read_bool_from_int(a, "Thermal_Lensing_Test_Passed"),
            thermal_lensing_focal_plane_shift=_read_float(a, "Thermal_Lensing_Focal_Plane_Shift"),
            thermal_lensing_focal_plane_shift_unit=_read_str_locked(a, "Thermal_Lensing_Focal_Plane_Shift_unit", "mm"),
            thermal_lensing_threshold=_read_float(a, "Thermal_Lensing_Threshold"),
            thermal_lensing_threshold_unit=_read_str_locked(a, "Thermal_Lensing_Threshold_unit", "mm"),
            scanner=scanner,
            light_source=light_source,
            collimator=collimator,
            scanner_card=scanner_card,
            optional_components=OptionalComponents(clearbox=clearbox),
            scan_field_correction_file=sfcf,
        )

    def _parse_scanner(self, grp: h5py.Group) -> Scanner:
        a        = grp.attrs
        axis_cfg = _read_str(a, "Axis_Configuration")
        x_axis   = self._parse_axis(grp["X_Axis"])
        y_axis   = self._parse_axis(grp["Y_Axis"])
        z_axis   = self._parse_axis(grp["Z_Axis"]) if "Z_Axis" in grp else None
        focus    = self._parse_axis(grp["Focus"])  if "Focus"  in grp else None
        return Scanner(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=_read_str(a, "Serial_Number") or "",
            working_distance=_read_float(a, L.ATTR_FOCAL_DISTANCE),        # name change
            working_distance_unit=_read_str_locked(a, "Working_Distance_unit", "mm"),
            scan_field_x=_read_float(a, "Scan_Field_Size_X"),
            scan_field_x_unit=_read_str_locked(a, "Scan_Field_Size_X_unit", "mm"),
            scan_field_y=_read_float(a, "Scan_Field_Size_Y"),
            scan_field_y_unit=_read_str_locked(a, "Scan_Field_Size_Y_unit", "mm"),
            scan_field_z=_read_float(a, "Scan_Field_Size_Z"),
            scan_field_z_unit=_read_str_locked(a, "Scan_Field_Size_Z_unit", "mm"),
            scan_head_offset_x=_read_float(a, "Scan_Head_Offset_X"),
            scan_head_offset_x_unit=_read_str_locked(a, "Scan_Head_Offset_X_unit", "mm"),
            scan_head_offset_y=_read_float(a, "Scan_Head_Offset_Y"),
            scan_head_offset_y_unit=_read_str_locked(a, "Scan_Head_Offset_Y_unit", "mm"),
            scan_head_offset_z=_read_float(a, "Scan_Head_Offset_Z"),
            scan_head_offset_z_unit=_read_str_locked(a, "Scan_Head_Offset_Z_unit", "mm"),
            scan_head_rotation=_read_float(a, "Scan_Head_Rotation"),
            scan_head_rotation_unit=_read_str_locked(a, "Scan_Head_Rotation_unit", "degrees"),
            axis_configuration=axis_cfg,
            x_axis=x_axis,
            y_axis=y_axis,
            z_axis=z_axis,
            focus=focus,
        )

    def _parse_axis(self, grp: h5py.Group) -> AxisConfig:
        a = grp.attrs
        return AxisConfig(
            actual_bit_resolution=_read_int(a, "Actual_Bit_Resolution"),
            actual_bit_resolution_unit=_read_str(a, "Actual_Bit_Resolution_unit"),
            commanded_bit_resolution=_read_int(a, "Commanded_Bit_Resolution"),
            commanded_bit_resolution_unit=_read_str(a, "Commanded_Bit_Resolution_unit"),
            control_type=_read_str(a, "Control_Type"),
            range_of_motion=_read_float(a, "Range_Of_Motion"),
            range_of_motion_unit=_read_str(a, "Range_Of_Motion_unit"),
            smoothing_kernel=_read_str(a, "Smoothing_Kernel"),
            smoothing_parameters=_read_float(a, "Smoothing_Parameters"),
            tuning_parameters=_read_str(a, "Tuning_Parameters"),
            tuning_type=_read_str(a, "Tuning_Type"),
        )

    def _parse_light_source(self, grp: h5py.Group) -> LightSource:
        a = grp.attrs
        return LightSource(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            wavelength=_read_float(a, "Light_Wavelength"),
            wavelength_unit=_read_str_locked(a, "Light_Wavelength_unit", "nm"),
            power_max_nominal=_read_float(a, "Power_Max_Nominal"),
            power_max_nominal_unit=_read_str_locked(a, "Power_Max_Nominal_unit", "W"),
            power_max_actual=_read_float(a, "Power_Max_Actual"),
            power_max_actual_unit=_read_str_locked(a, "Power_Max_Actual_unit", "W"),
            power_min_actual=_read_float(a, "Power_Min_Actual"),
            power_min_actual_unit=_read_str_locked(a, "Power_Min_Actual_unit", "W"),
            power_min_nominal=_read_float(a, "Power_Min_Nominal"),
            power_min_nominal_unit=_read_str_locked(a, "Power_Min_Nominal_unit", "W"),
            power_bit_resolution=_read_float(a, "Power_Bit_Resolution"),
            power_bit_resolution_unit=_read_str_locked(a, "Power_Bit_Resolution_unit", "bits"),
            watts_to_volts_algorithm=_read_str(a, "Watts_To_Volts_Algorithm"),
            watts_to_volts_params=_read_str(a, "Watts_To_Volts_Params"),
        )

    def _parse_collimator(self, grp: h5py.Group) -> Collimator:
        a = grp.attrs
        return Collimator(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            focal_length=_read_float(a, "Focal_Length"),
            focal_length_unit=_read_str_locked(a, "Focal_Length_unit", "mm"),
        )

    def _parse_scanner_card(self, grp: h5py.Group) -> ScannerCard:
        a = grp.attrs
        return ScannerCard(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=str(a.get("Serial_Number", "")),
            communication_protocol=_read_str(a, "Communication_Protocol"),
            sample_period=_read_float(a, "Sample_Period"),
            sample_period_unit=_read_str_locked(a, "Sample_Period_unit", "μs"),
        )

    def _parse_clearbox(self, grp: h5py.Group) -> ClearBox:
        a = grp.attrs
        corr_data = nan_array_to_nested(grp["Correction_Data"][:])
        inv_data  = nan_array_to_nested(grp["Inverse_Correction_Data"][:])
        synchronous_sensors: dict[str, SynchronousSensor] = {}
        if "Synchronous_Sensors" in grp:
            sensors_grp = grp["Synchronous_Sensors"]
            for name in sensors_grp.keys():
                synchronous_sensors[name] = self._parse_synchronous_sensor(sensors_grp[name])
        return ClearBox(
            ip_address=str(a.get("Ip_Address", "")),
            serial_number=_read_str(a, "Serial_Number"),
            data_port=_read_int(a, "Data_Port"),
            server_port=_read_int(a, "Server_Port"),
            actual_timing_offset=_read_int(a, "Actual_Timing_Offset"),
            commanded_timing_offset=_read_int(a, "Commanded_Timing_Offset"),
            correction_data=corr_data,
            inverse_correction_data=inv_data,
            manufacturer=_read_str(a, "Manufacturer"),
            model=_read_str(a, "Model"),
            output_path=_read_str(a, "Output_Path"),
            selected_camera=_read_str(a, "Selected_Camera"),
            custom_video_format=_read_str(a, "Custom_Video_Format"),
            video_output=_read_str(a, "Video_Output"),
            show_console=_read_bool_from_int(a, "Show_Console"),
            software_trigger_delay=_read_int(a, "Software_Trigger_Delay"),
            volts_to_watts_algorithm=_read_str(a, "Volts_To_Watts_Algorithm"),
            volts_to_watts_params=_read_str(a, "Volts_To_Watts_Params"),
            correction_grid_domain_shape=_read_str(a, "Correction_Grid_Domain_Shape"),
            inverse_grid_domain_shape=_read_str(a, "Inverse_Grid_Domain_Shape"),
            synchronous_sensors=synchronous_sensors,
        )

    def _parse_synchronous_sensor(self, grp: h5py.Group) -> SynchronousSensor:
        a = grp.attrs
        derivation_equation_constants: list[EquationConstant] = []
        if "Derivation_Equation_Constants" in grp:
            rows = grp["Derivation_Equation_Constants"][()]
            derivation_equation_constants = [
                EquationConstant(name=row["name"].decode("utf-8"), value=float(row["value"]))
                for row in rows
            ]
        calibration_points: list[CalibrationPoint] = []
        if "Calibration_Points" in grp:
            rows = grp["Calibration_Points"][()]
            calibration_points = [
                CalibrationPoint(input_value=float(row["input_value"]), output_value=float(row["output_value"]))
                for row in rows
            ]
        return SynchronousSensor(
            enabled=_read_bool_from_int(a, "Enabled"),
            sensor_name=_read_str(a, "Sensor_Name"),
            sensor_output_range_low=_read_float(a, "Sensor_Output_Range_Low"),
            sensor_output_range_high=_read_float(a, "Sensor_Output_Range_High"),
            sensor_output_space=_read_str(a, "Sensor_Output_Space"),
            sensor_model=_read_str(a, "Sensor_Model"),
            sensor_manufacturer=_read_str(a, "Sensor_Manufacturer"),
            sensor_scope=_read_str(a, "Sensor_Scope"),
            units_derived_quantity=_read_str(a, "Units_Derived_Quantity"),
            port_id=_read_int(a, "Port_ID"),
            sensor_type=_read_str(a, "Sensor_Type"),
            input_type=_read_str(a, "Input_Type"),
            algorithm_type=_read_str(a, "Algorithm_Type"),
            algorithm_equation=_read_str(a, "Algorithm_Equation"),
            calibration_source=_read_str(a, "Calibration_Source"),
            calibration_verified=_read_bool_from_int(a, "Calibration_Verified"),
            sample_period=_read_float(a, "Sample_Period"),
            metadata=_read_str(a, "Metadata"),
            derivation_equation_constants=derivation_equation_constants,
            calibration_points=calibration_points,
        )

    def _parse_sfcf(self, ds: h5py.Dataset) -> ScanFieldCorrectionFile:
        a = ds.attrs
        return ScanFieldCorrectionFile(
            document_name=str(a.get("document_name", "")),
            document_id=str(a.get("document_id", "")),
            file_size=int(a.get("file_size", 0)),
            valid_as_of_date=str(a.get("valid_as_of_date", "")),
            document_created_at=_read_str(a, "document_created_at"),
            document_type=_read_str(a, "document_type"),
            original_uri=_read_str(a, "original_uri"),
            raw_bytes=bytes(ds[()]),
        )


# ---------------------------------------------------------------------------
# MockV1_1Writer
# ---------------------------------------------------------------------------

class MockV1_1Writer:
    """Write a stable MachineConfig as a mock v1.1 HDF5 file.

    Fully self-contained — see the matching note on :class:`MockV1_1Reader`.
    """

    def __init__(self, config: MachineConfig) -> None:
        self.config = config

    def write(self, path) -> None:
        with h5py.File(Path(path), "w") as f:
            self._write_root_attrs(f)
            self._write_machine(f)
            self._write_optical_trains(f)

    def _write_root_attrs(self, f: h5py.File) -> None:
        meta = self.config.meta
        f.attrs["machine_name"]       = meta.machine_name
        f.attrs["manufacturer"]       = meta.manufacturer
        f.attrs["model"]              = meta.model
        f.attrs["serial_number"]      = meta.serial_number
        f.attrs["File_Version"]       = L.FILE_VERSION
        f.attrs["Export_Date"]        = meta.export_date
        f.attrs["Configuration_Hash"] = meta.configuration_hash
        f.attrs[L.ATTR_FACILITY_ID]   = meta.facility_id   or ""  # ADDITION (×2): always present
        f.attrs[L.ATTR_CONFIG_AUTHOR] = meta.config_author or ""
        for k, v in meta.extra.items():
            f.attrs[k] = str(v)

    def _write_machine(self, f: h5py.File) -> None:
        grp = f.require_group(L.ROOT_MACHINE)
        ma  = self.config.machine
        bp  = ma.build_plate

        grp.attrs["ID"]                 = _s(ma.id)
        grp.attrs[L.ATTR_MACHINE_LABEL] = ma.machine_name  # NAME CHANGE
        grp.attrs["Manufacturer"]       = ma.manufacturer
        grp.attrs["Model"]              = ma.model
        grp.attrs["Serial_Number"]      = ma.serial_number
        # REMOVAL (×2): Gas_Flow_Direction and Recoat_Direction intentionally omitted

        # Unit attrs stay in Machine/ (path unchanged)
        grp.attrs["Build_Plate_X_Dimension_unit"]   = bp.x_unit or "mm"
        grp.attrs["Build_Plate_Y_Dimension_unit"]   = bp.y_unit or "mm"
        grp.attrs["Build_Plate_Z_Dimension_unit"]   = bp.z_unit or "mm"
        grp.attrs["Build_Plate_Corner_Radius_unit"] = bp.corner_radius_unit or "mm"

        # NAME+PATH (×2) and PATH (×2): all dimension values move to Dimensions/ subgroup
        dims = f.require_group(L.SUBGROUP_DIMENSIONS)
        dims.attrs[L.ATTR_BP_WIDTH]             = _f(bp.x)             # name+path change
        dims.attrs[L.ATTR_BP_HEIGHT]            = _f(bp.y)             # name+path change
        dims.attrs["Build_Plate_Z_Dimension"]   = _f(bp.z)             # path change only
        dims.attrs["Build_Plate_Corner_Radius"] = _f(bp.corner_radius) # path change only

        f.require_group(L.ROOT_OPTICAL_TRAINS)

    def _write_optical_trains(self, f: h5py.File) -> None:
        for i, train in enumerate(self.config.optical_trains):
            tid  = f"{L.TRAIN_ID_PREFIX}{i + 1:02d}"
            base = f"{L.ROOT_OPTICAL_TRAINS}/{tid}"
            self._write_train_attrs(f.require_group(base), train)
            self._write_scanner(f.require_group(f"{base}/{L.GROUP_SCANNER}"), train.scanner)
            self._write_light_source(f.require_group(f"{base}/{L.GROUP_LIGHT_SOURCE}"), train.light_source)
            self._write_collimator(f.require_group(f"{base}/{L.GROUP_COLLIMATOR}"), train.collimator)
            self._write_scanner_card(f.require_group(f"{base}/{L.GROUP_SCANNER_CARD}"), train.scanner_card)
            if train.optional_components.clearbox is not None:
                opt = f.require_group(f"{base}/{L.GROUP_OPTIONAL_COMPONENTS}")
                self._write_clearbox(opt.require_group(L.GROUP_CLEARBOX), train.optional_components.clearbox)
            if train.scan_field_correction_file is not None:
                self._write_sfcf(f, base, train.scan_field_correction_file)

    def _write_train_attrs(self, grp: h5py.Group, t: OpticalTrain) -> None:
        grp.attrs["ID"] = _s(t.id)
        grp.attrs["Beam_Profile_Type"] = _s(t.beam_profile_type)
        grp.attrs["Beam_Waist_Definition"] = _s(t.beam_waist_definition)
        grp.attrs["Beam_Waist_Major"] = _f(t.beam_waist_major)
        grp.attrs["Beam_Waist_Major_unit"] = t.beam_waist_major_unit or "μm"
        grp.attrs["Beam_Waist_Minor"] = _f(t.beam_waist_minor)
        grp.attrs["Beam_Waist_Minor_unit"] = t.beam_waist_minor_unit or "μm"
        grp.attrs["Beam_Waist_Offset_Z"] = _f(t.beam_waist_offset_z)
        grp.attrs["Beam_Waist_Offset_Z_unit"] = t.beam_waist_offset_z_unit or "mm"
        grp.attrs["Build_Plane_Offset_Major"] = _f(t.build_plane_offset_major)
        grp.attrs["Build_Plane_Offset_Major_unit"] = t.build_plane_offset_major_unit or "mm"
        grp.attrs["Build_Plane_Offset_Minor"] = _f(t.build_plane_offset_minor)
        grp.attrs["Build_Plane_Offset_Minor_unit"] = t.build_plane_offset_minor_unit or "mm"
        grp.attrs["Collimator_Focal_Length"] = _f(t.collimator_focal_length)
        grp.attrs["Collimator_Focal_Length_unit"] = t.collimator_focal_length_unit or "mm"
        grp.attrs["M2_Major"] = _f(t.m2_major)
        grp.attrs["M2_Minor"] = _f(t.m2_minor)
        grp.attrs["Major_Axis_Angle"] = _f(t.major_axis_angle)
        grp.attrs["Major_Axis_Angle_unit"] = t.major_axis_angle_unit or "degrees"
        grp.attrs["Rayleigh_Length_Major"] = _f(t.rayleigh_length_major)
        grp.attrs["Rayleigh_Length_Major_unit"] = t.rayleigh_length_major_unit or "mm"
        grp.attrs["Rayleigh_Length_Minor"] = _f(t.rayleigh_length_minor)
        grp.attrs["Rayleigh_Length_Minor_unit"] = t.rayleigh_length_minor_unit or "mm"
        grp.attrs["Scanner_Number"] = _s(t.scanner_number)
        grp.attrs["Thermal_Lensing_Test_Passed"] = _b(t.thermal_lensing_passed)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift"] = _f(t.thermal_lensing_focal_plane_shift)
        grp.attrs["Thermal_Lensing_Focal_Plane_Shift_unit"] = t.thermal_lensing_focal_plane_shift_unit or "mm"
        grp.attrs["Thermal_Lensing_Threshold"] = _f(t.thermal_lensing_threshold)
        grp.attrs["Thermal_Lensing_Threshold_unit"] = t.thermal_lensing_threshold_unit or "mm"

    def _write_scanner(self, grp: h5py.Group, s: Scanner) -> None:
        grp.attrs["Manufacturer"]          = s.manufacturer
        grp.attrs["Model"]                 = s.model
        grp.attrs["Serial_Number"]         = s.serial_number
        grp.attrs[L.ATTR_FOCAL_DISTANCE]   = _f(s.working_distance)  # NAME CHANGE
        grp.attrs["Working_Distance_unit"] = s.working_distance_unit or "mm"
        grp.attrs["Scan_Field_Size_X"]      = _f(s.scan_field_x)
        grp.attrs["Scan_Field_Size_X_unit"] = s.scan_field_x_unit or "mm"
        grp.attrs["Scan_Field_Size_Y"]      = _f(s.scan_field_y)
        grp.attrs["Scan_Field_Size_Y_unit"] = s.scan_field_y_unit or "mm"
        grp.attrs["Scan_Field_Size_Z"]      = _f(s.scan_field_z)
        grp.attrs["Scan_Field_Size_Z_unit"] = s.scan_field_z_unit or "mm"
        grp.attrs["Scan_Head_Offset_X"]      = _f(s.scan_head_offset_x)
        grp.attrs["Scan_Head_Offset_X_unit"] = s.scan_head_offset_x_unit or "mm"
        grp.attrs["Scan_Head_Offset_Y"]      = _f(s.scan_head_offset_y)
        grp.attrs["Scan_Head_Offset_Y_unit"] = s.scan_head_offset_y_unit or "mm"
        grp.attrs["Scan_Head_Offset_Z"]      = _f(s.scan_head_offset_z)
        grp.attrs["Scan_Head_Offset_Z_unit"] = s.scan_head_offset_z_unit or "mm"
        grp.attrs["Scan_Head_Rotation"]      = _f(s.scan_head_rotation)
        grp.attrs["Scan_Head_Rotation_unit"] = s.scan_head_rotation_unit or "degrees"
        grp.attrs["Axis_Configuration"]      = _s(s.axis_configuration)
        self._write_axis(grp.require_group("X_Axis"), s.x_axis)
        self._write_axis(grp.require_group("Y_Axis"), s.y_axis)
        if s.z_axis is not None:
            self._write_axis(grp.require_group("Z_Axis"), s.z_axis)
        if s.focus is not None:
            self._write_axis(grp.require_group("Focus"), s.focus)

    def _write_axis(self, grp: h5py.Group, ax: AxisConfig) -> None:
        grp.attrs["Actual_Bit_Resolution"] = _i(ax.actual_bit_resolution)
        grp.attrs["Actual_Bit_Resolution_unit"] = _s(ax.actual_bit_resolution_unit)
        grp.attrs["Commanded_Bit_Resolution"] = _i(ax.commanded_bit_resolution)
        grp.attrs["Commanded_Bit_Resolution_unit"] = _s(ax.commanded_bit_resolution_unit)
        grp.attrs["Control_Type"] = _s(ax.control_type)
        grp.attrs["Range_Of_Motion"] = _f(ax.range_of_motion)
        grp.attrs["Range_Of_Motion_unit"] = _s(ax.range_of_motion_unit)
        grp.attrs["Smoothing_Kernel"] = _s(ax.smoothing_kernel)
        grp.attrs["Smoothing_Parameters"] = _f(ax.smoothing_parameters)
        grp.attrs["Tuning_Parameters"] = _s(ax.tuning_parameters)
        grp.attrs["Tuning_Type"] = _s(ax.tuning_type)

    def _write_light_source(self, grp: h5py.Group, ls: LightSource) -> None:
        grp.attrs["Manufacturer"]  = ls.manufacturer
        grp.attrs["Model"]         = ls.model
        grp.attrs["Serial_Number"] = ls.serial_number
        grp.attrs["Light_Wavelength"]      = _f(ls.wavelength)
        grp.attrs["Light_Wavelength_unit"] = ls.wavelength_unit or "nm"
        grp.attrs["Power_Max_Nominal"]      = _f(ls.power_max_nominal)
        grp.attrs["Power_Max_Nominal_unit"] = ls.power_max_nominal_unit or "W"
        grp.attrs["Power_Max_Actual"]      = _f(ls.power_max_actual)
        grp.attrs["Power_Max_Actual_unit"] = ls.power_max_actual_unit or "W"
        grp.attrs["Power_Min_Actual"]      = _f(ls.power_min_actual)
        grp.attrs["Power_Min_Actual_unit"] = ls.power_min_actual_unit or "W"
        grp.attrs["Power_Min_Nominal"]      = _f(ls.power_min_nominal)
        grp.attrs["Power_Min_Nominal_unit"] = ls.power_min_nominal_unit or "W"
        grp.attrs["Power_Bit_Resolution"]      = _s(ls.power_bit_resolution)
        grp.attrs["Power_Bit_Resolution_unit"] = ls.power_bit_resolution_unit or "bits"
        grp.attrs["Watts_To_Volts_Algorithm"] = _s(ls.watts_to_volts_algorithm)
        grp.attrs["Watts_To_Volts_Params"]    = _s(ls.watts_to_volts_params)

    def _write_collimator(self, grp: h5py.Group, c: Collimator) -> None:
        grp.attrs["Manufacturer"]  = c.manufacturer
        grp.attrs["Model"]         = c.model
        grp.attrs["Serial_Number"] = c.serial_number
        grp.attrs["Focal_Length"]      = _f(c.focal_length)
        grp.attrs["Focal_Length_unit"] = c.focal_length_unit or "mm"

    def _write_scanner_card(self, grp: h5py.Group, sc: ScannerCard) -> None:
        grp.attrs["Manufacturer"]  = sc.manufacturer
        grp.attrs["Model"]         = sc.model
        grp.attrs["Serial_Number"] = sc.serial_number
        grp.attrs["Communication_Protocol"] = _s(sc.communication_protocol)
        grp.attrs["Sample_Period"]      = _f(sc.sample_period)
        grp.attrs["Sample_Period_unit"] = sc.sample_period_unit or "μs"

    def _write_clearbox(self, grp: h5py.Group, cb: ClearBox) -> None:
        grp.attrs["Ip_Address"]           = cb.ip_address
        grp.attrs["Serial_Number"]        = _s(cb.serial_number)
        grp.attrs["Data_Port"]            = _i(cb.data_port)
        grp.attrs["Server_Port"]          = _i(cb.server_port)
        grp.attrs["Actual_Timing_Offset"]    = _i(cb.actual_timing_offset)
        grp.attrs["Commanded_Timing_Offset"] = _i(cb.commanded_timing_offset)
        grp.attrs["Manufacturer"]         = _s(cb.manufacturer)
        grp.attrs["Model"]                = _s(cb.model)
        grp.attrs["Output_Path"]          = _s(cb.output_path)
        grp.attrs["Selected_Camera"]      = _s(cb.selected_camera)
        grp.attrs["Custom_Video_Format"]  = _s(cb.custom_video_format)
        grp.attrs["Video_Output"]         = _s(cb.video_output)
        grp.attrs["Show_Console"]         = _b(cb.show_console)
        grp.attrs["Software_Trigger_Delay"]    = _i(cb.software_trigger_delay)
        grp.attrs["Volts_To_Watts_Algorithm"]  = _s(cb.volts_to_watts_algorithm)
        grp.attrs["Volts_To_Watts_Params"]     = _s(cb.volts_to_watts_params)
        grp.attrs["Correction_Grid_Domain_Shape"]  = _s(cb.correction_grid_domain_shape)
        grp.attrs["Inverse_Grid_Domain_Shape"]     = _s(cb.inverse_grid_domain_shape)
        grp.create_dataset("Correction_Data", data=nested_to_array(cb.correction_data))
        cds = grp["Correction_Data"]
        cds.attrs["dimensions"] = "H,W,D"
        cds.attrs["dtype"]      = "float64"
        cds.attrs["shape"]      = f"{cds.shape[0]}x{cds.shape[1]}x{cds.shape[2]}"
        grp.create_dataset("Inverse_Correction_Data", data=nested_to_array(cb.inverse_correction_data))
        ids = grp["Inverse_Correction_Data"]
        ids.attrs["dimensions"] = "H,W,D"
        ids.attrs["dtype"]      = "float64"
        ids.attrs["shape"]      = f"{ids.shape[0]}x{ids.shape[1]}x{ids.shape[2]}"
        if cb.synchronous_sensors:
            sensors_grp = grp.create_group("Synchronous_Sensors")
            for name, sensor in cb.synchronous_sensors.items():
                self._write_synchronous_sensor(sensors_grp.require_group(name), sensor)

    def _write_synchronous_sensor(self, grp: h5py.Group, sensor: SynchronousSensor) -> None:
        grp.attrs["Enabled"]                  = _b(sensor.enabled)
        grp.attrs["Sensor_Name"]              = _s(sensor.sensor_name)
        grp.attrs["Sensor_Output_Range_Low"]  = _f(sensor.sensor_output_range_low)
        grp.attrs["Sensor_Output_Range_High"] = _f(sensor.sensor_output_range_high)
        grp.attrs["Sensor_Output_Space"]      = _s(sensor.sensor_output_space)
        grp.attrs["Sensor_Model"]             = _s(sensor.sensor_model)
        grp.attrs["Sensor_Manufacturer"]      = _s(sensor.sensor_manufacturer)
        grp.attrs["Sensor_Scope"]             = _s(sensor.sensor_scope)
        grp.attrs["Units_Derived_Quantity"]   = _s(sensor.units_derived_quantity)
        grp.attrs["Port_ID"]                  = _i(sensor.port_id)
        grp.attrs["Sensor_Type"]              = _s(sensor.sensor_type)
        grp.attrs["Input_Type"]               = _s(sensor.input_type)
        grp.attrs["Algorithm_Type"]           = _s(sensor.algorithm_type)
        grp.attrs["Algorithm_Equation"]       = _s(sensor.algorithm_equation)
        grp.attrs["Calibration_Source"]       = _s(sensor.calibration_source)
        grp.attrs["Calibration_Verified"]     = _b(sensor.calibration_verified)
        grp.attrs["Sample_Period"]            = _f(sensor.sample_period)
        grp.attrs["Metadata"]                 = _s(sensor.metadata)

        for c in sensor.derivation_equation_constants:
            name_bytes = len(c.name.encode("utf-8"))
            if name_bytes > EQUATION_CONSTANT_NAME_MAX_BYTES:
                raise ValueError(
                    f"Derivation_Equation_Constants name {c.name!r} is {name_bytes} UTF-8 "
                    f"bytes, which does not fit in the {EQUATION_CONSTANT_NAME_MAX_BYTES}-byte "
                    "fixed-length field (would otherwise be silently truncated on write)."
                )
        constants = np.array(
            [(c.name, c.value) for c in sensor.derivation_equation_constants],
            dtype=_EQUATION_CONSTANT_DTYPE,
        )
        grp.create_dataset("Derivation_Equation_Constants", data=constants)

        points = np.array(
            [(p.input_value, p.output_value) for p in sensor.calibration_points],
            dtype=_CALIBRATION_POINT_DTYPE,
        )
        grp.create_dataset("Calibration_Points", data=points)

    def _write_sfcf(self, f: h5py.File, train_path: str, sfcf: ScanFieldCorrectionFile) -> None:
        if sfcf.raw_bytes is not None:
            data = np.frombuffer(sfcf.raw_bytes, dtype=np.uint8)
        else:
            data = np.zeros(max(sfcf.file_size, 1), dtype=np.uint8)
        ds = f.create_dataset(f"{train_path}/{L.DS_SCAN_FIELD_CORRECTION_FILE}", data=data)
        ds.attrs["document_name"]    = sfcf.document_name
        ds.attrs["document_id"]      = sfcf.document_id
        ds.attrs["file_size"]        = sfcf.file_size
        ds.attrs["valid_as_of_date"] = sfcf.valid_as_of_date
        ds.attrs["document_created_at"] = _s(sfcf.document_created_at)
        ds.attrs["document_type"]       = _s(sfcf.document_type)
        ds.attrs["original_uri"]        = _s(sfcf.original_uri)


# ---------------------------------------------------------------------------
# Factory helpers
# ---------------------------------------------------------------------------

def _axis(label: str = "X") -> AxisConfig:
    return AxisConfig(
        actual_bit_resolution=20,
        actual_bit_resolution_unit="bits",
        commanded_bit_resolution=18,
        commanded_bit_resolution_unit="bits",
        control_type=f"CLOSED_{label}",
        range_of_motion=120.5,
        range_of_motion_unit="mm",
        smoothing_kernel="GAUSSIAN",
        smoothing_parameters=60.0,
        tuning_parameters="1.0,2.0",
        tuning_type="PID",
    )


def _make_config(
    machine_name: str = "TestMachine",
    facility_id: str | None = None,
    config_author: str | None = None,
) -> MachineConfig:
    return MachineConfig(
        meta=MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=machine_name,
            manufacturer="AcmeCorp",
            model="Model-X",
            serial_number="SN-001",
            file_version="1.1-mock",
            export_date="2026-01-01",
            configuration_hash="deadbeef",
            facility_id=facility_id,
            config_author=config_author,
        ),
        machine=Machine(
            id="machine-uuid",
            machine_name=machine_name,
            manufacturer="AcmeCorp",
            model="Model-X",
            serial_number="SN-001",
            build_plate=BuildPlate(
                x=250.0, x_unit="mm",
                y=175.0, y_unit="mm",   # distinct from x so name+path assertions are unambiguous
                z=300.0, z_unit="mm",
                corner_radius=10.0, corner_radius_unit="mm",
            ),
            gas_flow_direction=None,  # absent in v1.1 by design
            recoat_direction=None,    # absent in v1.1 by design
        ),
        optical_trains=[
            OpticalTrain(
                train_id="Optical_Train_01",
                id="train-uuid",
                beam_profile_type="Gaussian",
                beam_waist_definition="1/e^2",
                beam_waist_major=50.0,  beam_waist_major_unit="μm",
                beam_waist_minor=50.0,  beam_waist_minor_unit="μm",
                beam_waist_offset_z=0.0, beam_waist_offset_z_unit="mm",
                m2_major=1.1, m2_minor=1.1,
                rayleigh_length_major=1.0, rayleigh_length_major_unit="mm",
                rayleigh_length_minor=1.0, rayleigh_length_minor_unit="mm",
                build_plane_offset_major=0.0, build_plane_offset_major_unit="mm",
                build_plane_offset_minor=0.0, build_plane_offset_minor_unit="mm",
                collimator_focal_length=100.0, collimator_focal_length_unit="mm",
                major_axis_angle=0.0, major_axis_angle_unit="degrees",
                scanner_number="1",
                thermal_lensing_passed=True,
                thermal_lensing_focal_plane_shift=0.1, thermal_lensing_focal_plane_shift_unit="mm",
                thermal_lensing_threshold=0.5, thermal_lensing_threshold_unit="mm",
                scanner=Scanner(
                    manufacturer="ScanLab",
                    model="HurrySCAN",
                    serial_number="SC-001",
                    working_distance=420.0, working_distance_unit="mm",
                    scan_field_x=100.0, scan_field_x_unit="mm",
                    scan_field_y=100.0, scan_field_y_unit="mm",
                    scan_field_z=None, scan_field_z_unit=None,
                    scan_head_offset_x=0.0, scan_head_offset_x_unit="mm",
                    scan_head_offset_y=0.0, scan_head_offset_y_unit="mm",
                    scan_head_offset_z=0.0, scan_head_offset_z_unit="mm",
                    scan_head_rotation=0.0, scan_head_rotation_unit="degrees",
                    axis_configuration="2D",
                    x_axis=_axis("X"),
                    y_axis=_axis("Y"),
                ),
                light_source=LightSource(
                    manufacturer="IPG",
                    model="YLR-500",
                    serial_number="LS-001",
                    wavelength=1070.0, wavelength_unit="nm",
                    power_max_nominal=500.0, power_max_nominal_unit="W",
                    power_max_actual=490.0,  power_max_actual_unit="W",
                    power_min_actual=10.0,   power_min_actual_unit="W",
                    power_min_nominal=10.0,  power_min_nominal_unit="W",
                    power_bit_resolution=None, power_bit_resolution_unit="bits",
                    watts_to_volts_algorithm=None,
                    watts_to_volts_params=None,
                ),
                collimator=Collimator(
                    manufacturer="Sill",
                    model="S6ASS2075",
                    serial_number="COL-001",
                    focal_length=75.0, focal_length_unit="mm",
                ),
                scanner_card=ScannerCard(
                    manufacturer="ScanLab",
                    model="RTC5",
                    serial_number="SC-CARD-001",
                    communication_protocol="PCIe",
                    sample_period=10.0, sample_period_unit="μs",
                ),
                optional_components=OptionalComponents(),
                scan_field_correction_file=None,
            )
        ],
        opcua=None,
    )


# ---------------------------------------------------------------------------
# Tests — adapter-level (bypass dispatcher)
# ---------------------------------------------------------------------------

def test_v1_1_read(tmp_path):
    """Mock v1.1 file → StableModel — all five change categories asserted."""
    cfg = _make_config(
        machine_name="MigrationTestMachine",
        facility_id="Lab-001",
        config_author="TestEngineer",
    )
    p = tmp_path / "v1_1.h5"
    MockV1_1Writer(cfg).write(p)
    result = MockV1_1Reader(p).parse()

    # ADDITION (×2)
    assert result.meta.facility_id   == "Lab-001"
    assert result.meta.config_author == "TestEngineer"

    # REMOVAL (×2)
    assert result.machine.gas_flow_direction is None
    assert result.machine.recoat_direction   is None

    # NAME CHANGE (×2)
    assert result.machine.machine_name                        == "MigrationTestMachine"
    assert result.optical_trains[0].scanner.working_distance == 420.0

    # PATH CHANGE (×2)
    assert result.machine.build_plate.z             == 300.0
    assert result.machine.build_plate.corner_radius == 10.0

    # NAME+PATH CHANGE (×2)
    assert result.machine.build_plate.x == 250.0
    assert result.machine.build_plate.y == 175.0


def test_v1_1_roundtrip(tmp_path):
    """Mock v1.1 → StableModel → mock v1.1 → StableModel — all categories survive both passes."""
    cfg = _make_config(facility_id="RoundtripLab", config_author="RoundtripEngineer")
    p1 = tmp_path / "v1_1_a.h5"
    MockV1_1Writer(cfg).write(p1)
    mid = MockV1_1Reader(p1).parse()
    p2 = tmp_path / "v1_1_b.h5"
    MockV1_1Writer(mid).write(p2)
    result = MockV1_1Reader(p2).parse()

    assert result.meta.facility_id              == "RoundtripLab"
    assert result.meta.config_author            == "RoundtripEngineer"
    assert result.machine.gas_flow_direction                  is None
    assert result.machine.recoat_direction                    is None
    assert result.machine.machine_name                        == cfg.machine.machine_name
    assert result.optical_trains[0].scanner.working_distance  == cfg.optical_trains[0].scanner.working_distance
    assert result.machine.build_plate.z                       == cfg.machine.build_plate.z
    assert result.machine.build_plate.corner_radius           == cfg.machine.build_plate.corner_radius
    assert result.machine.build_plate.x                       == cfg.machine.build_plate.x
    assert result.machine.build_plate.y                       == cfg.machine.build_plate.y


def test_v1_to_v1_1(tmp_path):
    """Real v1.0 fixture → StableModel → mock v1.1 layout — surviving fields preserved."""
    source = MachineConfigReader(_REFERENCE_H5).parse()
    out    = tmp_path / "migrated_v1_1.h5"
    MockV1_1Writer(dc_replace(source, meta=dc_replace(source.meta, file_version="1.1-mock"))).write(out)
    result = MockV1_1Reader(out).parse()

    # NAME CHANGE: machine_name survives both the read and write name remapping
    assert result.machine.machine_name == source.machine.machine_name

    # NAME+PATH CHANGE: x and y survive via StableModel intermediary
    assert result.machine.build_plate.x == source.machine.build_plate.x
    assert result.machine.build_plate.y == source.machine.build_plate.y

    # PATH CHANGE: z and corner_radius survive
    assert result.machine.build_plate.z             == source.machine.build_plate.z
    assert result.machine.build_plate.corner_radius == source.machine.build_plate.corner_radius

    # NAME CHANGE: scanner working_distance survives
    assert result.optical_trains[0].scanner.working_distance == source.optical_trains[0].scanner.working_distance

    # REMOVAL: always None regardless of what the v1.0 source contained
    assert result.machine.gas_flow_direction is None
    assert result.machine.recoat_direction   is None

    # ADDITION: no v1.0 source → typed fields are None after forward migration
    assert result.meta.facility_id   is None
    assert result.meta.config_author is None


def test_v1_1_to_v1(tmp_path):
    """Mock v1.1 file → StableModel → v1.0 layout — surviving fields preserved."""
    cfg = _make_config(facility_id="Lab-V11", config_author="MigrationBot")

    v1_1_file  = tmp_path / "source_v1_1.h5"
    MockV1_1Writer(cfg).write(v1_1_file)
    v1_1_config = MockV1_1Reader(v1_1_file).parse()

    v1_out = tmp_path / "migrated_v1.h5"
    MachineConfigWriter(dc_replace(v1_1_config, meta=dc_replace(v1_1_config.meta, file_version="1.0"))).write(v1_out)
    result = MachineConfigReader(v1_out).parse()

    assert result.machine.machine_name                        == cfg.machine.machine_name
    assert result.machine.build_plate.x                       == cfg.machine.build_plate.x
    assert result.machine.build_plate.y                       == cfg.machine.build_plate.y
    assert result.machine.build_plate.z                       == cfg.machine.build_plate.z
    assert result.machine.build_plate.corner_radius           == cfg.machine.build_plate.corner_radius
    assert result.optical_trains[0].scanner.working_distance  == cfg.optical_trains[0].scanner.working_distance

    # REMOVAL: absent in v1.1 → remain None after round-trip through v1.0 layout
    assert result.machine.gas_flow_direction is None
    assert result.machine.recoat_direction   is None

    # ADDITION: typed v1.1 fields are lost during backward migration (v1.0 writer does not write them)
    assert result.meta.facility_id   is None
    assert result.meta.config_author is None


def test_v1_unaffected():
    """Existing v1.0 read path is undisturbed — no mock adapter involved."""
    config = MachineConfigReader(_REFERENCE_H5).parse()
    assert config.meta.file_version == "1.0"
    assert len(config.optical_trains) > 0
    assert config.machine is not None
    assert config.machine.build_plate.x is not None


# ---------------------------------------------------------------------------
# Tests — dispatcher-level (full public API via monkeypatched _ADAPTERS)
# ---------------------------------------------------------------------------

def test_dispatcher_v1_to_v1_1(tmp_path, monkeypatch):
    """Public API: v1.1 file read and then migrated to v1.0 through the dispatcher."""
    monkeypatch.setitem(_reader_mod._ADAPTERS, "1.1-mock", MockV1_1Reader)
    monkeypatch.setitem(_writer_mod._ADAPTERS, "1.1-mock", MockV1_1Writer)

    cfg        = _make_config(machine_name="DispatcherTest", facility_id="Dispatch-Lab")
    v1_1_file  = tmp_path / "dispatcher_v1_1.h5"
    MockV1_1Writer(cfg).write(v1_1_file)

    # Public API read — dispatcher peeks "1.1-mock" and routes to MockV1_1Reader
    result = MachineConfigReader(v1_1_file).parse()
    assert result.meta.file_version                  == "1.1-mock"
    assert result.meta.facility_id               == "Dispatch-Lab"
    assert result.machine.machine_name            == cfg.machine.machine_name
    assert result.machine.build_plate.x              == cfg.machine.build_plate.x

    # Public API write to v1.0 — dispatcher sees file_version="1.0" and routes to Hdf5WriterV1_0
    v1_out = tmp_path / "dispatcher_migrated_v1.h5"
    MachineConfigWriter(dc_replace(result, meta=dc_replace(result.meta, file_version="1.0"))).write(v1_out)
    final = MachineConfigReader(v1_out).parse()
    assert final.meta.file_version    == "1.0"
    assert final.machine.machine_name == cfg.machine.machine_name
    assert final.machine.build_plate.x == cfg.machine.build_plate.x


def test_dispatcher_v1_1_to_v1(tmp_path, monkeypatch):
    """Public API: real v1.0 file migrated to v1.1 and read back through the dispatcher."""
    monkeypatch.setitem(_reader_mod._ADAPTERS, "1.1-mock", MockV1_1Reader)
    monkeypatch.setitem(_writer_mod._ADAPTERS, "1.1-mock", MockV1_1Writer)

    source = MachineConfigReader(_REFERENCE_H5).parse()

    # Public API write — dispatcher sees file_version="1.1-mock" and routes to MockV1_1Writer
    v1_1_out = tmp_path / "dispatcher_v1_1_out.h5"
    MachineConfigWriter(dc_replace(source, meta=dc_replace(source.meta, file_version="1.1-mock"))).write(v1_1_out)

    # Public API read — dispatcher peeks "1.1-mock" and routes to MockV1_1Reader
    result = MachineConfigReader(v1_1_out).parse()
    assert result.meta.file_version                           == "1.1-mock"
    assert result.machine.machine_name                        == source.machine.machine_name
    assert result.machine.build_plate.x                       == source.machine.build_plate.x
    assert result.optical_trains[0].scanner.working_distance  == source.optical_trains[0].scanner.working_distance
    assert result.machine.gas_flow_direction                  is None  # removed in v1.1
    assert result.machine.recoat_direction                    is None  # removed in v1.1


def test_mock_adapters_satisfy_protocol(tmp_path):
    """MockV1_1Reader and MockV1_1Writer satisfy the ReaderAdapter / WriterAdapter Protocols."""
    cfg = _make_config()
    p   = tmp_path / "proto_check.h5"
    MockV1_1Writer(cfg).write(p)
    assert isinstance(MockV1_1Reader(p), ReaderAdapter)
    assert isinstance(MockV1_1Writer(cfg), WriterAdapter)
