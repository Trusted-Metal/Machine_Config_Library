/**
 * Public HDF5 writer: dispatch to a File_Version adapter.
 *
 * On-disk group paths and HDF5 attribute names live in the matching adapter.
 */
import type { MachineConfig } from "./models.js";
import { UnsupportedFileVersion } from "./capabilities/file_version.js";
import { Hdf5WriterV1_0 } from "./capabilities/v1_0/writer.js";
import { Hdf5WriterV1_1 } from "./capabilities/v1_1/writer.js";

type WriterBackend = { write(path: string): Promise<void> };

/**
 * Version → writer-adapter dispatch table. Exported (underscore-prefixed,
 * matching Python's `_ADAPTERS` convention) solely so tests can register a
 * mock adapter for the duration of one test — see
 * `docs/migrations/mock_v1_0_to_v1_1.md`. Not for application use; the real
 * consumer entry point is `MachineConfigWriter`.
 */
export const _WRITERS: Record<string, new (config: MachineConfig) => WriterBackend> = {
  "1.0": Hdf5WriterV1_0,
  "1.1": Hdf5WriterV1_1,
};

export class MachineConfigWriter {
  private readonly backend: WriterBackend;

  /**
   * @param targetVersion - Writes as this version regardless of
   *   `config.meta.file_version` — lets a caller upgrade/downgrade without
   *   mutating the model just to express intent (e.g. reading a v1.0 file
   *   and writing it as v1.1 no longer requires setting
   *   `config.meta.file_version = "1.1"` first). Never mutates `config`
   *   itself; only the on-disk `File_Version` changes.
   */
  constructor(config: MachineConfig, targetVersion?: string) {
    const current = (config.meta.file_version || "1.0").trim() || "1.0";
    const fv = (targetVersion || current).trim() || "1.0";
    const Ctor = _WRITERS[fv];
    if (!Ctor) {
      throw new UnsupportedFileVersion(fv);
    }
    // Every adapter stamps config.meta.file_version verbatim as the on-disk
    // File_Version attribute — if targetVersion overrides the adapter
    // choice, the config handed to the adapter must reflect that too, or
    // the file would claim the wrong version on disk. A copy, not a
    // mutation of the caller's config, and only made when actually needed
    // (the common case — no override — never pays for it).
    const resolvedConfig = fv === current ? config : { ...config, meta: { ...config.meta, file_version: fv } };
    this.backend = new Ctor(resolvedConfig);
  }

  write(outputPath: string): Promise<void> {
    return this.backend.write(outputPath);
  }
}
