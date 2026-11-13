//! Where a machine configuration file is read from.
//!
//! Normally just a path. A [`FileSource`] can also carry a hook that configures the HDF5
//! file-access property list before the file is opened. That lets the MCF live somewhere other
//! than a plain file, e.g. as a STORED member of a Lossless Build Log (`.lbl`) container read in
//! place through the LBL virtual file driver, without extracting it:
//!
//! ```ignore
//! let log = lbl::BuildLog::open("build.lbl")?;
//! let archive = log.archive().unwrap();
//! let member = log.machine_config_member().unwrap();
//! let src = FileSource::with_fapl_hook(archive.path(), move |fapl| {
//!     archive.fapl_set(member, fapl).map_err(|e| e.to_string())
//! });
//! let config = MachineConfigReader::open_source(src)?.parse()?;
//! ```
//!
//! The hook runs on a fresh `H5P_FILE_ACCESS` list while hdf5-metno's global lock is held, and
//! HDF5 then opens `path` read-only with it. The hook must target the same HDF5 library instance
//! this crate links (static HDF5 builds cannot be shared). A redirected source is read-only: it
//! is never used as a default `save()` target.

use std::ffi::CString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hdf5::File as H5File;
use hdf5_sys::h5i::hid_t;

/// Configures a file-access property list (`hid_t`) for opening a [`FileSource`].
pub type FaplHook = Arc<dyn Fn(hid_t) -> std::result::Result<(), String> + Send + Sync>;

#[derive(Clone)]
pub struct FileSource {
    path: PathBuf,
    hook: Option<FaplHook>,
}

impl FileSource {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf(), hook: None }
    }

    pub fn with_fapl_hook(
        path: impl AsRef<Path>,
        hook: impl Fn(hid_t) -> std::result::Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self { path: path.as_ref().to_path_buf(), hook: Some(Arc::new(hook)) }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// True when the bytes are not simply the file at [`Self::path`].
    pub fn is_redirected(&self) -> bool {
        self.hook.is_some()
    }

    /// Default `save()` target for a file opened from this source: none when redirected (the path
    /// may name the container that holds the MCF).
    pub fn save_path(&self) -> Option<PathBuf> {
        (!self.is_redirected()).then(|| self.path.clone())
    }

    /// Opens the file read-only.
    pub fn open(&self) -> hdf5::Result<H5File> {
        let Some(hook) = &self.hook else { return H5File::open(&self.path) };
        let path = self
            .path
            .to_str()
            .ok_or_else(|| hdf5::Error::from("FileSource path is not valid UTF-8"))?;
        let cpath = CString::new(path).map_err(|_| hdf5::Error::from("FileSource path contains NUL"))?;
        hdf5::sync::sync(|| unsafe {
            use hdf5_sys::h5f::{H5Fopen, H5F_ACC_RDONLY};
            let fapl = hdf5::plist::FileAccess::try_new()?;
            hook(fapl.id()).map_err(|e| hdf5::Error::from(format!("FileSource FAPL hook failed: {e}")))?;
            let id = H5Fopen(cpath.as_ptr(), H5F_ACC_RDONLY, fapl.id());
            if id < 0 {
                return Err(hdf5::Error::from(format!("unable to open {path} through FAPL hook")));
            }
            hdf5::from_id::<H5File>(id)
        })
    }
}

impl fmt::Debug for FileSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSource")
            .field("path", &self.path)
            .field("redirected", &self.is_redirected())
            .finish()
    }
}

