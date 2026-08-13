export { MachineConfigReader } from './reader.js';
export type { ReadOptions, ToJsonOptions, CorrectionData } from './reader.js';

export { MachineConfigWriter } from './writer.js';

export { MockConfigBuilder } from './builder.js';
export type { MockConfigBuilderOptions } from './builder.js';

export { SCHEMA_VERSION, getSchema, validate } from './schema.js';

export * from './adapters/index.js';

export {
  openMachineConfig,
  createMachineConfig,
  supportedFileVersions,
  MachineConfigFileV1_0,
  SetMode,
  ok,
  err,
  isOk,
  capabilityError,
  SessionClosedError,
  applySetMode,
  snapshot,
} from './capabilities/index.js';
export type {
  MachineConfigFile,
  Result,
  CapabilityError,
  CapabilityErrorCode,
  MetaHandle,
  MachineHandle,
  TrainHandle,
  TrainCollection,
  OpcuaHandle,
  ClearBoxHandle,
  OptionalComponentsHandle,
  MetaModel,
  MachineModel,
  ScannerModel,
  OpticalTrainModel,
} from './capabilities/index.js';

export type * from './models.js';
