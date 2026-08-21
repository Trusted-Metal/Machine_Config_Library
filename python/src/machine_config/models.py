from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class ScanFieldCorrectionFile:
    document_name: str
    document_id: str
    file_size: int
    valid_as_of_date: str
    document_created_at: Optional[str]
    document_type: Optional[str]
    original_uri: Optional[str]
    raw_bytes: Optional[bytes] = None  # raw .fc3 binary content


@dataclass
class EquationConstant:
    """One named equation constant for a SynchronousSensor's algorithm_equation
    (e.g. a/b for a Log-Linear fit: log(ppm) = a*mA + b). HDF5 source: one row
    of the Derivation_Equation_Constants compound dataset. A named row rather
    than a positional array so a reader never has to infer which index means
    what from algorithm_type alone, and so adding a constant is an additive
    row rather than an ordering hazard.
    """
    name: str
    value: float


@dataclass
class CalibrationPoint:
    """One calibration sample pair for a SynchronousSensor. HDF5 source: one
    row of the Calibration_Points compound dataset.

    ``input_value`` is in the sensor's input_type unit; ``output_value`` is in
    the sensor's sensor_output_space unit (**not** units_derived_quantity) —
    every row of a given sensor's calibration curve is in the same units by
    construction. Confirmed against the ZR800 reference example: both points
    satisfy algorithm_equation exactly in log-space
    (0.4375 * 4 - 2.75 == -1.0, 0.4375 * 20 - 2.75 == 6.0), not linear ppm.
    """
    input_value: float
    output_value: float


@dataclass
class SynchronousSensor:
    """A synchronous sensor attached to a ClearBox. HDF5 source: one
    sub-group under .../ClearBox/Synchronous_Sensors/<key>/ — see
    ClearBox.synchronous_sensors for what <key> means.
    """
    enabled: Optional[bool]                              # HDF5 int 0/1
    sensor_name: Optional[str]
    sensor_output_range_low: Optional[float]
    sensor_output_range_high: Optional[float]
    sensor_output_space: Optional[str]
    sensor_model: Optional[str]
    sensor_manufacturer: Optional[str]
    sensor_scope: Optional[str]
    units_derived_quantity: Optional[str]
    port_id: Optional[int]
    sensor_type: Optional[str]
    input_type: Optional[str]
    algorithm_type: Optional[str]
    algorithm_equation: Optional[str]
    calibration_source: Optional[str]
    calibration_verified: Optional[bool]                 # HDF5 int 0/1
    sample_period: Optional[float]
    metadata: Optional[str]
    derivation_equation_constants: list[EquationConstant]
    calibration_points: list[CalibrationPoint]


@dataclass
class ClearBox:
    ip_address: str
    serial_number: Optional[str]
    data_port: Optional[int]
    server_port: Optional[int]
    actual_timing_offset: Optional[int]
    commanded_timing_offset: Optional[int]
    correction_data: list[list[list[float | None]]] | None
    inverse_correction_data: list[list[list[float | None]]] | None
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
    # Key = free-form sensor label (HDF5 sub-group name under
    # ClearBox/Synchronous_Sensors/) — an arbitrary value chosen by the
    # file's author, not required to match any attribute inside that
    # sensor's own group (e.g. "Oxygen Sensor", "O2_Port5", anything). No
    # Synchronous_Sensors group on disk is represented identically to an
    # empty dict here — there is no separate "absent" state to track.
    synchronous_sensors: dict[str, SynchronousSensor]


