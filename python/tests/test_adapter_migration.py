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
import pytest

import machine_config.reader as _reader_mod
import machine_config.writer as _writer_mod
from machine_config import MachineConfigReader, MachineConfigWriter
from machine_config.reader import ReaderAdapter
from machine_config.writer import WriterAdapter
from machine_config.capabilities.v1_0 import layout as _v1_0_layout
from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0
from machine_config.models import (
    AxisConfig,
    BuildPlate,
    Collimator,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    OptionalComponents,
    Scanner,
    ScannerCard,
)
from machine_config.schema import SCHEMA_VERSION

_REPO_ROOT = Path(__file__).parent.parent.parent
_REFERENCE_H5 = _REPO_ROOT / "fixtures" / "reference_config.h5"


# ---------------------------------------------------------------------------
# MockV1_1Layout — on-disk constants that differ from v1.0
# ---------------------------------------------------------------------------

class MockV1_1Layout:
    FILE_VERSION = "1.1"

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


L = MockV1_1Layout  # shorthand used throughout this module


# ---------------------------------------------------------------------------
# MockV1_1Reader
# ---------------------------------------------------------------------------

class MockV1_1Reader:
    """Read a mock v1.1 HDF5 file and return a stable MachineConfig."""

    def __init__(self, path) -> None:
        self.path = Path(path)
        self._v1_0 = Hdf5AdapterV1_0(self.path)  # delegate for unchanged subcomponents

    def parse(self) -> MachineConfig:
        with h5py.File(self.path, "r") as f:
            return self._parse(f)

    def _parse(self, f: h5py.File) -> MachineConfig:
        R = Hdf5AdapterV1_0  # static type-conversion helpers

        # ADDITION (×2): typed fields read from dedicated HDF5 attrs; empty string → None
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=str(f.attrs.get("machine_name", "")),
            manufacturer=str(f.attrs.get("manufacturer", "")),
            model=str(f.attrs.get("model", "")),
            serial_number=str(f.attrs.get("serial_number", "")),
            file_version=str(f.attrs.get("File_Version", "")).strip() or "1.1",
            export_date=str(f.attrs.get("Export_Date", "")),
            configuration_hash=str(f.attrs.get("Configuration_Hash", "")),
            facility_id=str(f.attrs.get(L.ATTR_FACILITY_ID, "")).strip() or None,
            config_author=str(f.attrs.get(L.ATTR_CONFIG_AUTHOR, "")).strip() or None,
        )

        ma   = f[L.ROOT_MACHINE].attrs
        dims = f[L.SUBGROUP_DIMENSIONS].attrs

        build_plate = BuildPlate(
            x=R._read_float(dims, L.ATTR_BP_WIDTH),                  # name+path change
            x_unit=R._read_str(ma, "Build_Plate_X_Dimension_unit"),
            y=R._read_float(dims, L.ATTR_BP_HEIGHT),                  # name+path change
            y_unit=R._read_str(ma, "Build_Plate_Y_Dimension_unit"),
            z=R._read_float(dims, "Build_Plate_Z_Dimension"),         # path change
            z_unit=R._read_str(ma, "Build_Plate_Z_Dimension_unit"),
            corner_radius=R._read_float(dims, "Build_Plate_Corner_Radius"),  # path change
            corner_radius_unit=R._read_str(ma, "Build_Plate_Corner_Radius_unit"),
        )

        machine = Machine(
            id=R._read_str(ma, "ID"),
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
        R    = Hdf5AdapterV1_0

        # Unchanged subcomponents: delegate to v1.0 adapter instance
        light_source = self._v1_0._parse_light_source(f[f"{base}/{L.GROUP_LIGHT_SOURCE}"])
        collimator   = self._v1_0._parse_collimator(f[f"{base}/{L.GROUP_COLLIMATOR}"])
        scanner_card = self._v1_0._parse_scanner_card(f[f"{base}/{L.GROUP_SCANNER_CARD}"])
        scanner      = self._parse_scanner(f[f"{base}/{L.GROUP_SCANNER}"])

        cb_path   = _v1_0_layout.clearbox_path_by_id(tid)
        clearbox  = self._v1_0._parse_clearbox(f[cb_path]) if cb_path in f else None
        sfcf_path = _v1_0_layout.scan_field_correction_file_path_by_id(tid)
        sfcf      = self._v1_0._parse_sfcf(f[sfcf_path]) if sfcf_path in f else None

        return OpticalTrain(
            train_id=tid,
            id=R._read_str(a, "ID"),
            beam_profile_type=R._read_str(a, "Beam_Profile_Type"),
            beam_waist_definition=R._read_str(a, "Beam_Waist_Definition"),
            beam_waist_major=R._read_float(a, "Beam_Waist_Major"),
            beam_waist_major_unit=R._read_str_locked(a, "Beam_Waist_Major_unit", "μm"),
            beam_waist_minor=R._read_float(a, "Beam_Waist_Minor"),
            beam_waist_minor_unit=R._read_str_locked(a, "Beam_Waist_Minor_unit", "μm"),
            beam_waist_offset_z=R._read_float(a, "Beam_Waist_Offset_Z"),
            beam_waist_offset_z_unit=R._read_str_locked(a, "Beam_Waist_Offset_Z_unit", "mm"),
            build_plane_offset_major=R._read_float(a, "Build_Plane_Offset_Major"),
            build_plane_offset_major_unit=R._read_str_locked(a, "Build_Plane_Offset_Major_unit", "mm"),
            build_plane_offset_minor=R._read_float(a, "Build_Plane_Offset_Minor"),
            build_plane_offset_minor_unit=R._read_str_locked(a, "Build_Plane_Offset_Minor_unit", "mm"),
            collimator_focal_length=R._read_float(a, "Collimator_Focal_Length"),
            collimator_focal_length_unit=R._read_str_locked(a, "Collimator_Focal_Length_unit", "mm"),
            m2_major=R._read_float(a, "M2_Major"),
            m2_minor=R._read_float(a, "M2_Minor"),
            major_axis_angle=R._read_float(a, "Major_Axis_Angle"),
            major_axis_angle_unit=R._read_str_locked(a, "Major_Axis_Angle_unit", "degrees"),
            rayleigh_length_major=R._read_float(a, "Rayleigh_Length_Major"),
            rayleigh_length_major_unit=R._read_str_locked(a, "Rayleigh_Length_Major_unit", "mm"),
            rayleigh_length_minor=R._read_float(a, "Rayleigh_Length_Minor"),
            rayleigh_length_minor_unit=R._read_str_locked(a, "Rayleigh_Length_Minor_unit", "mm"),
            scanner_number=R._read_str(a, "Scanner_Number"),
            thermal_lensing_passed=R._read_bool_from_int(a, "Thermal_Lensing_Test_Passed"),
            thermal_lensing_focal_plane_shift=R._read_float(a, "Thermal_Lensing_Focal_Plane_Shift"),
            thermal_lensing_focal_plane_shift_unit=R._read_str_locked(a, "Thermal_Lensing_Focal_Plane_Shift_unit", "mm"),
            thermal_lensing_threshold=R._read_float(a, "Thermal_Lensing_Threshold"),
            thermal_lensing_threshold_unit=R._read_str_locked(a, "Thermal_Lensing_Threshold_unit", "mm"),
            scanner=scanner,
            light_source=light_source,
            collimator=collimator,
            scanner_card=scanner_card,
            optional_components=OptionalComponents(clearbox=clearbox),
            scan_field_correction_file=sfcf,
        )

    def _parse_scanner(self, grp: h5py.Group) -> Scanner:
        R        = Hdf5AdapterV1_0
        a        = grp.attrs
        axis_cfg = R._read_str(a, "Axis_Configuration")
        x_axis   = self._v1_0._parse_axis(grp["X_Axis"])
        y_axis   = self._v1_0._parse_axis(grp["Y_Axis"])
        z_axis   = self._v1_0._parse_axis(grp["Z_Axis"]) if "Z_Axis" in grp else None
        focus    = self._v1_0._parse_axis(grp["Focus"])  if "Focus"  in grp else None
        return Scanner(
            manufacturer=str(a.get("Manufacturer", "")),
            model=str(a.get("Model", "")),
            serial_number=R._read_str(a, "Serial_Number") or "",
            working_distance=R._read_float(a, L.ATTR_FOCAL_DISTANCE),        # name change
            working_distance_unit=R._read_str_locked(a, "Working_Distance_unit", "mm"),
            scan_field_x=R._read_float(a, "Scan_Field_Size_X"),
            scan_field_x_unit=R._read_str_locked(a, "Scan_Field_Size_X_unit", "mm"),
            scan_field_y=R._read_float(a, "Scan_Field_Size_Y"),
            scan_field_y_unit=R._read_str_locked(a, "Scan_Field_Size_Y_unit", "mm"),
            scan_field_z=R._read_float(a, "Scan_Field_Size_Z"),
            scan_field_z_unit=R._read_str_locked(a, "Scan_Field_Size_Z_unit", "mm"),
            scan_head_offset_x=R._read_float(a, "Scan_Head_Offset_X"),
            scan_head_offset_x_unit=R._read_str_locked(a, "Scan_Head_Offset_X_unit", "mm"),
            scan_head_offset_y=R._read_float(a, "Scan_Head_Offset_Y"),
            scan_head_offset_y_unit=R._read_str_locked(a, "Scan_Head_Offset_Y_unit", "mm"),
            scan_head_offset_z=R._read_float(a, "Scan_Head_Offset_Z"),
            scan_head_offset_z_unit=R._read_str_locked(a, "Scan_Head_Offset_Z_unit", "mm"),
            scan_head_rotation=R._read_float(a, "Scan_Head_Rotation"),
            scan_head_rotation_unit=R._read_str_locked(a, "Scan_Head_Rotation_unit", "degrees"),
            axis_configuration=axis_cfg,
            x_axis=x_axis,
            y_axis=y_axis,
            z_axis=z_axis,
            focus=focus,
        )


# ---------------------------------------------------------------------------
# MockV1_1Writer
# ---------------------------------------------------------------------------

class MockV1_1Writer:
    """Write a stable MachineConfig as a mock v1.1 HDF5 file."""

    def __init__(self, config: MachineConfig) -> None:
        self.config = config
        self._v1_0 = Hdf5WriterV1_0(config)  # delegate for unchanged subcomponents

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
        W   = Hdf5WriterV1_0

        grp.attrs["ID"]                 = W._s(ma.id)
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
        dims.attrs[L.ATTR_BP_WIDTH]             = W._f(bp.x)             # name+path change
        dims.attrs[L.ATTR_BP_HEIGHT]            = W._f(bp.y)             # name+path change
        dims.attrs["Build_Plate_Z_Dimension"]   = W._f(bp.z)             # path change only
        dims.attrs["Build_Plate_Corner_Radius"] = W._f(bp.corner_radius) # path change only

        f.require_group(L.ROOT_OPTICAL_TRAINS)

    def _write_optical_trains(self, f: h5py.File) -> None:
        for i, train in enumerate(self.config.optical_trains):
            tid  = f"{L.TRAIN_ID_PREFIX}{i + 1:02d}"
            base = f"{L.ROOT_OPTICAL_TRAINS}/{tid}"
            self._v1_0._write_train_attrs(f.require_group(base), train)
            self._write_scanner(f.require_group(f"{base}/{L.GROUP_SCANNER}"), train.scanner)
            self._v1_0._write_light_source(f.require_group(f"{base}/{L.GROUP_LIGHT_SOURCE}"), train.light_source)
            self._v1_0._write_collimator(f.require_group(f"{base}/{L.GROUP_COLLIMATOR}"), train.collimator)
            self._v1_0._write_scanner_card(f.require_group(f"{base}/{L.GROUP_SCANNER_CARD}"), train.scanner_card)
            if train.optional_components.clearbox is not None:
                opt = f.require_group(f"{base}/{_v1_0_layout.GROUP_OPTIONAL_COMPONENTS}")
                self._v1_0._write_clearbox(opt.require_group(_v1_0_layout.GROUP_CLEARBOX), train.optional_components.clearbox)
            if train.scan_field_correction_file is not None:
                self._v1_0._write_sfcf(f, base, train.scan_field_correction_file)

    def _write_scanner(self, grp: h5py.Group, s: Scanner) -> None:
        W = Hdf5WriterV1_0
        grp.attrs["Manufacturer"]          = s.manufacturer
        grp.attrs["Model"]                 = s.model
        grp.attrs["Serial_Number"]         = s.serial_number
        grp.attrs[L.ATTR_FOCAL_DISTANCE]   = W._f(s.working_distance)  # NAME CHANGE
        grp.attrs["Working_Distance_unit"] = s.working_distance_unit or "mm"
        grp.attrs["Scan_Field_Size_X"]      = W._f(s.scan_field_x)
        grp.attrs["Scan_Field_Size_X_unit"] = s.scan_field_x_unit or "mm"
        grp.attrs["Scan_Field_Size_Y"]      = W._f(s.scan_field_y)
        grp.attrs["Scan_Field_Size_Y_unit"] = s.scan_field_y_unit or "mm"
        grp.attrs["Scan_Field_Size_Z"]      = W._f(s.scan_field_z)
        grp.attrs["Scan_Field_Size_Z_unit"] = s.scan_field_z_unit or "mm"
        grp.attrs["Scan_Head_Offset_X"]      = W._f(s.scan_head_offset_x)
        grp.attrs["Scan_Head_Offset_X_unit"] = s.scan_head_offset_x_unit or "mm"
        grp.attrs["Scan_Head_Offset_Y"]      = W._f(s.scan_head_offset_y)
        grp.attrs["Scan_Head_Offset_Y_unit"] = s.scan_head_offset_y_unit or "mm"
        grp.attrs["Scan_Head_Offset_Z"]      = W._f(s.scan_head_offset_z)
        grp.attrs["Scan_Head_Offset_Z_unit"] = s.scan_head_offset_z_unit or "mm"
        grp.attrs["Scan_Head_Rotation"]      = W._f(s.scan_head_rotation)
        grp.attrs["Scan_Head_Rotation_unit"] = s.scan_head_rotation_unit or "degrees"
        grp.attrs["Axis_Configuration"]      = W._s(s.axis_configuration)
        self._v1_0._write_axis(grp.require_group("X_Axis"), s.x_axis)
        self._v1_0._write_axis(grp.require_group("Y_Axis"), s.y_axis)
        if s.z_axis is not None:
            self._v1_0._write_axis(grp.require_group("Z_Axis"), s.z_axis)
        if s.focus is not None:
            self._v1_0._write_axis(grp.require_group("Focus"), s.focus)


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
            file_version="1.1",
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
    MockV1_1Writer(dc_replace(source, meta=dc_replace(source.meta, file_version="1.1"))).write(out)
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
    monkeypatch.setitem(_reader_mod._ADAPTERS, "1.1", MockV1_1Reader)
    monkeypatch.setitem(_writer_mod._ADAPTERS, "1.1", MockV1_1Writer)

    cfg        = _make_config(machine_name="DispatcherTest", facility_id="Dispatch-Lab")
    v1_1_file  = tmp_path / "dispatcher_v1_1.h5"
    MockV1_1Writer(cfg).write(v1_1_file)

    # Public API read — dispatcher peeks "1.1" and routes to MockV1_1Reader
    result = MachineConfigReader(v1_1_file).parse()
    assert result.meta.file_version                  == "1.1"
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
    monkeypatch.setitem(_reader_mod._ADAPTERS, "1.1", MockV1_1Reader)
    monkeypatch.setitem(_writer_mod._ADAPTERS, "1.1", MockV1_1Writer)

    source = MachineConfigReader(_REFERENCE_H5).parse()

    # Public API write — dispatcher sees file_version="1.1" and routes to MockV1_1Writer
    v1_1_out = tmp_path / "dispatcher_v1_1_out.h5"
    MachineConfigWriter(dc_replace(source, meta=dc_replace(source.meta, file_version="1.1"))).write(v1_1_out)

    # Public API read — dispatcher peeks "1.1" and routes to MockV1_1Reader
    result = MachineConfigReader(v1_1_out).parse()
    assert result.meta.file_version                           == "1.1"
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
