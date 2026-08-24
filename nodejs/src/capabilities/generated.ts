// AUTO-GENERATED from schema/capabilities — DO NOT EDIT
// python tools/generate_capabilities.py


import type { Result } from "./result.js";
import type { CapabilityError } from "./errors.js";
import type {
  MachineConfigMeta,
  Machine,
  OpticalTrain,
  Scanner,
  LightSource,
  Collimator,
  ScannerCard,
  ClearBox,
  OpcuaConfig,
  CorrectionData,
} from "../models.js";

/** Write mode for set* model APIs. Default is Merge. */
export enum SetMode {
  Merge = 'Merge',
  Replace = 'Replace',
}

export type MetaModel = MachineConfigMeta & { extra?: Record<string, unknown> };
export type MachineModel = Machine & { extra?: Record<string, unknown> };
export type ScannerModel = Scanner & { extra?: Record<string, unknown> };
export type LightSourceModel = LightSource & { extra?: Record<string, unknown> };
export type CollimatorModel = Collimator & { extra?: Record<string, unknown> };
export type ScannerCardModel = ScannerCard & { extra?: Record<string, unknown> };
export type ClearBoxModel = ClearBox & { extra?: Record<string, unknown> };
export type OpticalTrainModel = OpticalTrain & { extra?: Record<string, unknown> };
export type OpcuaModel = OpcuaConfig & { extra?: Record<string, unknown> };

export interface MetaHandle {
  getModel(): MetaModel;
  setModel(model: MetaModel, mode?: SetMode): Result<void, CapabilityError>;
}
export interface MachineHandle {
  getModel(): MachineModel;
  setModel(model: MachineModel, mode?: SetMode): Result<void, CapabilityError>;
}
export interface ClearBoxHandle {
  getModel(): ClearBoxModel;
  setModel(model: ClearBoxModel, mode?: SetMode): Result<void, CapabilityError>;
}
export interface OptionalComponentsHandle {
  clearbox(): Result<ClearBoxHandle, CapabilityError>;
}
export interface TrainHandle {
  getModel(): OpticalTrainModel;
  setModel(model: OpticalTrainModel, mode?: SetMode): Result<void, CapabilityError>;
  getScanner(): ScannerModel;
  setScanner(model: ScannerModel, mode?: SetMode): Result<void, CapabilityError>;
  getLightSource(): LightSourceModel;
  setLightSource(model: LightSourceModel, mode?: SetMode): Result<void, CapabilityError>;
  getCollimator(): CollimatorModel;
  setCollimator(model: CollimatorModel, mode?: SetMode): Result<void, CapabilityError>;
  getScannerCard(): ScannerCardModel;
  setScannerCard(model: ScannerCardModel, mode?: SetMode): Result<void, CapabilityError>;
  optionalComponents(): OptionalComponentsHandle | null;
}
export interface TrainCollection {
  readonly length: number;
  get(index: number): Result<TrainHandle, CapabilityError>;
  [Symbol.iterator](): Iterator<TrainHandle>;
}
export interface OpcuaHandle {
  getModel(): OpcuaModel;
  setModel(model: OpcuaModel, mode?: SetMode): Result<void, CapabilityError>;
}
export interface MachineConfigFile {
  fileVersion(): string;
  meta(): MetaHandle;
  machine(): MachineHandle;
  opticalTrains(): TrainCollection;
  opticalTrain(index: number): Result<TrainHandle, CapabilityError>;
  opcua(): Result<OpcuaHandle, CapabilityError>;
  getCorrectionData(trainIndex: number): Result<CorrectionData, CapabilityError>;
  getInverseCorrectionData(trainIndex: number): Result<CorrectionData, CapabilityError>;
  save(path?: string): Promise<Result<void, CapabilityError>>;
  close(): void;
}

