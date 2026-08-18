//! Mock v1.1 adapter — test artifact only, exercises the change-category
//! architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
//! change. See `docs/migrations/mock_v1_0_to_v1_1.md` for the full manifest.
//!
//! Design: the reader/writer *delegate* to the real, public v1.0 adapter for
//! the whole file, then patch exactly the 10 documented differences, rather
//! than reimplementing or reaching into v1.0's private parsing internals.
//! "Unchanged" subcomponents (light_source, collimator, scanner_card,
//! clearbox, sfcf, opcua) are never re-tested here — they're already covered
//! by the existing suite exercising the real v1.0 adapter. Mirrors
//! `nodejs/tests/mockV1_1.ts`'s design exactly.
//!
//! Lives at `rust/tests/mock_v1_1/mod.rs` (not a flat `tests/mock_v1_1.rs`)
//! so it's an ordinary module `mod`-included by `adapter_migration_test.rs`,
//! not a second, independent integration-test binary that Cargo would try to
//! compile on its own — a flat file directly under `tests/` is always
//! auto-registered as its own test target.
//!
//! It needs only the library's `pub` items — `Hdf5AdapterV1_0::parse`/
//! `Hdf5WriterV1_0::write` are already public, so no new exports were needed
//! to build this. There is no dispatch-table injection here (unlike Python's
//! `_ADAPTERS` or Node's `_READERS`/`_WRITERS`): Rust's public dispatcher is
//! a hardcoded `match` in `reader.rs`/`writer.rs`, not a registry, so
//! AV-09–11 test this mock directly rather than through
//! `MachineConfigReader`/`MachineConfigWriter` (see VALIDATION_PLAN.md §9.3
//! for the reasoning).
//!
//! Verified against the real `hdf5-metno` 0.12.5 source (not assumed) that
//! `File::open_rw`, `Location::delete_attr`, `Group::create_group`, and
//! `Location::new_attr::<T>().create(name)?.write_scalar(&v)?` are all part
//! of its stable public API before committing to this design.

use hdf5::types::VarLenUnicode;
use hdf5::{File as H5File, Group};
use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_0::writer::Hdf5WriterV1_0;
use machine_config::error::Result;
use machine_config::{MachineConfig, MockConfigBuilder};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// MockV1_1Layout — on-disk constants that differ from v1.0
// ---------------------------------------------------------------------------

pub const FILE_VERSION: &str = "1.1-mock";
pub const ATTR_FACILITY_ID: &str = "Facility_ID";
pub const ATTR_CONFIG_AUTHOR: &str = "Config_Author";
pub const ATTR_MACHINE_LABEL: &str = "Machine_Label";
pub const ATTR_FOCAL_DISTANCE: &str = "Focal_Distance";
pub const ATTR_BP_WIDTH: &str = "Width";
pub const ATTR_BP_HEIGHT: &str = "Height";

