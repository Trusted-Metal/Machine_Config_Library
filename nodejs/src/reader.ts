/**
 * Public HDF5 reader: peek File_Version, then dispatch to a version adapter.
 *
 * On-disk group paths and HDF5 attribute names live in the matching adapter.
 * This module does not assume a v1.0 layout.
 */
import { peekFileVersion, UnsupportedFileVersion } from "./capabilities/file_version.js";
import {
  Hdf5AdapterV1_0,
  type CorrectionData,
  type ReadOptions,
  type ToJsonOptions,
} from "./capabilities/v1_0/hdf5.js";
import { Hdf5AdapterV1_1 } from "./capabilities/v1_1/hdf5.js";

export type { CorrectionData, ReadOptions, ToJsonOptions };

type ReaderBackend = {
  parse(options?: ReadOptions): Promise<import("./models.js").MachineConfig>;
  toJson(options?: ToJsonOptions): Promise<string>;
  getCorrectionData(trainIndex: number): Promise<CorrectionData>;
  getInverseCorrectionData(trainIndex: number): Promise<CorrectionData>;
  getRawGroup(hdfPath: string): Promise<Record<string, unknown>>;
};

/**
 * Version → reader-adapter dispatch table. Exported (underscore-prefixed,
 * matching Python's `_ADAPTERS` convention) solely so tests can register a
 * mock adapter for the duration of one test — see
 * `docs/migrations/mock_v1_0_to_v1_1.md`. Not for application use; the real
 * consumer entry point is `MachineConfigReader`.
 */
export const _READERS: Record<string, new (path: string) => ReaderBackend> = {
  "1.0": Hdf5AdapterV1_0,
  "1.1": Hdf5AdapterV1_1,
};

export class MachineConfigReader {
  private readonly filePath: string;
  private backend: ReaderBackend | null = null;

  constructor(filePath: string) {
    this.filePath = filePath;
  }

  private async adapter(): Promise<ReaderBackend> {
    if (this.backend) return this.backend;
    const version = await peekFileVersion(this.filePath);
    const Ctor = _READERS[version];
    if (!Ctor) {
      throw new UnsupportedFileVersion(version);
    }
    this.backend = new Ctor(this.filePath);
    return this.backend;
  }

  async parse(options: ReadOptions = {}) {
    return (await this.adapter()).parse(options);
  }

  async toJson(options: ToJsonOptions = {}) {
    return (await this.adapter()).toJson(options);
  }

  async getCorrectionData(trainIndex: number) {
    return (await this.adapter()).getCorrectionData(trainIndex);
  }

  async getInverseCorrectionData(trainIndex: number) {
    return (await this.adapter()).getInverseCorrectionData(trainIndex);
  }

  async getRawGroup(hdfPath: string) {
    return (await this.adapter()).getRawGroup(hdfPath);
  }
}
