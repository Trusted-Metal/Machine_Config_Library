/**
 * File_Version 1.1 on-disk HDF5 layout (Node).
 *
 * Deliberately independent of the previous version's layout module: nothing
 * here imports from or refers to it, so that module can change or be removed
 * later without affecting this one.
 */
export const FILE_VERSION = '1.1';

export const ROOT_MACHINE = 'Machine';
export const ROOT_OPTICAL_TRAINS = 'Machine/Optical_Trains';
export const ROOT_EXTENSIONS = 'Extensions';
export const GROUP_CLEARBOX = 'Extensions/ClearBox';
export const ROOT_OPCUA = 'Extensions/TM_OPCUA';
export const OPCUA_CLIENT = 'Extensions/TM_OPCUA/Client';
export const OPCUA_PIPE = 'Extensions/TM_OPCUA/Pipe';
export const OPCUA_TRIGGERS = 'Extensions/TM_OPCUA/Triggers';

export const TRAIN_ID_PREFIX = 'Optical_Train_';
export const GROUP_SCANNER = 'Scanner';
export const GROUP_LIGHT_SOURCE = 'Light_Source';
export const GROUP_COLLIMATOR = 'Collimator';
export const GROUP_SCANNER_CARD = 'Scanner_Card';
export const GROUP_POWER_CHARACTERIZATION = 'Power_Characterization';
export const DS_DERIVATION_EQUATION_CONSTANTS = 'Derivation_Equation_Constants';
export const DS_CHARACTERIZATION_POINTS = 'Characterization_Points';
export const DS_CORRECTION_DATA = 'Correction_Data';
export const DS_INVERSE_CORRECTION_DATA = 'Inverse_Correction_Data';
export const DS_SCAN_FIELD_CORRECTION_FILE = 'scan_field_correction_file';

export function trainId(index: number): string {
  return `${TRAIN_ID_PREFIX}${String(index + 1).padStart(2, '0')}`;
}

export function trainIds(keys: string[]): string[] {
  return keys.filter((k) => k.startsWith(TRAIN_ID_PREFIX)).sort();
}

export function trainPathById(tid: string): string {
  return `${ROOT_OPTICAL_TRAINS}/${tid}`;
}

export function trainPath(index: number): string {
  return trainPathById(trainId(index));
}

export function scannerPathById(tid: string): string {
  return `${trainPathById(tid)}/${GROUP_SCANNER}`;
}

export function lightSourcePathById(tid: string): string {
  return `${trainPathById(tid)}/${GROUP_LIGHT_SOURCE}`;
}

export function collimatorPathById(tid: string): string {
  return `${trainPathById(tid)}/${GROUP_COLLIMATOR}`;
}

export function scannerCardPathById(tid: string): string {
  return `${trainPathById(tid)}/${GROUP_SCANNER_CARD}`;
}

/** Per-train ClearBox subgroup — `Extensions/ClearBox/<train_id>/`. */
export function clearboxPathById(tid: string): string {
  return `${GROUP_CLEARBOX}/${tid}`;
}

export function clearboxPath(index: number): string {
  return clearboxPathById(trainId(index));
}

export function correctionDataPathById(tid: string): string {
  return `${clearboxPathById(tid)}/${DS_CORRECTION_DATA}`;
}

export function inverseCorrectionDataPathById(tid: string): string {
  return `${clearboxPathById(tid)}/${DS_INVERSE_CORRECTION_DATA}`;
}

export function scanFieldCorrectionFilePath(index: number): string {
  return `${trainPath(index)}/${DS_SCAN_FIELD_CORRECTION_FILE}`;
}