fn read_opt_str(loc: &Group, key: &str) -> Option<String> {
    let v = loc.attr(key).ok()?.read_scalar::<VarLenUnicode>().ok()?;
    let trimmed = v.as_str().trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn read_opt_f64(loc: &Group, key: &str) -> Option<f64> {
    loc.attr(key).ok()?.read_scalar::<f64>().ok()
}

fn write_str_attr(loc: &Group, key: &str, value: &str) -> Result<()> {
    let v: VarLenUnicode = value
        .parse()
        .map_err(|e| machine_config::MachineConfigError::Parse(format!("VarLenUnicode: {e:?}")))?;
    loc.new_attr::<VarLenUnicode>().create(key)?.write_scalar(&v)?;
    Ok(())
}

/// Reads `old_key` as a string, writes it to `new_key`, deletes `old_key`.
/// No-op (beyond the delete attempt) if `old_key` is absent — mirrors Node's
/// `renameAttr`'s early return.
fn rename_attr_str(loc: &Group, old_key: &str, new_key: &str) -> Result<()> {
    if let Ok(attr) = loc.attr(old_key) {
        if let Ok(v) = attr.read_scalar::<VarLenUnicode>() {
            write_str_attr(loc, new_key, v.as_str())?;
        }
    }
    let _ = loc.delete_attr(old_key);
    Ok(())
}

/// Reads `old_key` (float64) from `from`, writes it to `new_key` on `to`,
/// deletes `old_key` from `from`. `from` and `to` may be the same group
/// (a same-group rename) or different groups (a move).
fn move_attr_f64(from: &Group, to: &Group, old_key: &str, new_key: &str) -> Result<()> {
    if let Ok(attr) = from.attr(old_key) {
        if let Ok(v) = attr.read_scalar::<f64>() {
            to.new_attr::<f64>().create(new_key)?.write_scalar(&v)?;
        }
    }
    let _ = from.delete_attr(old_key);
    Ok(())
}

// ---------------------------------------------------------------------------
// MockV1_1Reader — delegates to the real v1.0 parser, then patches the delta
// ---------------------------------------------------------------------------

pub struct MockV1_1Reader {
    path: PathBuf,
}

impl MockV1_1Reader {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    pub fn parse(&self) -> Result<MachineConfig> {
        // Step 1: the real v1.0 parser correctly reads every subcomponent
        // this mock doesn't change. Fields it can't find (renamed, moved, or
        // removed) come back null/empty — never a panic, since none of the
        // 10 changes remove or rename a *group*, only attributes within one.
        let base = Hdf5AdapterV1_0::open(&self.path)?.parse()?;
        let MachineConfig { mut meta, mut machine, mut optical_trains, opcua } = base;

        // Step 2: patch exactly the 10 documented differences by reading
        // their real, mock-v1.1 locations directly.
        let f = H5File::open(&self.path)?;
        let root = f.group("/")?;
        let machine_grp = f.group("Machine")?;
        let dims = machine_grp.group("Dimensions").ok();

        let facility_id = read_opt_str(&root, ATTR_FACILITY_ID);
        let config_author = read_opt_str(&root, ATTR_CONFIG_AUTHOR);
        let machine_label = read_opt_str(&machine_grp, ATTR_MACHINE_LABEL).unwrap_or_default();
        let build_plate_x = dims.as_ref().and_then(|d| read_opt_f64(d, ATTR_BP_WIDTH));
        let build_plate_y = dims.as_ref().and_then(|d| read_opt_f64(d, ATTR_BP_HEIGHT));
        let build_plate_z = dims.as_ref().and_then(|d| read_opt_f64(d, "Build_Plate_Z_Dimension"));
        let build_plate_radius =
            dims.as_ref().and_then(|d| read_opt_f64(d, "Build_Plate_Corner_Radius"));

        for train in optical_trains.iter_mut() {
            if let Ok(scanner_grp) =
                machine_grp.group(&format!("Optical_Trains/{}/Scanner", train.train_id))
            {
                train.scanner.working_distance = read_opt_f64(&scanner_grp, ATTR_FOCAL_DISTANCE);
            }
        }

        // The base parser doesn't know Facility_ID/Config_Author are typed
        // fields, so it swept them into meta.extra as unknown attrs. Strip
        // them out there before setting the typed fields, or they'd exist in
        // both places — and get written twice, colliding, on the next write.
        // This is the exact bug found and fixed in Node's mock this session.
        meta.extra.shift_remove(ATTR_FACILITY_ID);
        meta.extra.shift_remove(ATTR_CONFIG_AUTHOR);
        meta.facility_id = facility_id;
        meta.config_author = config_author;

        machine.machine_name = machine_label;
        machine.build_plate_x = build_plate_x;
        machine.build_plate_y = build_plate_y;
        machine.build_plate_z = build_plate_z;
        machine.build_plate_radius = build_plate_radius;
        // gas_flow_direction / recoat_direction: already None from the base
        // parse (the attrs are genuinely absent) — no patch needed.

        Ok(MachineConfig { meta, machine, optical_trains, opcua })
    }
}

// ---------------------------------------------------------------------------
// MockV1_1Writer — writes a real v1.0-shaped file, then patches the delta
// ---------------------------------------------------------------------------

pub struct MockV1_1Writer<'a> {
    config: &'a MachineConfig,
}