@dataclass
class OptionalComponents:
    """Optional add-on hardware that may or may not be installed on an optical train.

    Maps to the ``Optional_Components`` HDF5 group under each ``Optical_Train_NN``.
    Fields are ``None`` when the corresponding hardware is not present.
    New optional components are added here as the schema grows.
    """
    clearbox: Optional[ClearBox] = None


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
class AxisConfig:
    actual_bit_resolution: Optional[int]
    actual_bit_resolution_unit: Optional[str]
    commanded_bit_resolution: Optional[int]
    commanded_bit_resolution_unit: Optional[str]
    control_type: Optional[str]
    range_of_motion: Optional[float]
    range_of_motion_unit: Optional[str]
    smoothing_kernel: Optional[str]
    smoothing_parameters: Optional[float]
    tuning_parameters: Optional[str]
    tuning_type: Optional[str]


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
    x_axis: AxisConfig
    y_axis: AxisConfig
    z_axis: Optional[AxisConfig] = None
    focus: Optional[AxisConfig] = None

    def __post_init__(self) -> None:
        cfg = self.axis_configuration
        if cfg == "2D":
            if self.z_axis is not None or self.focus is not None:
                raise ValueError(
                    "axis_configuration='2D' forbids z_axis and focus subgroups"
                )
        elif cfg == "3D":
            if self.z_axis is None:
                raise ValueError(
                    "axis_configuration='3D' requires z_axis subgroup"
                )
            if self.focus is not None:
                raise ValueError(
                    "axis_configuration='3D' forbids focus subgroup"
                )
        elif cfg == "3D+Focus":
            if self.z_axis is None or self.focus is None:
                raise ValueError(
                    "axis_configuration='3D+Focus' requires both z_axis and focus subgroups"
                )


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
    optional_components: OptionalComponents                       # always present; contents vary
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
    # TEST FIXTURE for the mock v1.1 adapter (docs/migrations/mock_v1_0_to_v1_1.md).
    # Not a real schema field and not part of any planned version — exercises the
    # ADDITION category only. Never wired into _config_to_dict() or the JSON schema.
    facility_id: Optional[str] = None
    config_author: Optional[str] = None
    extra: dict[str, Any] = field(default_factory=dict)  # preserves any non-typed root HDF5 attrs


@dataclass
class OpcuaClientConfig:
    server_url: str
    auth_mode: str
    security_mode: str
    security_policy: str
    bfs_max_depth: int
    publish_interval: int
    sampling_interval: int
    session_timeout: int
    keep_alive_count: Optional[int] = None
    lifetime_count: Optional[int] = None
    machine_profile: Optional[str] = None
    queue_policy: Optional[str] = None
    queue_size_data_change: Optional[int] = None
    queue_size_events: Optional[int] = None
    reconnect_interval: Optional[int] = None
    root_node: Optional[str] = None
    sync_loop_interval_initial: Optional[int] = None
    sync_loop_interval_settled: Optional[int] = None
    extra: dict[str, Any] = field(default_factory=dict)  # preserves any non-typed HDF5 attrs


@dataclass
class OpcuaPipeConfig:
    pipe_enabled: bool                   # HDF5 int 0/1
    buffer_size: int
    configure_client: Optional[bool] = None      # HDF5 int 0/1
    inbound_rate_limit: Optional[int] = None
    max_inbound_message_size: Optional[int] = None
    min_integrity_level: Optional[str] = None
    pipe_name: Optional[str] = None
    user_access_level: Optional[str] = None
    extra: dict[str, Any] = field(default_factory=dict)  # preserves any non-typed HDF5 attrs


@dataclass
class OpcuaTrigger:
    id: Optional[str] = None
    signal: Optional[str] = None
    subsystem: Optional[str] = None
    rule_enabled: Optional[bool] = None  # HDF5 int 0/1
    start_value: Optional[str] = None
    stop_value: Optional[str] = None
    case_sensitivity: Optional[str] = None
    component: Optional[str] = None
    cooldown_period: Optional[int] = None
    event: Optional[str] = None
    max_fires_per_job: Optional[int] = None
    trigger_label: Optional[str] = None
    extra: dict[str, Any] = field(default_factory=dict)


@dataclass
class OpcuaConfig:
    client: OpcuaClientConfig
    pipe: OpcuaPipeConfig
    triggers: dict[str, OpcuaTrigger]
    triggers_enabled: Optional[bool] = None  # HDF5 float 0.0/1.0 on OPCUA/Triggers group
    trigger_stop_ceiling_layers: Optional[int] = None  # HDF5 int on OPCUA/Triggers group


@dataclass
class MachineConfig:
    meta: MachineConfigMeta
    machine: Machine
    optical_trains: list[OpticalTrain]
    opcua: Optional[OpcuaConfig] = None
