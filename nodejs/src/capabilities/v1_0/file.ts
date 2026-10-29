/**
 * File_Version 1.0 stable model facade — navigation + get/set with SetMode.
 */
import { Hdf5AdapterV1_0 } from './hdf5.js';
import { Hdf5WriterV1_0 } from './writer.js';
import { MockConfigBuilder } from '../../builder.js';
import { computeConfigurationHash } from '../../hash.js';
import type { MachineConfig } from '../../models.js';
import { err, ok, type Result } from '../result.js';
import { capabilityError, SessionClosedError, type CapabilityError } from '../errors.js';
import { applySetMode, snapshot } from '../merge.js';
import {
  SetMode,
  type CollimatorModel,
  type LightSourceModel,
  type MachineConfigFile,
  type MachineHandle,
  type MachineModel,
  type MetaHandle,
  type MetaModel,
  type OpticalTrainModel,
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
   * Recomputes the configuration hash from the session's *current* content
   * and compares it to the session's current `meta.configuration_hash` —
   * fresh, every call. Deliberately live, not cached at open() time: this
   * facade is a mutable session (setMeta/setScanner/etc. all mutate
   * `this.data` in place), so a value computed once at open() would
   * silently go stale the instant any set* call ran. See
   * CAPABILITIES_HASH_INTEGRATION_PLAN.md's live-method design — in
   * particular, this reads `false` for the entire span between an edit and
   * the next save(), which is expected (an unsaved edit hasn't been hashed
   * yet), not a sign of corruption. Never throws: a mismatch is
   * informational, matching CONFIGURATION_HASH_PLAN.md's non-fatal design
   * throughout.
   */
  isValid(): boolean {
    const config = this.data as unknown as MachineConfig;
    return computeConfigurationHash(config) === config.meta.configuration_hash;
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
      const config = this.data as unknown as MachineConfig;
      // Compute-and-stamp before writing. `this.data` and `config` are the
      // exact same object (asJson is a pure type-cast, no copy), so this
      // single assignment also updates the session's own retained state --
      // isValid() reads true immediately after save(), without requiring a
      // fresh open(). See CAPABILITIES_HASH_INTEGRATION_PLAN.md's
      // live-method design.
      config.meta.configuration_hash = computeConfigurationHash(config);
      await new Hdf5WriterV1_0(config).write(out);
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
