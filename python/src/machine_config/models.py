from __future__ import annotations
from dataclasses import dataclass
from typing import Optional


@dataclass
class ScanFieldCorrectionFile:
    document_name: str
    document_id: str
    file_size: int
    valid_as_of_date: str
    document_created_at: Optional[str]
    document_type: Optional[str]
    original_uri: Optional[str]


@dataclass
class ClearBox:
    ip_address: str
    serial_number: Optional[str]
    data_port: Optional[int]
    server_port: Optional[int]
    actual_timing_offset: Optional[int]
    commanded_timing_offset: Optional[int]
    correction_data_shape: tuple[int, int, int]          # e.g. (257, 257, 2)
    inverse_correction_data_shape: tuple[int, int, int]  # same shape
    manufacturer: Optional[str]
    model: Optional[str]
    output_path: Optional[str]
    selected_camera: Optional[str]
    custom_video_format: Optional[str]
    video_output: Optional[str]
    show_console: Optional[bool]                         # HDF5 int 0/1
    software_trigger_delay: Optional[int]
    volts_to_watts_algorithm: Optional[str]
    volts_to_watts_params: Optional[str]
    correction_grid_domain_shape: Optional[str]
    inverse_grid_domain_shape: Optional[str]


@dataclass
class Collimator:
    manufacturer: str
    model: str
    serial_number: str
    focal_length: Optional[float]
    focal_length_unit: Optional[str]


@dataclass
class ScannerCard:
    manufacturer: str
    model: str
    serial_number: str
    communication_protocol: Optional[str]
    sample_period: Optional[float]
    sample_period_unit: Optional[str]


@dataclass
class Scanner:
    manufacturer: str
    model: str
    serial_number: str
    working_distance: Optional[float]
    working_distance_unit: Optional[str]
    scan_field_x: Optional[float]
    scan_field_x_unit: Optional[str]
    scan_field_y: Optional[float]
    scan_field_y_unit: Optional[str]
    scan_field_z: Optional[float]
    scan_field_z_unit: Optional[str]
    scan_head_offset_x: Optional[float]
    scan_head_offset_x_unit: Optional[str]
    scan_head_offset_y: Optional[float]
    scan_head_offset_y_unit: Optional[str]
    scan_head_offset_z: Optional[float]
    scan_head_offset_z_unit: Optional[str]
    scan_head_rotation: Optional[float]
    scan_head_rotation_unit: Optional[str]
    axis_configuration: Optional[str]


@dataclass
class LightSource:
    manufacturer: str
    model: str
    serial_number: str
    wavelength: Optional[float]
    wavelength_unit: Optional[str]
    power_max_nominal: Optional[float]
    power_max_nominal_unit: Optional[str]
    power_max_actual: Optional[float]
    power_max_actual_unit: Optional[str]
    power_min_actual: Optional[float]
    power_min_actual_unit: Optional[str]
    power_min_nominal: Optional[float]
    power_min_nominal_unit: Optional[str]
    power_bit_resolution: Optional[float]                # HDF5 stores as string; parsed via _read_float
    power_bit_resolution_unit: Optional[str]
    watts_to_volts_algorithm: Optional[str]
    watts_to_volts_params: Optional[str]


@dataclass
class OpticalTrain:
    train_id: str
    id: Optional[str]                                         # UUID attribute on the train group (often empty)
    beam_profile_type: Optional[str]
    beam_waist_definition: Optional[str]
    beam_waist_major: Optional[float]
    beam_waist_major_unit: Optional[str]
    beam_waist_minor: Optional[float]
    beam_waist_minor_unit: Optional[str]
    beam_waist_offset_z: Optional[float]
    beam_waist_offset_z_unit: Optional[str]
    m2_major: Optional[float]
    m2_minor: Optional[float]
    rayleigh_length_major: Optional[float]
    rayleigh_length_major_unit: Optional[str]
    rayleigh_length_minor: Optional[float]
    rayleigh_length_minor_unit: Optional[str]
    build_plane_offset_major: Optional[float]
    build_plane_offset_major_unit: Optional[str]
    build_plane_offset_minor: Optional[float]
    build_plane_offset_minor_unit: Optional[str]
    collimator_focal_length: Optional[float]                      # train-level duplicate of Collimator.focal_length
    collimator_focal_length_unit: Optional[str]
    major_axis_angle: Optional[float]
    major_axis_angle_unit: Optional[str]
    scanner_number: Optional[str]
    thermal_lensing_passed: Optional[bool]                        # HDF5 int 0/1; None if absent
    thermal_lensing_focal_plane_shift: Optional[float]
    thermal_lensing_focal_plane_shift_unit: Optional[str]
    thermal_lensing_threshold: Optional[float]
    thermal_lensing_threshold_unit: Optional[str]
    scanner: Scanner
    light_source: LightSource
    collimator: Collimator                                        # required; see Rule 7
    scanner_card: ScannerCard                                     # required; see Rule 7
    clearbox: Optional[ClearBox]
    scan_field_correction_file: Optional[ScanFieldCorrectionFile]


@dataclass
class BuildPlate:
    x: Optional[float]
    x_unit: Optional[str]
    y: Optional[float]
    y_unit: Optional[str]
    z: Optional[float]
    z_unit: Optional[str]
    corner_radius: Optional[float]
    corner_radius_unit: Optional[str]


@dataclass
class Machine:
    id: Optional[str]
    machine_name: str
    manufacturer: str
    model: str
    serial_number: str
    build_plate: BuildPlate
    gas_flow_direction: Optional[str]
    recoat_direction: Optional[str]


@dataclass
class MachineConfigMeta:
    schema_version: str
    machine_name: str
    manufacturer: str
    model: str
    serial_number: str
    file_version: str
    export_date: str
    configuration_hash: str


@dataclass
class MachineConfig:
    meta: MachineConfigMeta
    machine: Machine
    optical_trains: list[OpticalTrain]
