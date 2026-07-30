"""Phase 1.5 — Config builders: MockConfigBuilder, YamlConfigBuilder, ConfigEditor."""
from __future__ import annotations

import uuid
from dataclasses import replace
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import h5py
import numpy as np
import yaml

from .models import (
    AxisConfig,
    BuildPlate,
    ClearBox,
    Collimator,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpticalTrain,
    OptionalComponents,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)
from .reader import MachineConfigReader
from .schema import SCHEMA_VERSION
from .writer import MachineConfigWriter


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _now_iso() -> str:
    return datetime.now(tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def _gaussian_correction_grid(shape: tuple[int, int, int] = (257, 257, 2)) -> np.ndarray:
    """Smooth 2-D Gaussian warp pattern. Non-zero by design — zeros would hide bugs."""
    nx, ny = shape[0], shape[1]
    x  = np.linspace(-1.0, 1.0, nx)
    y  = np.linspace(-1.0, 1.0, ny)
    xx, yy = np.meshgrid(x, y, indexing="ij")
    r2   = xx ** 2 + yy ** 2
    warp = 2.0 * np.exp(-r2 / 0.5)
    grid = np.zeros(shape, dtype=np.float64)
    grid[:, :, 0] = warp
    grid[:, :, 1] = warp * 0.8
    return grid


def _mock_axis(bit_res: int = 20) -> AxisConfig:
    return AxisConfig(
        actual_bit_resolution=bit_res,
        actual_bit_resolution_unit="bits",
        commanded_bit_resolution=bit_res,
        commanded_bit_resolution_unit="bits",
        control_type=None,
        range_of_motion=None,
        range_of_motion_unit="mm",
        smoothing_kernel="GAUSSIAN",
        smoothing_parameters=60.0,
        tuning_parameters=None,
        tuning_type=None,
    )


def _mock_clearbox(ip: str, serial: str) -> ClearBox:
    grid = _gaussian_correction_grid()
    inv  = _gaussian_correction_grid() * 0.95
    # Convert ndarray to nested Python list (no NaNs in synthetic data)
    correction_data:         list = grid.tolist()
    inverse_correction_data: list = inv.tolist()
    return ClearBox(
        ip_address=ip,
        serial_number=serial,
        data_port=5001,
        server_port=20101,
        actual_timing_offset=-8,
        commanded_timing_offset=50,
        correction_data=correction_data,
        inverse_correction_data=inverse_correction_data,
        manufacturer=None,
        model=None,
        output_path="/recordings/",
        selected_camera="Default",
        custom_video_format="MP4",
        video_output="HDMI",
        show_console=False,
        software_trigger_delay=3000,
        volts_to_watts_algorithm="LINEAR",
        volts_to_watts_params="50.0,100.0",
        correction_grid_domain_shape=None,
        inverse_grid_domain_shape=None,
    )


def _mock_sfcf(laser_num: int) -> ScanFieldCorrectionFile:
    return ScanFieldCorrectionFile(
        document_name=f"mock_laser_{laser_num}.fc3",
        document_id=str(uuid.uuid4()),
        file_size=1024,
        valid_as_of_date=_now_iso(),
        document_created_at=None,
        document_type="Scan Field Correction File",
        original_uri=None,
    )


def _mock_train(
    index: int,
    n_lasers: int,
    include_clearbox: bool,
    working_distance_unit: str,
) -> OpticalTrain:
    """Generate one synthetic optical train."""
    # Mirror offsets: laser 0 left, laser 1 right, etc.
    sign = -1 if index % 2 == 0 else 1
    offset_x = sign * 87.5
    offset_y = sign * -23.5
    rotation = 0.0 if index % 2 == 0 else 180.0

    scanner = Scanner(
        manufacturer="MockCo",
        model="MockScan",
        serial_number=f"MOCK-SC-{index + 1:02d}",
        working_distance=670.0,
        working_distance_unit=working_distance_unit,
        scan_field_x=600.0,
        scan_field_x_unit="mm",
        scan_field_y=600.0,
        scan_field_y_unit="mm",
        scan_field_z=76.5,
        scan_field_z_unit="mm",
        scan_head_offset_x=offset_x,
        scan_head_offset_x_unit="mm",
        scan_head_offset_y=offset_y,
        scan_head_offset_y_unit="mm",
        scan_head_offset_z=-1.0,
        scan_head_offset_z_unit="mm",
        scan_head_rotation=rotation,
        scan_head_rotation_unit="degrees",
        axis_configuration="3D",
        x_axis=_mock_axis(),
        y_axis=_mock_axis(),
        z_axis=_mock_axis(),
    )

    light_source = LightSource(
        manufacturer="MockLaser",
        model="MockFiber-1070",
        serial_number=f"MOCK-LS-{index + 1:02d}",
        wavelength=1070.0,
        wavelength_unit="nm",
        power_max_nominal=1000.0,
        power_max_nominal_unit="W",
        power_max_actual=1020.0,
        power_max_actual_unit="W",
        power_min_actual=100.0,
        power_min_actual_unit="W",
        power_min_nominal=None,
        power_min_nominal_unit="W",
        power_bit_resolution=None,
        power_bit_resolution_unit="bits",
        watts_to_volts_algorithm="LINEAR",
        watts_to_volts_params="[1,100,10,1000]",
    )

    collimator = Collimator(
        manufacturer="MockOptics",
        model="D50_F120",
        serial_number=f"MOCK-COL-{index + 1:02d}",
        focal_length=120.0,
        focal_length_unit="mm",
    )

    scanner_card = ScannerCard(
        manufacturer="Raylase",
        model="SP-ICE-3",
        serial_number=f"MOCK-SC-CARD-{index + 1:02d}",
        communication_protocol="SL2-100",
        sample_period=10.0,
        sample_period_unit="μs",
    )

    clearbox = _mock_clearbox(f"192.168.1.{10 + index}", f"{index + 1:03d}") if include_clearbox else None
    sfcf     = _mock_sfcf(index + 1) if include_clearbox else None

    return OpticalTrain(
        train_id=f"Optical_Train_{index + 1:02d}",
        id=None,
        beam_profile_type=None,
        beam_waist_definition="knife-edge",
        beam_waist_major=67.0,
        beam_waist_major_unit="μm",
        beam_waist_minor=68.0,
        beam_waist_minor_unit="μm",
        beam_waist_offset_z=0.5,
        beam_waist_offset_z_unit="mm",
        build_plane_offset_major=0.2,
        build_plane_offset_major_unit="mm",
        build_plane_offset_minor=0.7,
        build_plane_offset_minor_unit="mm",
        collimator_focal_length=120.0,
        collimator_focal_length_unit="mm",
        m2_major=1.05,
        m2_minor=1.08,
        major_axis_angle=0.0,
        major_axis_angle_unit="degrees",
        rayleigh_length_major=3.1,
        rayleigh_length_major_unit="mm",
        rayleigh_length_minor=3.2,
        rayleigh_length_minor_unit="mm",
        scanner_number=None,
        thermal_lensing_passed=False,
        thermal_lensing_focal_plane_shift=1.0,
        thermal_lensing_focal_plane_shift_unit="mm",
        thermal_lensing_threshold=0.75,
        thermal_lensing_threshold_unit="mm",
        scanner=scanner,
        light_source=light_source,
        collimator=collimator,
        scanner_card=scanner_card,
        optional_components=OptionalComponents(clearbox=clearbox),
        scan_field_correction_file=sfcf,
    )


# ---------------------------------------------------------------------------
# MockConfigBuilder
# ---------------------------------------------------------------------------

class MockConfigBuilder:
    """Generate a plausible synthetic `.h5` machine config for testing.

    Parameters
    ----------
    n_lasers : int
        Number of optical trains (1 or 2).
    build_plate_x, build_plate_y, build_plate_z : float
        Build volume dimensions in mm.
    include_clearbox : bool
        Whether to include the Optional_Components/ClearBox group.
    file_version : str
        Value to write for File_Version root attribute.
    working_distance_unit : str
        Unit written for Working_Distance — change to test Rule 8 unit locking.
    machine_name, manufacturer, model, serial_number : str
        Machine identity fields.
    """

    def __init__(
        self,
        n_lasers: int = 2,
        build_plate_x: float = 250.0,
        build_plate_y: float = 250.0,
        build_plate_z: float = 20.0,
        include_clearbox: bool = True,
        file_version: str = "1.0",
        working_distance_unit: str = "mm",
        machine_name: str = "MockMachine",
        manufacturer: str = "MockCo",
        model: str = "MockMIDI+",
        serial_number: str = "MOCK-001",
    ) -> None:
        self.n_lasers            = n_lasers
        self.build_plate_x       = build_plate_x
        self.build_plate_y       = build_plate_y
        self.build_plate_z       = build_plate_z
        self.include_clearbox    = include_clearbox
        self.file_version        = file_version
        self.working_distance_unit = working_distance_unit
        self.machine_name        = machine_name
        self.manufacturer        = manufacturer
        self.model               = model
        self.serial_number       = serial_number
        self._config_override: Optional[MachineConfig] = None

    @classmethod
    def from_config(cls, config: MachineConfig) -> "MockConfigBuilder":
        """Wrap an existing :class:`MachineConfig` for roundtrip testing."""
        instance = cls.__new__(cls)
        instance._config_override = config
        return instance

    def build(self) -> MachineConfig:
        """Return a :class:`MachineConfig` with synthetic values."""
        if self._config_override is not None:
            return self._config_override

        bp = BuildPlate(
            x=float(self.build_plate_x), x_unit="mm",
            y=float(self.build_plate_y), y_unit="mm",
            z=float(self.build_plate_z), z_unit="mm",
            corner_radius=None, corner_radius_unit=None,
        )
        machine = Machine(
            id=str(uuid.uuid4()),
            machine_name=self.machine_name,
            manufacturer=self.manufacturer,
            model=self.model,
            serial_number=self.serial_number,
            build_plate=bp,
            gas_flow_direction="Y+",
            recoat_direction="X+",
        )
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=self.machine_name,
            manufacturer=self.manufacturer,
            model=self.model,
            serial_number=self.serial_number,
            file_version=self.file_version,
            export_date=_now_iso(),
            configuration_hash="0" * 64,
        )
        trains = [
            _mock_train(i, self.n_lasers, self.include_clearbox, self.working_distance_unit)
            for i in range(self.n_lasers)
        ]
        return MachineConfig(meta=meta, machine=machine, optical_trains=trains)

    def save(self, path: str | Path) -> None:
        """Write the config to *path*.  Correction grids are Gaussian (non-zero)."""
        config = self.build()
        MachineConfigWriter(config).write(path)
        # Patch ClearBox correction datasets with non-zero Gaussian data.
        if getattr(self, "include_clearbox", True) and getattr(self, "_config_override", None) is None:
            with h5py.File(path, "a") as f:
                for i in range(self.n_lasers):
                    cb_path = (
                        f"Machine/Optical_Trains/Optical_Train_{i + 1:02d}"
                        "/Optional_Components/ClearBox"
                    )
                    if cb_path in f:
                        grid = _gaussian_correction_grid()
                        del f[f"{cb_path}/Correction_Data"]
                        del f[f"{cb_path}/Inverse_Correction_Data"]
                        f.create_dataset(f"{cb_path}/Correction_Data",         data=grid)
                        f.create_dataset(f"{cb_path}/Inverse_Correction_Data", data=grid * 0.9)


