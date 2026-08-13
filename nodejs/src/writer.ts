/**
 * Public HDF5 writer: dispatch to a File_Version adapter.
 *
 * On-disk group paths and HDF5 attribute names live in the matching adapter.
 */
import type { MachineConfig } from "./models.js";
import { UnsupportedFileVersion } from "./capabilities/file_version.js";
import { Hdf5WriterV1_0 } from "./capabilities/v1_0/writer.js";

type WriterBackend = { write(path: string): Promise<void> };

const WRITERS: Record<string, new (config: MachineConfig) => WriterBackend> = {
  "1.0": Hdf5WriterV1_0,
};

export class MachineConfigWriter {
  private readonly backend: WriterBackend;

  constructor(config: MachineConfig) {
    const fv = (config.meta.file_version || "1.0").trim() || "1.0";
    const Ctor = WRITERS[fv];
    if (!Ctor) {
      throw new UnsupportedFileVersion(fv);
    }
    this.backend = new Ctor(config);
  }

  write(outputPath: string): Promise<void> {
    return this.backend.write(outputPath);
  }
}
