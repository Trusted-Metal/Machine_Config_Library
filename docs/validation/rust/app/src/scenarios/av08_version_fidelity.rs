// AV-08: File_Version string survives write→read unchanged
use machine_config::{MachineConfigReader, MachineConfigWriter};
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");
    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };
    let orig_version = cfg.meta.file_version.trim().to_string();

    let tmp = match tempfile::Builder::new().suffix(".h5").tempfile() {
        Ok(t) => t,
        Err(e) => return (false, format!("tempfile creation failed: {e}")),
    };
    if let Err(e) = MachineConfigWriter::new(&cfg).write(tmp.path()) {
        return (false, format!("write failed: {e}"));
    }
    let rb = match MachineConfigReader::open(tmp.path()).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback failed: {e}")),
    };
    let rb_version = rb.meta.file_version.trim().to_string();

    if rb_version != "1.0" {
        return (false, format!("file_version after roundtrip: expected '1.0', got '{rb_version}'"));
    }
    if rb_version != orig_version {
        return (false, format!("file_version changed: '{orig_version}' -> '{rb_version}'"));
    }

    (true, format!("File_Version survives roundtrip unchanged: '{rb_version}'"))
}
