import type {
  MachineConfig,
  MachineConfigMeta,
  Machine,
  OpticalTrain,
  Scanner,
  AxisConfig,
  LightSource,
  Collimator,
  ScannerCard,
  ScanFieldCorrectionFile,
} from './models.js';
import { SCHEMA_VERSION } from './schema.js';
import { MachineConfigWriter } from './writer.js';
import { forwardPowerCharacterizationPoints } from './powerCharacterization.js';
import { emptyExtensionTree, type ExtensionTree } from './extensions/contract.js';
import { CLEARBOX_EXTENSION_NAME, clearboxFromV1_0Trains } from './clearboxLegacy.js';

export interface MockConfigBuilderOptions {
  /** Number of optical trains (lasers) to generate. Default 2. */
  nLasers?: number;
  /** Build plate X dimension in mm. Default 250. */
  buildPlateX?: number;
  /** Build plate Y dimension in mm. Default 250. */
  buildPlateY?: number;
  /** Build plate Z dimension in mm. Default 20. */
  buildPlateZ?: number;
  /**
   * Include a ClearBox and ScanFieldCorrectionFile on every optical train.
   * Gates both together (matching the Python and Rust builders). Default true.
   */
  includeClearbox?: boolean;
  /** File_Version attribute written to the config. Default "1.0". */
  fileVersion?: string;
  /** Unit string for Working_Distance. Default "mm". */
  workingDistanceUnit?: string;
  machineName?: string;
  manufacturer?: string;
  model?: string;
  serialNumber?: string;
}

type ResolvedOptions = Required<MockConfigBuilderOptions>;

const DEFAULTS: ResolvedOptions = {
  nLasers: 2,
  buildPlateX: 250.0,
  buildPlateY: 250.0,
  buildPlateZ: 20.0,
  includeClearbox: true,
  fileVersion: '1.0',
  workingDistanceUnit: 'mm',
  machineName: 'MockMachine',
  manufacturer: 'MockCo',
  model: 'MockMIDI+',
  serialNumber: 'MOCK-001',
};

// Fixed rather than randomly/time generated, so builds are reproducible across
// runs and platforms — matches the Rust builder's convention (Python's own
// builder uses uuid4()/current time here, which the Node.js port deliberately
// does not follow, for the same reproducibility reason).
const MOCK_MACHINE_ID = '00000000-0000-0000-0000-000000000001';
const MOCK_EXPORT_DATE = '2026-01-01T00:00:00.000Z';

// ---------------------------------------------------------------------------
// Synthetic data generators — mirror python/src/machine_config/builder.py and
// rust/src/builder.rs field-for-field (including the Gaussian correction-grid
// formula) so a config built here is structurally and numerically comparable
// to one built by the other two languages.
// ---------------------------------------------------------------------------

const CORRECTION_N = 257;

/**
 * Smooth 2-D Gaussian warp pattern, flat row-major for shape [257, 257, 2]. Non-zero by design —
 * zeros would hide bugs in downstream consumers. `scale` multiplies the whole grid (used to derive
 * the inverse grid at 0.9× the forward grid, matching the Rust builder).
 */
function gaussianCorrectionGrid(scale: number): Float64Array {
  const n = CORRECTION_N;
  const grid = new Float64Array(n * n * 2);
  for (let i = 0; i < n; i++) {
    const x = -1.0 + (2.0 * i) / (n - 1);
    for (let j = 0; j < n; j++) {
      const y = -1.0 + (2.0 * j) / (n - 1);
      const warp = 2.0 * Math.exp(-(x * x + y * y) / 0.5) * scale;
      grid[(i * n + j) * 2] = warp;
      grid[(i * n + j) * 2 + 1] = warp * 0.8;
    }
  }
  return grid;
}

function mockAxis(bitRes = 20): AxisConfig {
  return {
    actual_bit_resolution: bitRes,
    actual_bit_resolution_unit: 'bits',
    commanded_bit_resolution: bitRes,
    commanded_bit_resolution_unit: 'bits',
    control_type: null,
    range_of_motion: null,
    range_of_motion_unit: 'mm',
    smoothing_kernel: 'GAUSSIAN',
    smoothing_parameters: 60.0,
    tuning_parameters: null,
    tuning_type: null,
  };
}

