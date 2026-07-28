"""Phase 1.3 — HDF5 reader: converts LPBF machine-config .h5 files to MachineConfig objects.

The reader is machine-agnostic. Any machine whose HDF5 export maps its attributes into the
MachineConfig structure (see models.py) can be read. The reference fixture is an AconityMIDI
file, but the library imposes no vendor-specific requirements.
"""
from __future__ import annotations

import json
import base64
import warnings
from pathlib import Path
from typing import Optional

import h5py
import numpy as np

from .models import (
    AxisConfig,
    BuildPlate,
    ClearBox,
    Collimator,
    LightSource,
    Machine,
    MachineConfig,
    MachineConfigMeta,
    OpcuaClientConfig,
    OpcuaConfig,
    OpcuaPipeConfig,
    OpcuaTrigger,
    OpticalTrain,
    ScanFieldCorrectionFile,
    Scanner,
    ScannerCard,
)
from .schema import SCHEMA, SCHEMA_VERSION


_KNOWN_ROOT_KEYS: frozenset[str] = frozenset(
    {"machine_name", "manufacturer", "model", "serial_number",
     "File_Version", "Export_Date", "Configuration_Hash"}
)

# HDF5 attribute names that are modelled as typed fields on OpcuaTrigger.
# Any attribute NOT in this set is collected into OpcuaTrigger.extra.
_KNOWN_TRIGGER_KEYS: frozenset[str] = frozenset(
    {"ID", "Signal", "Subsystem", "Rule_Enabled", "Start_Value", "Stop_Value"}
)

# HDF5 attribute names that are modelled as typed fields on OpcuaClientConfig.
# Any attribute NOT in this set is collected into OpcuaClientConfig.extra.
_KNOWN_CLIENT_KEYS: frozenset[str] = frozenset(
    {"Server_URL", "Auth_Mode", "Security_Mode", "Security_Policy",
     "BFS_Max_Depth", "Publish_Interval", "Sampling_Interval", "Session_Timeout"}
)

# HDF5 attribute names that are modelled as typed fields on OpcuaPipeConfig.
# Any attribute NOT in this set is collected into OpcuaPipeConfig.extra.
_KNOWN_PIPE_KEYS: frozenset[str] = frozenset(
    {"Pipe_Enabled", "Buffer_Size"}
)


def _hdf5_attrs_extra(attrs, known: frozenset[str]) -> dict:
    """Return all HDF5 attrs not in *known*, converting numpy scalars to Python natives."""
    import numpy as np
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


