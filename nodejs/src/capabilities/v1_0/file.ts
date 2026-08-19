/**
 * File_Version 1.0 stable model facade — navigation + get/set with SetMode.
 */
import { Hdf5AdapterV1_0 } from './hdf5.js';
import { Hdf5WriterV1_0 } from './writer.js';
import { MockConfigBuilder } from '../../builder.js';
import type { MachineConfig } from '../../models.js';
import { err, ok, type Result } from '../result.js';
import { capabilityError, SessionClosedError, type CapabilityError } from '../errors.js';
import { applySetMode, snapshot } from '../merge.js';
import {
  SetMode,
  type ClearBoxHandle,
  type ClearBoxModel,
  type CollimatorModel,
  type LightSourceModel,
  type MachineConfigFile,
  type MachineHandle,
  type MachineModel,
  type MetaHandle,
  type MetaModel,
  type OpcuaHandle,
  type OpcuaModel,
  type OpticalTrainModel,
  type OptionalComponentsHandle,
  type ScannerCardModel,
  type ScannerModel,
  type TrainCollection,
  type TrainHandle,
} from '../generated.js';

type Json = Record<string, unknown>;

function asJson(config: MachineConfig): Json {
  return config as unknown as Json;
}

export class MachineConfigFileV1_0 implements MachineConfigFile {
  private closed = false;
  private path: string | null;
  private data: Json;
  private readonly version: string;

  private constructor(data: Json, path: string | null, version: string) {
    this.data = data;
    this.path = path;
    this.version = version;
  }

  static async open(path: string): Promise<Result<MachineConfigFileV1_0, CapabilityError>> {
    try {
      const reader = new Hdf5AdapterV1_0(path);
      const config = await reader.parse({ includeBinary: true });
      const fv = (config.meta.file_version || '1.0').trim() || '1.0';
      if (fv !== '1.0') {
        return err(
          capabilityError(
            'UnsupportedVersion',
            `No adapter for File_Version "${fv}" (v1.0 facade)`,
          ),
        );
      }
      return ok(new MachineConfigFileV1_0(asJson(config), path, fv));
    } catch (e) {
      return err(capabilityError('IoError', e instanceof Error ? e.message : String(e)));
    }
  }

  static create(version: string): Result<MachineConfigFileV1_0, CapabilityError> {
    const fv = version.trim() || '1.0';
    if (fv !== '1.0') {
      return err(
        capabilityError('UnsupportedVersion', `create() unsupported for File_Version "${fv}"`),
      );
    }
    const config = new MockConfigBuilder({ nLasers: 1 }).build();
    config.meta.file_version = '1.0';
    return ok(new MachineConfigFileV1_0(asJson(config), null, '1.0'));
  }

  private assertOpen(): void {
    if (this.closed) throw new SessionClosedError();
  }

  fileVersion(): string {
    return this.version;
  }

  meta(): MetaHandle {
    this.assertOpen();
    return {
      getModel: (): MetaModel => snapshot(this.data['meta']) as MetaModel,
      setModel: (model, mode = SetMode.Merge): Result<void, CapabilityError> => {
        this.assertOpen();
        this.data['meta'] = applySetMode(
          this.data['meta'] as Json,
          model as unknown as Json,
          mode,
        );
        return ok(undefined);
      },
    };
  }

  machine(): MachineHandle {
    this.assertOpen();
    return {
      getModel: (): MachineModel => snapshot(this.data['machine']) as MachineModel,
      setModel: (model, mode = SetMode.Merge): Result<void, CapabilityError> => {
        this.assertOpen();
        this.data['machine'] = applySetMode(
          this.data['machine'] as Json,
          model as unknown as Json,
          mode,
        );
        return ok(undefined);
      },
    };
  }

