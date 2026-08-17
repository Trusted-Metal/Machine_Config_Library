import tempfile
from pathlib import Path
from machine_config import MachineConfigReader, MachineConfigWriter

"""
AV-08: Roundtrip version string fidelity
ID:          AV-08
Title:       File_Version string survives write→read unchanged
Category:    adapter-versioning
Layer:       writer + reader
Precondition: fixtures/reference_config.h5
Action:      Read → write to temp → read temp → compare File_Version strings
Expected:    File_Version is exactly "1.0" after roundtrip in all five languages.
             No whitespace added, no truncation, no case change.
Rationale:   A writer that silently normalizes the version string would break
             future version dispatch for files it produces.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()
    orig_version = cfg.meta.file_version.strip()

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()
    rb_version = rb.meta.file_version.strip()

    if rb_version != "1.0":
        return False, f"file_version after roundtrip: expected '1.0', got '{rb_version}'"
    if rb_version != orig_version:
        return False, f"file_version changed: '{orig_version}' → '{rb_version}'"

    return True, f"File_Version survives roundtrip unchanged: '{rb_version}'"
