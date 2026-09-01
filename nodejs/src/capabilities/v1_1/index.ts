/**
 * File_Version 1.1 adapter — layout, HDF5 parse/write, and stable facade.
 *
 * No migration functions here — Phase 2 clean-up (see the migration
 * implementation plan) removed migrateV1ToV1_1/migrateV1_1ToV1. Upgrade/
 * downgrade is now just parse()/write(): each writer writes its own native
 * fields, falling back to ../../powerCharacterization.js's shape-conversion
 * functions only when its own native field is absent.
 */
export { MachineConfigFileV1_1 } from './file.js';
export { Hdf5AdapterV1_1 } from './hdf5.js';
export { Hdf5WriterV1_1 } from './writer.js';
export * from './layout.js';