# ---------------------------------------------------------------------------
# YamlConfigBuilder
# ---------------------------------------------------------------------------

class YamlConfigBuilder:
    """Build an HDF5 config from a minimal YAML specification file.

    Only the fields present in the YAML are set; everything else gets a
    sensible default or ``None``.
    """

    def __init__(self, spec_path: str | Path) -> None:
        self._spec: dict = yaml.safe_load(Path(spec_path).read_text(encoding="utf-8"))

    def build(self) -> MachineConfig:
        sm = self._spec.get("machine", {})
        machine_name = sm.get("name", sm.get("machine_name", "Unknown"))

        bp = BuildPlate(
            x=float(sm.get("build_plate_x", 250.0)), x_unit="mm",
            y=float(sm.get("build_plate_y", 250.0)), y_unit="mm",
            z=float(sm.get("build_plate_z", 20.0)),  z_unit="mm",
            corner_radius=sm.get("build_plate_radius"),
            corner_radius_unit="mm" if sm.get("build_plate_radius") is not None else None,
        )
        machine = Machine(
            id=None,
            machine_name=machine_name,
            manufacturer=sm.get("manufacturer", ""),
            model=sm.get("model", ""),
            serial_number=sm.get("serial_number", ""),
            build_plate=bp,
            gas_flow_direction=sm.get("gas_flow_direction"),
            recoat_direction=sm.get("recoat_direction"),
        )
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=machine_name,
            manufacturer=machine.manufacturer,
            model=machine.model,
            serial_number=machine.serial_number,
            file_version="1.0",
            export_date=_now_iso(),
            configuration_hash="0" * 64,
        )
        trains = [
            self._build_train(i, spec)
            for i, spec in enumerate(self._spec.get("optical_trains", []))
        ]
        return MachineConfig(meta=meta, machine=machine, optical_trains=trains)

    def _build_train(self, index: int, spec: dict) -> OpticalTrain:
        ss = spec.get("scanner", {})
        ls = spec.get("light_source", {})

        def _fv(d: dict, key: str) -> Optional[float]:
            v = d.get(key)
            return float(v) if v is not None else None

        scanner = Scanner(
            manufacturer=ss.get("manufacturer", ""),
            model=ss.get("model", ""),
            serial_number=ss.get("serial_number", ""),
            working_distance=_fv(ss, "working_distance"),
            working_distance_unit="mm",
            scan_field_x=_fv(ss, "scan_field_x"),
            scan_field_x_unit="mm" if ss.get("scan_field_x") is not None else None,
            scan_field_y=_fv(ss, "scan_field_y"),
            scan_field_y_unit="mm" if ss.get("scan_field_y") is not None else None,
            scan_field_z=_fv(ss, "scan_field_z"),
            scan_field_z_unit="mm" if ss.get("scan_field_z") is not None else None,
            scan_head_offset_x=_fv(ss, "scan_head_offset_x"),
            scan_head_offset_x_unit="mm" if ss.get("scan_head_offset_x") is not None else None,
            scan_head_offset_y=_fv(ss, "scan_head_offset_y"),
            scan_head_offset_y_unit="mm" if ss.get("scan_head_offset_y") is not None else None,
            scan_head_offset_z=_fv(ss, "scan_head_offset_z"),
            scan_head_offset_z_unit="mm" if ss.get("scan_head_offset_z") is not None else None,
            scan_head_rotation=_fv(ss, "scan_head_rotation"),
            scan_head_rotation_unit="degrees" if ss.get("scan_head_rotation") is not None else None,
            axis_configuration=ss.get("axis_configuration"),
            x_axis=_mock_axis(),
            y_axis=_mock_axis(),
            z_axis=_mock_axis() if ss.get("axis_configuration") in ("3D", "3D+Focus", None) else None,
            focus=_mock_axis() if ss.get("axis_configuration") == "3D+Focus" else None,
        )

        light_source = LightSource(
            manufacturer=ls.get("manufacturer", ""),
            model=ls.get("model", ""),
            serial_number=ls.get("serial_number", ""),
            wavelength=_fv(ls, "wavelength"),
            wavelength_unit="nm" if ls.get("wavelength") is not None else None,
            power_max_nominal=_fv(ls, "power_max_nominal"),
            power_max_nominal_unit="W" if ls.get("power_max_nominal") is not None else None,
            power_max_actual=_fv(ls, "power_max_actual"),
            power_max_actual_unit="W" if ls.get("power_max_actual") is not None else None,
            power_min_actual=_fv(ls, "power_min_actual"),
            power_min_actual_unit="W" if ls.get("power_min_actual") is not None else None,
            power_min_nominal=_fv(ls, "power_min_nominal"),
            power_min_nominal_unit="W" if ls.get("power_min_nominal") is not None else None,
            power_bit_resolution=None,
            power_bit_resolution_unit=None,
            watts_to_volts_algorithm=ls.get("watts_to_volts_algorithm"),
            watts_to_volts_params=ls.get("watts_to_volts_params"),
        )

        col_s = spec.get("collimator", {})
        collimator = Collimator(
            manufacturer=col_s.get("manufacturer", ""),
            model=col_s.get("model", ""),
            serial_number=col_s.get("serial_number", ""),
            focal_length=_fv(col_s, "focal_length"),
            focal_length_unit="mm" if col_s.get("focal_length") is not None else None,
        )

        sc_s = spec.get("scanner_card", {})
        scanner_card = ScannerCard(
            manufacturer=sc_s.get("manufacturer", ""),
            model=sc_s.get("model", ""),
            serial_number=sc_s.get("serial_number", ""),
            communication_protocol=sc_s.get("communication_protocol"),
            sample_period=_fv(sc_s, "sample_period"),
            sample_period_unit="μs" if sc_s.get("sample_period") is not None else None,
        )

        return OpticalTrain(
            train_id=f"Optical_Train_{index + 1:02d}",
            id=None,
            beam_profile_type=None,
            beam_waist_definition=None,
            beam_waist_major=None, beam_waist_major_unit=None,
            beam_waist_minor=None, beam_waist_minor_unit=None,
            beam_waist_offset_z=None, beam_waist_offset_z_unit=None,
            build_plane_offset_major=None, build_plane_offset_major_unit=None,
            build_plane_offset_minor=None, build_plane_offset_minor_unit=None,
            collimator_focal_length=None, collimator_focal_length_unit=None,
            m2_major=None, m2_minor=None,
            major_axis_angle=None, major_axis_angle_unit=None,
            rayleigh_length_major=None, rayleigh_length_major_unit=None,
            rayleigh_length_minor=None, rayleigh_length_minor_unit=None,
            scanner_number=None,
            thermal_lensing_passed=None,
            thermal_lensing_focal_plane_shift=None, thermal_lensing_focal_plane_shift_unit=None,
            thermal_lensing_threshold=None, thermal_lensing_threshold_unit=None,
            scanner=scanner,
            light_source=light_source,
            collimator=collimator,
            scanner_card=scanner_card,
            optional_components=OptionalComponents(clearbox=None),
            scan_field_correction_file=None,
        )

    def save(self, path: str | Path) -> None:
        MachineConfigWriter(self.build()).write(path)