/** One train's ClearBox group, authored the way a File_Version 1.0 file stores it. */
function mockClearBox(index: number): ExtensionTree {
  const tree = emptyExtensionTree();
  const text = (name: string, value: string): void => {
    tree.attrs[name] = { dtype: 'utf8', value };
  };
  const int = (name: string, value: number): void => {
    tree.attrs[name] = { dtype: 'i64', value: BigInt(value) };
  };
  text('Ip_Address', `192.168.1.${10 + index}`);
  text('Serial_Number', String(index + 1).padStart(3, '0'));
  int('Data_Port', 5001);
  int('Server_Port', 20101);
  int('Actual_Timing_Offset', -8);
  int('Commanded_Timing_Offset', 50);
  text('Output_Path', '/recordings/');
  text('Selected_Camera', 'Default');
  text('Custom_Video_Format', 'MP4');
  text('Video_Output', 'HDMI');
  int('Show_Console', 0);
  int('Software_Trigger_Delay', 3000);
  text('Volts_To_Watts_Algorithm', 'LINEAR');
  text('Volts_To_Watts_Params', '50.0,100.0');
  const shape = [CORRECTION_N, CORRECTION_N, 2];
  tree.datasets['Correction_Data'] = { dtype: 'f64', shape, data: gaussianCorrectionGrid(1.0), attrs: {} };
  tree.datasets['Inverse_Correction_Data'] = { dtype: 'f64', shape, data: gaussianCorrectionGrid(0.9), attrs: {} };
  return tree;
}

function mockSfcf(index: number): ScanFieldCorrectionFile {
  return {
    document_name: `mock_laser_${index + 1}.fc3`,
    document_id: `00000000-0000-0000-0000-${String(index + 1).padStart(12, '0')}`,
    file_size: 1024,
    valid_as_of_date: MOCK_EXPORT_DATE,
    document_created_at: null,
    document_type: 'Scan Field Correction File',
    original_uri: null,
  };
}

