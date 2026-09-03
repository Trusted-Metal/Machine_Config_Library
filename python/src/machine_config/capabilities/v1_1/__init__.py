"""File_Version 1.1 adapter: layout, HDF5 parse/write, and stable facade.

No migration functions here — Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed
migrate_v1_to_v1_1/migrate_v1_1_to_v1. Upgrade/downgrade is now just
parse()/write(): each writer writes its own native fields, falling back to
machine_config.power_characterization's shape-conversion functions only when
its own native field is absent.
"""
from .file import MachineConfigFileV1_1
from .hdf5 import Hdf5AdapterV1_1, config_from_dict
from .writer import Hdf5WriterV1_1

__all__ = [
    "MachineConfigFileV1_1",
    "Hdf5AdapterV1_1",
    "Hdf5WriterV1_1",
    "config_from_dict",
]
