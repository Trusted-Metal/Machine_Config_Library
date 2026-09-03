/** File_Version 1.0 on-disk HDF5 layout (Node). */
export const FILE_VERSION = '1.0';

export const ROOT_MACHINE = 'Machine';
export const ROOT_OPTICAL_TRAINS = 'Machine/Optical_Trains';
export const ROOT_OPCUA = 'OPCUA';
export const OPCUA_CLIENT = 'OPCUA/Client';
export const OPCUA_PIPE = 'OPCUA/Pipe';
export const OPCUA_TRIGGERS = 'OPCUA/Triggers';

export const TRAIN_ID_PREFIX = 'Optical_Train_';
export const GROUP_SCANNER = 'Scanner';
export const GROUP_LIGHT_SOURCE = 'Light_Source';
export const GROUP_COLLIMATOR = 'Collimator';
export const GROUP_SCANNER_CARD = 'Scanner_Card';
export const GROUP_OPTIONAL_COMPONENTS = 'Optional_Components';
export const GROUP_CLEARBOX = 'ClearBox';
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

export function clearboxPath(index: number): string {
  return clearboxPathById(trainId(index));
}

export function clearboxPathById(tid: string): string {
  return `${trainPathById(tid)}/${GROUP_OPTIONAL_COMPONENTS}/${GROUP_CLEARBOX}`;
}

export function correctionDataPath(index: number): string {
  return `${clearboxPath(index)}/${DS_CORRECTION_DATA}`;
}

export function inverseCorrectionDataPath(index: number): string {
  return `${clearboxPath(index)}/${DS_INVERSE_CORRECTION_DATA}`;
}

export function scanFieldCorrectionFilePath(index: number): string {
  return `${trainPath(index)}/${DS_SCAN_FIELD_CORRECTION_FILE}`;
}