class MachineConfigReader:
    """Read a machine-config HDF5 file into a :class:`MachineConfig`.

    The reader is machine-agnostic — any machine that maps its attributes into
    the MachineConfig structure can be parsed. Pass the path to any conforming
    ``.h5`` file.
    """

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def parse(self) -> MachineConfig:
        """Open the HDF5 file and return a fully-populated :class:`MachineConfig`."""
        with h5py.File(self.path, "r") as f:
            self._check_file_version(f)
            return self._parse(f)

    def get_correction_data(self, train_index: int) -> np.ndarray:
        """Return the ``(257, 257, 2)`` float64 galvo correction grid (0-indexed train)."""
        with h5py.File(self.path, "r") as f:
            return f[self._clearbox_path(train_index) + "/Correction_Data"][:]

    def get_inverse_correction_data(self, train_index: int) -> np.ndarray:
        """Return the ``(257, 257, 2)`` float64 inverse correction grid (0-indexed train)."""
        with h5py.File(self.path, "r") as f:
            return f[self._clearbox_path(train_index) + "/Inverse_Correction_Data"][:]

    def get_scan_field_correction_bytes(self, train_index: int) -> bytes:
        """Return the raw ``.fc3`` correction-file bytes embedded as a uint8 dataset."""
        with h5py.File(self.path, "r") as f:
            train_id = f"Optical_Train_{train_index + 1:02d}"
            path = f"Machine/Optical_Trains/{train_id}/scan_field_correction_file"
            return bytes(f[path][:])

    def get_raw_group(self, hdf5_path: str) -> dict:
        """Return all attributes of an arbitrary HDF5 path as a plain dict.

        Returns an empty dict (not an error) if the path does not exist.
        Useful for non-schema groups such as ``OPCUA`` or scanner sub-axes.
        """
        with h5py.File(self.path, "r") as f:
            if hdf5_path not in f:
                return {}
            return dict(f[hdf5_path].attrs)

    def to_json(self, indent: int = 2, include_binary: bool = False) -> str:
        """Parse and return the canonical JSON representation.

        Args:
            indent: JSON indentation level.
            include_binary: When ``True``, correction arrays
                (``correction_data``, ``inverse_correction_data``) and the
                raw ``.fc3`` bytes (``raw_bytes``) are included in the output.
                Defaults to ``False`` — the JSON contains only scalar/metadata
                fields, keeping the output compact and human-readable.
                The HDF5 file remains the source of truth for binary data;
                use :meth:`get_correction_data` and
                :meth:`get_scan_field_correction_bytes` to access it.
        """
        config = self.parse()
        return json.dumps(
            self._config_to_dict(config, include_binary=include_binary),
            indent=indent,
            default=str,
            ensure_ascii=False,
        )

    # ------------------------------------------------------------------
    # Attribute-reading helpers (all static; all attribute access flows here)
    # ------------------------------------------------------------------

    @staticmethod
    def _read_str(attrs, key: str) -> Optional[str]:
        """``None`` if absent or empty after stripping; never raises."""
        val = attrs.get(key)
        if val is None:
            return None
        s = str(val).strip()
        return s if s else None

    @staticmethod
    def _read_float(attrs, key: str) -> Optional[float]:
        """``None`` if absent or empty string; ``ValueError`` if non-numeric."""
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, (str, bytes)) and str(val).strip() == "":
            return None
        return float(val)

    @staticmethod
    def _read_int(attrs, key: str) -> Optional[int]:
        """``None`` if absent or empty string."""
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, (str, bytes)) and str(val).strip() == "":
            return None
        return int(val)

    @staticmethod
    def _read_bool_from_int(attrs, key: str) -> Optional[bool]:
        """Converts HDF5 integer ``0``/``1`` to ``bool``. ``ValueError`` for other values."""
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
        """Read a ``_unit`` attribute and assert it matches the locked expected value.

        Rule 8: unit attributes are locked at parse time.  Returns ``None`` if the
        attribute is absent or blank.  Raises ``ValueError`` if present but wrong.
        """
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

    def _check_file_version(self, f: h5py.File) -> None:
        version = str(f.attrs.get("File_Version", "")).strip()
        if version != "1.0":
            warnings.warn(
                f"File_Version is {version!r}; this reader targets '1.0'. "
                "Output may be incomplete or incorrect.",
                UserWarning,
                stacklevel=3,
            )

    def _clearbox_path(self, train_index: int) -> str:
        tid = f"Optical_Train_{train_index + 1:02d}"
        return f"Machine/Optical_Trains/{tid}/Optional_Components/ClearBox"

    def _parse(self, f: h5py.File) -> MachineConfig:
        # ---- root attributes → MachineConfigMeta --------------------------------
        meta = MachineConfigMeta(
            schema_version=SCHEMA_VERSION,
            machine_name=str(f.attrs.get("machine_name", "")),
            manufacturer=str(f.attrs.get("manufacturer", "")),
            model=str(f.attrs.get("model", "")),
            serial_number=str(f.attrs.get("serial_number", "")),
            file_version=str(f.attrs.get("File_Version", "")),
            export_date=str(f.attrs.get("Export_Date", "")),
            configuration_hash=str(f.attrs.get("Configuration_Hash", "")),
            extra=_hdf5_attrs_extra(f.attrs, _KNOWN_ROOT_KEYS),
        )

        # ---- Machine group → Machine + BuildPlate --------------------------------
        ma = f["Machine"].attrs
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
        )

        # ---- Optical trains -------------------------------------------------------
        trains_grp = f["Machine/Optical_Trains"]
        train_ids = sorted(
            k for k in trains_grp.keys() if k.startswith("Optical_Train_")
        )
        optical_trains = [self._parse_train(f, tid) for tid in train_ids]

        # ---- Optional OPCUA group ------------------------------------------------
        opcua = self._parse_opcua(f)

        return MachineConfig(
            meta=meta,
            machine=machine,
            optical_trains=optical_trains,
            opcua=opcua,
        )

    def _parse_train(self, f: h5py.File, train_id: str) -> OpticalTrain:
        base_path = f"Machine/Optical_Trains/{train_id}"
        a = f[base_path].attrs

        scanner = self._parse_scanner(f[f"{base_path}/Scanner"])
        light_source = self._parse_light_source(f[f"{base_path}/Light_Source"])
        collimator = self._parse_collimator(f[f"{base_path}/Collimator"])
        scanner_card = self._parse_scanner_card(f[f"{base_path}/Scanner_Card"])

        clearbox_path = f"{base_path}/Optional_Components/ClearBox"
        clearbox = self._parse_clearbox(f[clearbox_path]) if clearbox_path in f else None

        sfcf_path = f"{base_path}/scan_field_correction_file"
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
            clearbox=clearbox,
            scan_field_correction_file=sfcf,
        )

    def _parse_axis(self, grp: h5py.Group) -> AxisConfig:
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
            tuning_parameters=self._read_str(a, "Tuning_Parameters"),
            tuning_type=self._read_str(a, "Tuning_Type"),
        )

    def _parse_scanner(self, grp: h5py.Group) -> Scanner:
        a = grp.attrs
        axis_cfg = self._read_str(a, "Axis_Configuration")
        x_axis = self._parse_axis(grp["X_Axis"])
        y_axis = self._parse_axis(grp["Y_Axis"])
        z_axis = self._parse_axis(grp["Z_Axis"]) if "Z_Axis" in grp else None
        focus  = self._parse_axis(grp["Focus"])  if "Focus"  in grp else None
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
        )

    def _parse_light_source(self, grp: h5py.Group) -> LightSource:
        a = grp.attrs
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
            watts_to_volts_algorithm=self._read_str(a, "Watts_To_Volts_Algorithm"),
            watts_to_volts_params=self._read_str(a, "Watts_To_Volts_Params"),
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

    @staticmethod
    def _nan_array_to_list(arr: np.ndarray) -> list:
        """Convert float64 ndarray to nested Python list, mapping NaN → None."""
        obj = arr.astype(object)
        obj[np.isnan(arr)] = None
        return obj.tolist()

    def _parse_clearbox(self, grp: h5py.Group) -> ClearBox:
        a = grp.attrs
        corr_data = self._nan_array_to_list(grp["Correction_Data"][:])
        inv_data  = self._nan_array_to_list(grp["Inverse_Correction_Data"][:])
        return ClearBox(
            ip_address=str(a.get("Ip_Address", "")),
            serial_number=self._read_str(a, "Serial_Number"),
            data_port=self._read_int(a, "Data_Port"),
            server_port=self._read_int(a, "Server_Port"),
            actual_timing_offset=self._read_int(a, "Actual_Timing_Offset"),
            commanded_timing_offset=self._read_int(a, "Commanded_Timing_Offset"),
            correction_data=corr_data,
            inverse_correction_data=inv_data,
            manufacturer=self._read_str(a, "Manufacturer"),
            model=self._read_str(a, "Model"),
            output_path=self._read_str(a, "Output_Path"),
            selected_camera=self._read_str(a, "Selected_Camera"),
            custom_video_format=self._read_str(a, "Custom_Video_Format"),
            video_output=self._read_str(a, "Video_Output"),
            show_console=self._read_bool_from_int(a, "Show_Console"),
            software_trigger_delay=self._read_int(a, "Software_Trigger_Delay"),
            volts_to_watts_algorithm=self._read_str(a, "Volts_To_Watts_Algorithm"),
            volts_to_watts_params=self._read_str(a, "Volts_To_Watts_Params"),
            correction_grid_domain_shape=self._read_str(
                a, "Correction_Grid_Domain_Shape"
            ),
            inverse_grid_domain_shape=self._read_str(a, "Inverse_Grid_Domain_Shape"),
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

    def _parse_opcua(self, f: h5py.File) -> Optional[OpcuaConfig]:
        """Parse the optional OPCUA group tree into an :class:`OpcuaConfig`.

        Returns ``None`` if the file has no ``OPCUA`` group (most machine configs).
        """
        if "OPCUA" not in f:
            return None

        # ---- Client --------------------------------------------------------------
        ca = f["OPCUA/Client"].attrs
        client = OpcuaClientConfig(
            server_url=str(ca.get("Server_URL", "")),
            auth_mode=str(ca.get("Auth_Mode", "")),
            security_mode=str(ca.get("Security_Mode", "")),
            security_policy=str(ca.get("Security_Policy", "")),
            bfs_max_depth=int(ca.get("BFS_Max_Depth", 0)),
            publish_interval=int(ca.get("Publish_Interval", 0)),
            sampling_interval=int(ca.get("Sampling_Interval", 0)),
            session_timeout=int(ca.get("Session_Timeout", 0)),
            extra=_hdf5_attrs_extra(ca, _KNOWN_CLIENT_KEYS),
        )

        # ---- Pipe ----------------------------------------------------------------
        pa = f["OPCUA/Pipe"].attrs
        pipe = OpcuaPipeConfig(
            pipe_enabled=bool(int(pa.get("Pipe_Enabled", 0))),
            buffer_size=int(pa.get("Buffer_Size", 0)),
            extra=_hdf5_attrs_extra(pa, _KNOWN_PIPE_KEYS),
        )

        # ---- Triggers ------------------------------------------------------------
        triggers_grp = f["OPCUA/Triggers"]
        triggers_enabled = self._read_bool_from_int(triggers_grp.attrs, "Triggers_Enabled")

        triggers: dict[str, OpcuaTrigger] = {}
        for name in triggers_grp.keys():
            ta = triggers_grp[name].attrs
            extra = _hdf5_attrs_extra(ta, _KNOWN_TRIGGER_KEYS)
            triggers[name] = OpcuaTrigger(
                id=self._read_str(ta, "ID"),
                signal=self._read_str(ta, "Signal"),
                subsystem=self._read_str(ta, "Subsystem"),
                rule_enabled=self._read_bool_from_int(ta, "Rule_Enabled"),
                start_value=self._read_str(ta, "Start_Value"),
                stop_value=self._read_str(ta, "Stop_Value"),
                extra=extra,
            )

        return OpcuaConfig(
            client=client,
            pipe=pipe,
            triggers=triggers,
            triggers_enabled=triggers_enabled,
        )

    # ------------------------------------------------------------------
    # JSON serialisation helpers
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
            },
            "optical_trains": [
                self._train_to_dict(t, include_binary=include_binary)
                for t in config.optical_trains
            ],
        }
        if config.opcua is not None:
            result["opcua"] = self._opcua_to_dict(config.opcua)
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
            "clearbox": self._clearbox_to_dict(train.clearbox, include_binary=include_binary),
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
        return {
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

    def _light_source_to_dict(self, ls: LightSource) -> dict:
        return {
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
            "watts_to_volts_algorithm": ls.watts_to_volts_algorithm,
            "watts_to_volts_params": ls.watts_to_volts_params,
        }

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

    def _clearbox_to_dict(
        self, cb: Optional[ClearBox], include_binary: bool = False
    ) -> Optional[dict]:
        if cb is None:
            return None
        d: dict = {
            "ip_address": cb.ip_address,
            "serial_number": cb.serial_number,
            "data_port": cb.data_port,
            "server_port": cb.server_port,
            "actual_timing_offset": cb.actual_timing_offset,
            "commanded_timing_offset": cb.commanded_timing_offset,
            "manufacturer": cb.manufacturer,
            "model": cb.model,
            "output_path": cb.output_path,
            "selected_camera": cb.selected_camera,
            "custom_video_format": cb.custom_video_format,
            "video_output": cb.video_output,
            "show_console": cb.show_console,
            "software_trigger_delay": cb.software_trigger_delay,
            "volts_to_watts_algorithm": cb.volts_to_watts_algorithm,
            "volts_to_watts_params": cb.volts_to_watts_params,
            "correction_grid_domain_shape": cb.correction_grid_domain_shape,
            "inverse_grid_domain_shape": cb.inverse_grid_domain_shape,
        }
        if include_binary:
            d["correction_data"] = cb.correction_data
            d["inverse_correction_data"] = cb.inverse_correction_data
        return d

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

    def _opcua_to_dict(self, opcua: OpcuaConfig) -> dict:
        return {
            "client": {
                "server_url": opcua.client.server_url,
                "auth_mode": opcua.client.auth_mode,
                "security_mode": opcua.client.security_mode,
                "security_policy": opcua.client.security_policy,
                "bfs_max_depth": opcua.client.bfs_max_depth,
                "publish_interval": opcua.client.publish_interval,
                "sampling_interval": opcua.client.sampling_interval,
                "session_timeout": opcua.client.session_timeout,
                "extra": opcua.client.extra,
            },
            "pipe": {
                "pipe_enabled": opcua.pipe.pipe_enabled,
                "buffer_size": opcua.pipe.buffer_size,
                "extra": opcua.pipe.extra,
            },
            "triggers_enabled": opcua.triggers_enabled,
            "triggers": {
                name: {
                    "id": t.id,
                    "signal": t.signal,
                    "subsystem": t.subsystem,
                    "rule_enabled": t.rule_enabled,
                    "start_value": t.start_value,
                    "stop_value": t.stop_value,
                    "extra": t.extra,
                }
                for name, t in opcua.triggers.items()
            },
        }


# ---------------------------------------------------------------------------
# JSON → MachineConfig deserialiser (inverse of MachineConfigReader._config_to_dict)
# ---------------------------------------------------------------------------

def config_from_dict(d: dict) -> MachineConfig:
    """Reconstruct a :class:`MachineConfig` from a canonical JSON-compatible dict.

    This is the inverse of :meth:`MachineConfigReader._config_to_dict` and is
    used by the CLI ``write`` command and any other code that needs to go from
    JSON → HDF5.
    """
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
    )

    optical_trains = [_train_from_dict(t) for t in d["optical_trains"]]
    opcua_d = d.get("opcua")
    opcua = _opcua_from_dict(opcua_d) if opcua_d is not None else None
    return MachineConfig(meta=meta, machine=machine, optical_trains=optical_trains, opcua=opcua)


