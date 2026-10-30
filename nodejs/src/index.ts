export { MachineConfigReader } from './reader.js';
export type { ReadOptions, ToJsonOptions } from './reader.js';

export { MachineConfigWriter } from './writer.js';

export { MockConfigBuilder } from './builder.js';
export type { MockConfigBuilderOptions } from './builder.js';

export { SCHEMA_VERSION, getSchema, validate } from './schema.js';

export { computeConfigurationHash, canonicalizeForHash } from './hash.js';

export {
  EXTENSION_CONTRACT_VERSION,
  EXTENSION_DTYPES,
  ExtensionContractError,
  assertValidExtensionTree,
  emptyExtensionTree,
} from './extensions/contract.js';
export type {
  ExtensionAttr,
  ExtensionDataset,
  ExtensionDtype,
  ExtensionNumericArray,
  ExtensionScalar,
  ExtensionTable,
  ExtensionTableField,
  ExtensionTree,
} from './extensions/contract.js';
export { canonicalDigest } from './extensions/digest.js';
export { extensionTreeToJson, extensionTreeFromJson } from './extensions/json.js';
export type { ExtensionTableJson, ExtensionTreeJson } from './extensions/json.js';

export {
  openMachineConfig,
  createMachineConfig,
  supportedFileVersions,
  MachineConfigFileV1_0,
  MachineConfigFileV1_1,
  SetMode,
  ok,
  err,
  isOk,
  capabilityError,
  SessionClosedError,
  applySetMode,
  snapshot,
  UnsupportedFileVersion,
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
  MetaModel,
  MachineModel,
  ScannerModel,
  OpticalTrainModel,
} from './capabilities/index.js';

export type * from './models.js';
