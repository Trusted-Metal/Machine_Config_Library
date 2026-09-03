//! File_Version 1.0 adapter: on-disk layout, HDF5 parse/write, and stable facade.

pub mod layout;
pub mod writer;
pub mod hdf5;
mod file;

pub use file::MachineConfigFileV1_0;