def _train_from_dict(t: dict) -> OpticalTrain:
    s = t["scanner"]
    ls = t["light_source"]
    col = t["collimator"]
    sc = t["scanner_card"]
    cb_d = t.get("clearbox")
    sfcf_d = t.get("scan_field_correction_file")

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

    x_axis_d = s.get("x_axis")
    y_axis_d = s.get("y_axis")
    if x_axis_d is None:
        x_axis_d = {}
    if y_axis_d is None:
        y_axis_d = {}

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
        watts_to_volts_algorithm=ls.get("watts_to_volts_algorithm"),
        watts_to_volts_params=ls.get("watts_to_volts_params"),
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

    clearbox: Optional[ClearBox] = None
    if cb_d is not None:
        clearbox = ClearBox(
            ip_address=cb_d.get("ip_address", ""),
            serial_number=cb_d.get("serial_number"),
            data_port=cb_d.get("data_port"),
            server_port=cb_d.get("server_port"),
            actual_timing_offset=cb_d.get("actual_timing_offset"),
            commanded_timing_offset=cb_d.get("commanded_timing_offset"),
            correction_data=cb_d.get("correction_data"),
            inverse_correction_data=cb_d.get("inverse_correction_data"),
            manufacturer=cb_d.get("manufacturer"),
            model=cb_d.get("model"),
            output_path=cb_d.get("output_path"),
            selected_camera=cb_d.get("selected_camera"),
            custom_video_format=cb_d.get("custom_video_format"),
            video_output=cb_d.get("video_output"),
            show_console=cb_d.get("show_console"),
            software_trigger_delay=cb_d.get("software_trigger_delay"),
            volts_to_watts_algorithm=cb_d.get("volts_to_watts_algorithm"),
            volts_to_watts_params=cb_d.get("volts_to_watts_params"),
            correction_grid_domain_shape=cb_d.get("correction_grid_domain_shape"),
            inverse_grid_domain_shape=cb_d.get("inverse_grid_domain_shape"),
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
        clearbox=clearbox,
        scan_field_correction_file=sfcf,
    )


