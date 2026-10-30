from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Optional

from machine_config.extension_contract import ExtensionTree


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
class PowerCharacterization:
    """A structured algorithm + equation + constants + characterization points describing a power
    conversion. ``LightSource.power_characterization`` uses it; ClearBox's own
    Power_Characterization is an opaque extension tree and is not modelled here. See
    ``docs/migrations/v1_0_to_v1_1.md`` Changes 3/4 for the derivation rules.
    """
    algorithm_type: Optional[str]
    algorithm_equation: Optional[str]
    input_type: Optional[str]
    units_derived_quantity: Optional[str]
    derivation_equation_constants: list[EquationConstant]
    characterization_points: list[CalibrationPoint]


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
    # Plain bool, not Optional[bool] — deliberately different from every
    # other bool field in this schema. Defaults to False whether the on-disk
    # attribute is absent or explicitly 0; the API never distinguishes those
    # two cases (user-confirmed, 2026-08-21). Read faithfully, but never
    # appears in any output (MCF or JSON) unless True — a write→read
    # round-trip is deliberately lossy for an explicit False, which becomes
    # indistinguishable from "never set".
    invert_actual_x: bool = False
    invert_actual_y: bool = False
    invert_commanded_x: bool = False
    invert_commanded_y: bool = False

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
    # The only representation of this concept, for files of either version
    # (v1.0's flat Watts_To_Volts_Algorithm/Params is a lossless encoding of it).
    power_characterization: Optional[PowerCharacterization] = None


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
    recoater_blade_type: Optional[str] = None


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
    # Reader-computed, in-memory only: True/False once a MachineConfigReader has
    # recomputed the configuration hash and compared it to the on-disk value;
    # None until then. Never set by a writer or by MockConfigBuilder/
    # YamlConfigBuilder. Deliberately excluded from _config_to_dict()/to_json()
    # and from the JSON schema — never written to HDF5, never part of the
    # interop contract. A mismatch is not an error: the file remains fully
    # readable either way (see machine_config.hash).
    is_valid: Optional[bool] = None
    extra: dict[str, Any] = field(default_factory=dict)  # preserves any non-typed root HDF5 attrs


@dataclass
class MachineConfig:
    meta: MachineConfigMeta
    machine: Machine
    optical_trains: list[OpticalTrain]
    # Opaque extension trees keyed by group name under ``Extensions/``; read and written verbatim
    # by core (see machine_config.extension_contract). ClearBox is modelled in core, never listed here.
    extensions: dict[str, ExtensionTree] = field(default_factory=dict)