impl From<&Path> for FileSource {
    fn from(p: &Path) -> Self {
        Self::new(p)
    }
}
impl From<PathBuf> for FileSource {
    fn from(p: PathBuf) -> Self {
        Self { path: p, hook: None }
    }
}
impl From<&PathBuf> for FileSource {
    fn from(p: &PathBuf) -> Self {
        Self::new(p)
    }
}
impl From<&str> for FileSource {
    fn from(p: &str) -> Self {
        Self::new(p)
    }
}
impl From<String> for FileSource {
    fn from(p: String) -> Self {
        Self::new(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_machine_config, open_machine_config_source, MachineConfigReader};

    const REFERENCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");
    const REFERENCE_V1_1: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_v1_1.h5");

    // An HDF5 in-memory file image stands in for the LBL container: HDF5 ignores the path when a
    // core-driver image is set, so the hook genuinely redirects the bytes. Each image needs its own
    // name because HDF5 treats core files with equal names as the same (already open) file.
    fn image_source(bytes: Arc<Vec<u8>>) -> FileSource {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let name = format!("not-on-disk-{}.h5", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        FileSource::with_fapl_hook(name, move |fapl| unsafe {
            use hdf5_sys::h5p::{H5Pset_fapl_core, H5Pset_file_image};
            if H5Pset_fapl_core(fapl, 1 << 20, 0) < 0
                || H5Pset_file_image(fapl, bytes.as_ptr() as *mut _, bytes.len()) < 0
            {
                return Err("core driver setup failed".into());
            }
            Ok(())
        })
    }

    #[test]
    fn plain_paths_convert() {
        let a: FileSource = REFERENCE.into();
        assert!(!a.is_redirected());
        assert_eq!(a.save_path().as_deref(), Some(Path::new(REFERENCE)));
        assert!(MachineConfigReader::open_source(PathBuf::from(REFERENCE)).is_ok());
    }

    fn json_diff(a: &serde_json::Value, b: &serde_json::Value, at: String, out: &mut Vec<String>) {
        use serde_json::Value::{Array, Object};
        match (a, b) {
            (Object(x), Object(y)) => {
                for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                    let null = serde_json::Value::Null;
                    json_diff(x.get(k).unwrap_or(&null), y.get(k).unwrap_or(&null), format!("{at}/{k}"), out);
                }
            }
            (Array(x), Array(y)) if x.len() == y.len() => {
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    json_diff(p, q, format!("{at}/{i}"), out);
                }
            }
            _ if a != b && out.len() < 10 => out.push(format!("{at}: {} vs {}", short(a), short(b))),
            _ => {}
        }
    }

    fn short(v: &serde_json::Value) -> String {
        let s = v.to_string();
        if s.len() > 80 { format!("{}...", &s[..80]) } else { s }
    }

    #[test]
    fn hook_reader_matches_path() {
        for path in [REFERENCE, REFERENCE_V1_1] {
            let bytes = Arc::new(std::fs::read(path).unwrap());
            let via_hook = MachineConfigReader::open_source(image_source(bytes)).unwrap().parse().unwrap();
            let direct = MachineConfigReader::open(path).unwrap().parse().unwrap();
            let mut diff = Vec::new();
            json_diff(
                &serde_json::to_value(&via_hook).unwrap(),
                &serde_json::to_value(&direct).unwrap(),
                String::new(),
                &mut diff,
            );
            assert!(diff.is_empty(), "{path}: {diff:#?}");
        }
    }

    #[test]
    fn hook_capability_open_is_read_only() {
        let bytes = Arc::new(std::fs::read(REFERENCE_V1_1).unwrap());
        let mut via_hook = open_machine_config_source(image_source(bytes)).unwrap();
        let direct = open_machine_config(REFERENCE_V1_1).unwrap();
        assert_eq!(via_hook.file_version(), "1.1");
        assert_eq!(
            serde_json::to_value(via_hook.get_meta().unwrap()).unwrap(),
            serde_json::to_value(direct.get_meta().unwrap()).unwrap()
        );
        assert!(via_hook.save(None).is_err(), "must not write back through the hook's path");
    }

    #[test]
    fn hook_errors_surface() {
        let broken = FileSource::with_fapl_hook("not-on-disk.h5", |_| Err("nope".into()));
        let err = MachineConfigReader::open_source(broken.clone()).err().expect("must fail");
        assert!(err.to_string().contains("nope"), "{err}");
        assert!(open_machine_config_source(broken).is_err());
    }
}