def _opcua_from_dict(d: dict) -> OpcuaConfig:
    """Reconstruct an :class:`OpcuaConfig` from a canonical JSON-compatible dict."""
    c = d["client"]
    p = d["pipe"]

    client = OpcuaClientConfig(
        server_url=c["server_url"],
        auth_mode=c["auth_mode"],
        security_mode=c["security_mode"],
        security_policy=c["security_policy"],
        bfs_max_depth=c["bfs_max_depth"],
        publish_interval=c["publish_interval"],
        sampling_interval=c["sampling_interval"],
        session_timeout=c["session_timeout"],
        extra=c.get("extra", {}),
    )

    pipe = OpcuaPipeConfig(
        pipe_enabled=p["pipe_enabled"],
        buffer_size=p["buffer_size"],
        extra=p.get("extra", {}),
    )

    triggers: dict[str, OpcuaTrigger] = {}
    for name, td in d.get("triggers", {}).items():
        triggers[name] = OpcuaTrigger(
            id=td.get("id"),
            signal=td.get("signal"),
            subsystem=td.get("subsystem"),
            rule_enabled=td.get("rule_enabled"),
            start_value=td.get("start_value"),
            stop_value=td.get("stop_value"),
            extra=td.get("extra", {}),
        )

    return OpcuaConfig(
        client=client,
        pipe=pipe,
        triggers=triggers,
        triggers_enabled=d.get("triggers_enabled"),
    )

