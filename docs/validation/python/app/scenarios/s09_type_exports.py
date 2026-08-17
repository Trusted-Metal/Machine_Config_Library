from machine_config import (
    MachineConfig, MachineConfigMeta, Machine, OpticalTrain,
    Scanner, LightSource, Collimator, ScannerCard,
    OptionalComponents, ClearBox, ScanFieldCorrectionFile,
    OpcuaConfig, MockConfigBuilder,
)

"""
S-09: Public types importable from machine_config top-level
ID:          S-09
Title:       Verify all public model types are importable from the library surface
Category:    happy-path
Layer:       public API / packaging
Precondition: Library installed from artifact (wheel / tarball / module), not from source
Action:      Import MachineConfig, Scanner, OpticalTrain, LightSource, Collimator,
             ScannerCard, ClearBox, ScanFieldCorrectionFile, OpcuaConfig, and
             MockConfigBuilder from the library's public import path only.
Expected:    All imports resolve without error.
             No type needs to be defined or re-implemented by the consumer.
Rationale:   If a consumer must import from internal paths the public API surface
             is incomplete. This scenario catches that gap at packaging time.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    _b: MockConfigBuilder = MockConfigBuilder()
    _: MachineConfig
    _: Scanner
    return True, "all public types importable from machine_config top-level"