function mockTrain(
  index: number,
  includeClearbox: boolean,
  workingDistanceUnit: string,
): OpticalTrain {
  const sign = index % 2 === 0 ? -1 : 1;
  const offsetX = sign * 87.5;
  const offsetY = sign * -23.5;
  const rotation = index % 2 === 0 ? 0.0 : 180.0;

  const scanner: Scanner = {
    manufacturer: 'MockCo',
    model: 'MockScan',
    serial_number: `MOCK-SC-${String(index + 1).padStart(2, '0')}`,
    working_distance: 670.0,
    working_distance_unit: workingDistanceUnit,
    scan_field_x: 600.0,
    scan_field_x_unit: 'mm',
    scan_field_y: 600.0,
    scan_field_y_unit: 'mm',
    scan_field_z: 76.5,
    scan_field_z_unit: 'mm',
    scan_head_offset_x: offsetX,
    scan_head_offset_x_unit: 'mm',
    scan_head_offset_y: offsetY,
    scan_head_offset_y_unit: 'mm',
    scan_head_offset_z: -1.0,
    scan_head_offset_z_unit: 'mm',
    scan_head_rotation: rotation,
    scan_head_rotation_unit: 'degrees',
    axis_configuration: '3D',
    x_axis: mockAxis(),
    y_axis: mockAxis(),
    z_axis: mockAxis(),
    focus: null,
    invert_actual_x: false,
    invert_actual_y: false,
    invert_commanded_x: false,
    invert_commanded_y: false,
  };

  const lightSource: LightSource = {
    manufacturer: 'MockLaser',
    model: 'MockFiber-1070',
    serial_number: `MOCK-LS-${String(index + 1).padStart(2, '0')}`,
    wavelength: 1070.0,
    wavelength_unit: 'nm',
    power_max_nominal: 1000.0,
    power_max_nominal_unit: 'W',
    power_max_actual: 1020.0,
    power_max_actual_unit: 'W',
    power_min_actual: 100.0,
    power_min_actual_unit: 'W',
    power_min_nominal: null,
    power_min_nominal_unit: 'W',
    power_bit_resolution: null,
    power_bit_resolution_unit: 'bits',
    power_characterization: forwardPowerCharacterizationPoints('LINEAR', '[1,100,10,1000]') ?? undefined,
  };

  const collimator: Collimator = {
    manufacturer: 'MockOptics',
    model: 'D50_F120',
    serial_number: `MOCK-COL-${String(index + 1).padStart(2, '0')}`,
    focal_length: 120.0,
    focal_length_unit: 'mm',
  };

  const scannerCard: ScannerCard = {
    manufacturer: 'Raylase',
    model: 'SP-ICE-3',
    serial_number: `MOCK-SC-CARD-${String(index + 1).padStart(2, '0')}`,
    communication_protocol: 'SL2-100',
    sample_period: 10.0,
    sample_period_unit: 'μs',
  };

  return {
    train_id: `Optical_Train_${String(index + 1).padStart(2, '0')}`,
    id: null,
    beam_profile_type: null,
    beam_waist_definition: 'knife-edge',
    beam_waist_major: 67.0,
    beam_waist_major_unit: 'μm',
    beam_waist_minor: 68.0,
    beam_waist_minor_unit: 'μm',
    beam_waist_offset_z: 0.5,
    beam_waist_offset_z_unit: 'mm',
    build_plane_offset_major: 0.2,
    build_plane_offset_major_unit: 'mm',
    build_plane_offset_minor: 0.7,
    build_plane_offset_minor_unit: 'mm',
    collimator_focal_length: 120.0,
    collimator_focal_length_unit: 'mm',
    m2_major: 1.05,
    m2_minor: 1.08,
    major_axis_angle: 0.0,
    major_axis_angle_unit: 'degrees',
    rayleigh_length_major: 3.1,
    rayleigh_length_major_unit: 'mm',
    rayleigh_length_minor: 3.2,
    rayleigh_length_minor_unit: 'mm',
    scanner_number: null,
    thermal_lensing_passed: false,
    thermal_lensing_focal_plane_shift: 1.0,
    thermal_lensing_focal_plane_shift_unit: 'mm',
    thermal_lensing_threshold: 0.75,
    thermal_lensing_threshold_unit: 'mm',
    scanner,
    light_source: lightSource,
    collimator,
    scanner_card: scannerCard,
    scan_field_correction_file: includeClearbox ? mockSfcf(index) : null,
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Builds a synthetic MachineConfig for testing and round-trip checks.
 *
 * Mirrors python/src/machine_config/builder.py's MockConfigBuilder and
 * rust/src/builder.rs's MockConfigBuilder — same field values, same Gaussian
 * correction-grid formula (peak 2.0 at centre; inverse grid is the forward
 * grid × 0.9) — so a config built here is structurally and numerically
 * comparable to one built by the other two languages.
 */
export class MockConfigBuilder {
  private readonly options: ResolvedOptions;

  constructor(options: MockConfigBuilderOptions = {}) {
    this.options = { ...DEFAULTS, ...options };
  }

  /**
   * Build and return the synthetic config without writing to disk.
   */
  build(): MachineConfig {
    const o = this.options;

    const machine: Machine = {
      id: MOCK_MACHINE_ID,
      machine_name: o.machineName,
      manufacturer: o.manufacturer,
      model: o.model,
      serial_number: o.serialNumber,
      build_plate_x: o.buildPlateX,
      build_plate_x_unit: 'mm',
      build_plate_y: o.buildPlateY,
      build_plate_y_unit: 'mm',
      build_plate_z: o.buildPlateZ,
      build_plate_z_unit: 'mm',
      build_plate_radius: null,
      build_plate_radius_unit: null,
      gas_flow_direction: 'Y+',
      recoat_direction: 'X+',
      recoater_blade_type: 'Standard',
    };

    const meta: MachineConfigMeta = {
      schema_version: SCHEMA_VERSION,
      machine_name: o.machineName,
      manufacturer: o.manufacturer,
      model: o.model,
      serial_number: o.serialNumber,
      file_version: o.fileVersion,
      export_date: MOCK_EXPORT_DATE,
      configuration_hash: '0'.repeat(64),
      extra: {},
    };

    const optical_trains: OpticalTrain[] = [];
    for (let i = 0; i < o.nLasers; i++) {
      optical_trains.push(mockTrain(i, o.includeClearbox, o.workingDistanceUnit));
    }

    const config: MachineConfig = { meta, machine, optical_trains };
    if (o.includeClearbox) {
      // Each train's ClearBox is authored as a File_Version 1.0 file stores it, then shimmed.
      const groups: Array<[string, ExtensionTree]> = optical_trains.map((t, i) => [t.train_id, mockClearBox(i)]);
      const clearbox = clearboxFromV1_0Trains(groups);
      if (clearbox !== null) config.extensions = { [CLEARBOX_EXTENSION_NAME]: clearbox };
    }
    return config;
  }

  /**
   * Build the config and write it to an HDF5 file at outputPath.
   */
  async save(outputPath: string): Promise<void> {
    await new MachineConfigWriter(this.build()).write(outputPath);
  }
}