# ---------------------------------------------------------------------------
# ConfigEditor
# ---------------------------------------------------------------------------

class ConfigEditor:
    """Read an existing config, apply targeted modifications, and write back.

    Uses :func:`dataclasses.replace` for immutable field updates so the
    original file is never modified in place.

    Example::

        editor = ConfigEditor("machine_A.h5")
        editor.set_scanner_offset(train_index=0, x=-90.0, y=25.0)
        editor.save("machine_A_adjusted.h5")
    """

    def __init__(self, path: str | Path) -> None:
        self._config: MachineConfig = MachineConfigReader(path).parse()

    @property
    def config(self) -> MachineConfig:
        """The current (possibly modified) :class:`MachineConfig`."""
        return self._config

    def set_scanner_offset(
        self,
        train_index: int,
        x: float,
        y: float,
        z: Optional[float] = None,
    ) -> None:
        """Update scan-head offsets for optical train *train_index* (0-based)."""
        train = self._config.optical_trains[train_index]
        kw: dict = {"scan_head_offset_x": x, "scan_head_offset_y": y}
        if z is not None:
            kw["scan_head_offset_z"] = z
        new_scanner = replace(train.scanner, **kw)
        new_train   = replace(train, scanner=new_scanner)
        trains = list(self._config.optical_trains)
        trains[train_index] = new_train
        self._config = replace(self._config, optical_trains=trains)

    def save(self, path: str | Path) -> None:
        """Write the current (possibly modified) config to *path*."""
        MachineConfigWriter(self._config).write(path)