impl<'a> MockV1_1Writer<'a> {
    pub fn new(config: &'a MachineConfig) -> Self {
        Self { config }
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Step 1: the real v1.0 writer correctly writes every subcomponent
        // this mock doesn't change, plus File_Version itself (already
        // "1.1-mock" on the input config — the writer just persists whatever
        // string is there).
        Hdf5WriterV1_0::new(self.config).write(&path)?;

        // Step 2: patch exactly the 10 documented differences in place.
        let f = H5File::open_rw(&path)?;
        let root = f.group("/")?;

        // ADDITION (x2)
        write_str_attr(&root, ATTR_FACILITY_ID, self.config.meta.facility_id.as_deref().unwrap_or(""))?;
        write_str_attr(&root, ATTR_CONFIG_AUTHOR, self.config.meta.config_author.as_deref().unwrap_or(""))?;

        let machine = f.group("Machine")?;

        // REMOVAL (x2)
        let _ = machine.delete_attr("Gas_Flow_Direction");
        let _ = machine.delete_attr("Recoat_Direction");

        // NAME (Machine_Name -> Machine_Label)
        rename_attr_str(&machine, "Machine_Name", ATTR_MACHINE_LABEL)?;

        // PATH / NAME+PATH: build-plate values move into Machine/Dimensions/.
        // Unit attrs are unaffected — they stay on Machine/ per the manifest.
        let dims = machine.create_group("Dimensions")?;
        move_attr_f64(&machine, &dims, "Build_Plate_X_Dimension", ATTR_BP_WIDTH)?;
        move_attr_f64(&machine, &dims, "Build_Plate_Y_Dimension", ATTR_BP_HEIGHT)?;
        move_attr_f64(&machine, &dims, "Build_Plate_Z_Dimension", "Build_Plate_Z_Dimension")?;
        move_attr_f64(&machine, &dims, "Build_Plate_Corner_Radius", "Build_Plate_Corner_Radius")?;

        // NAME (Working_Distance -> Focal_Distance), once per optical train.
        let trains_grp = machine.group("Optical_Trains")?;
        for name in trains_grp.member_names()? {
            if let Ok(scanner_grp) = trains_grp.group(&format!("{name}/Scanner")) {
                move_attr_f64(&scanner_grp, &scanner_grp, "Working_Distance", ATTR_FOCAL_DISTANCE)?;
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's `_make_config()` / Node's `makeMockConfig`
// ---------------------------------------------------------------------------

pub fn make_mock_config(
    machine_name: Option<&str>,
    facility_id: Option<&str>,
    config_author: Option<&str>,
) -> MachineConfig {
    let mut builder = MockConfigBuilder::new(1);
    builder.build_plate_x = 250.0;
    builder.build_plate_y = 175.0; // distinct from x so name+path assertions are unambiguous
    builder.machine_name = machine_name.unwrap_or("MigrationTestMachine").to_string();

    let mut cfg = builder.build();
    cfg.meta.file_version = FILE_VERSION.to_string();
    cfg.meta.facility_id = facility_id.map(str::to_string);
    cfg.meta.config_author = config_author.map(str::to_string);
    cfg.machine.gas_flow_direction = None; // absent in v1.1-mock by design
    cfg.machine.recoat_direction = None;
    // MockConfigBuilder never sets this (always None) — give it a real value
    // so the PATH category (Build_Plate_Corner_Radius) has something
    // non-trivial to verify preservation of, mirroring Python's `_make_config`.
    cfg.machine.build_plate_radius = Some(10.0);
    cfg
}