  private trainHandle(index: number): TrainHandle {
    const getTrain = (): Json =>
      (this.data['optical_trains'] as Json[])[index];

    const setTrainProp = (
      prop: 'scanner' | 'light_source' | 'collimator' | 'scanner_card',
      model: unknown,
      mode: SetMode,
    ): Result<void, CapabilityError> => {
      this.assertOpen();
      const train = getTrain();
      const updated = applySetMode(train[prop] as Json, model as Json, mode);
      switch (prop) {
        case 'scanner': train['scanner'] = updated; break;
        case 'light_source': train['light_source'] = updated; break;
        case 'collimator': train['collimator'] = updated; break;
        case 'scanner_card': train['scanner_card'] = updated; break;
      }
      return ok(undefined);
    };

    return {
      getModel: (): OpticalTrainModel =>
        snapshot(getTrain()) as unknown as OpticalTrainModel,
      setModel: (model, mode = SetMode.Merge): Result<void, CapabilityError> => {
        this.assertOpen();
        const trains = this.data['optical_trains'] as Json[];
        trains[index] = applySetMode(trains[index], model as unknown as Json, mode);
        return ok(undefined);
      },
      getScanner: (): ScannerModel => snapshot(getTrain()['scanner']) as ScannerModel,
      setScanner: (model, mode = SetMode.Merge) => setTrainProp('scanner', model, mode),
      getLightSource: (): LightSourceModel =>
        snapshot(getTrain()['light_source']) as LightSourceModel,
      setLightSource: (model, mode = SetMode.Merge) =>
        setTrainProp('light_source', model, mode),
      getCollimator: (): CollimatorModel =>
        snapshot(getTrain()['collimator']) as CollimatorModel,
      setCollimator: (model, mode = SetMode.Merge) =>
        setTrainProp('collimator', model, mode),
      getScannerCard: (): ScannerCardModel =>
        snapshot(getTrain()['scanner_card']) as ScannerCardModel,
      setScannerCard: (model, mode = SetMode.Merge) =>
        setTrainProp('scanner_card', model, mode),
      optionalComponents: (): OptionalComponentsHandle | null => {
        this.assertOpen();
        const oc = getTrain()['optional_components'] as Json | undefined;
        const cb = oc?.['clearbox'];
        if (cb == null) return null;
        return {
          clearbox: (): Result<ClearBoxHandle, CapabilityError> => {
            this.assertOpen();
            const again = getTrain()['optional_components'] as Json | undefined;
            if (again?.['clearbox'] == null) {
              return err(
                capabilityError('NotPresent', 'clearbox is not present'),
              );
            }
            return ok({
              getModel: (): ClearBoxModel =>
                snapshot(again['clearbox']) as ClearBoxModel,
              setModel: (model, mode = SetMode.Merge): Result<void, CapabilityError> => {
                this.assertOpen();
                const o = getTrain()['optional_components'] as Json;
                o['clearbox'] = applySetMode(
                  o['clearbox'] as Json,
                  model as unknown as Json,
                  mode,
                );
                return ok(undefined);
              },
            });
          },
        };
      },
    };
  }

  opticalTrain(index: number): Result<TrainHandle, CapabilityError> {
    this.assertOpen();
    const trains = this.data['optical_trains'] as unknown[];
    if (index < 0 || index >= trains.length) {
      return err(
        capabilityError(
          'InvalidIndex',
          `optical train index ${index} out of range [0, ${trains.length})`,
        ),
      );
    }
    return ok(this.trainHandle(index));
  }

  opticalTrains(): TrainCollection {
    this.assertOpen();
    const self = this;
    return {
      get length() {
        self.assertOpen();
        return (self.data['optical_trains'] as unknown[]).length;
      },
      get(index: number) {
        return self.opticalTrain(index);
      },
      *[Symbol.iterator](): Iterator<TrainHandle> {
        self.assertOpen();
        const n = (self.data['optical_trains'] as unknown[]).length;
        for (let i = 0; i < n; i++) {
          yield self.trainHandle(i);
        }
      },
    };
  }

  /**
   * Returns the OPCUA node, or `Err(ValidationError)` if OPCUA is present
   * but missing one or more required fields. Collects every missing field
   * at once (in `details`) rather than failing on the first one — see
   * OPCUA_FIELD_PROMOTION_PLAN.md's "Why facade-only enforcement". The
   * low-level reader/writer stay fully permissive; this is the one place
   * "required" is enforced.
   */
  opcua(): Result<OpcuaHandle, CapabilityError> {
    this.assertOpen();
    const opcuaData = this.data['opcua'];
    if (opcuaData == null) {
      return err(capabilityError('NotPresent', 'OPCUA group is not present'));
    }

    const current = opcuaData as unknown as OpcuaModel;
    const missing: string[] = [];
    if (current.client.machine_profile == null) missing.push('Machine_Profile');
    if (current.client.root_node == null) missing.push('Root_Node');
    if (current.pipe.configure_client == null) missing.push('Configure_Client');
    if (current.pipe.pipe_name == null) missing.push('Pipe_Name');
    if (current.triggers_enabled == null) missing.push('Triggers_Enabled');
    if (current.trigger_stop_ceiling_layers == null) missing.push('Trigger_Stop_Ceiling_Layers');
    for (const [name, trigger] of Object.entries(current.triggers)) {
      if (trigger.event == null) missing.push(`${name}.Event`);
    }

    if (missing.length > 0) {
      return err(
        capabilityError(
          'ValidationError',
          `OPCUA is present but missing required field(s): ${missing.join(', ')}`,
          missing,
        ),
      );
    }

    return ok({
      getModel: (): OpcuaModel => snapshot(this.data['opcua']) as OpcuaModel,
      setModel: (model, mode = SetMode.Merge): Result<void, CapabilityError> => {
        this.assertOpen();
        this.data['opcua'] = applySetMode(
          this.data['opcua'] as Json,
          model as unknown as Json,
          mode,
        );
        return ok(undefined);
      },
    });
  }

  async save(path?: string): Promise<Result<void, CapabilityError>> {
    this.assertOpen();
    const out = path ?? this.path;
    if (!out) {
      return err(
        capabilityError('ValidationError', 'save() requires a path for create()-d files'),
      );
    }
    try {
      await new Hdf5WriterV1_0(
        this.data as unknown as MachineConfig,
      ).write(out);
      this.path = out;
      return ok(undefined);
    } catch (e) {
      return err(capabilityError('IoError', e instanceof Error ? e.message : String(e)));
    }
  }

  close(): void {
    this.closed = true;
  }
}
