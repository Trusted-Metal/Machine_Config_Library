//! Compound datasets as contract [`Table`]s.
//!
//! The `hdf5` crate only reads and writes compound types it knows statically, so the row bytes are
//! moved with `H5Dread`/`H5Dwrite` (hdf5-sys) using a memory type built from the dataset's own
//! type descriptor; everything else goes through the safe API. Members are limited to the
//! contract's dtypes (fixed-length strings and fixed-width numbers).

use hdf5::types::{CompoundField, CompoundType, FloatSize, IntSize, TypeDescriptor};
use hdf5::{Datatype, Extents, Group};
use hdf5_sys::h5d::{H5Dread, H5Dwrite};
use hdf5_sys::h5p::H5P_DEFAULT;
use hdf5_sys::h5s::H5S_ALL;
use mcf_extension_contract::{ColumnData, Dtype, Table, TableField};

use super::{join, read_attrs, unsupported, write_attrs};
use crate::error::{MachineConfigError, Result};

fn member_dtype(ty: &TypeDescriptor, path: &str) -> Result<(Dtype, Option<usize>)> {
    Ok(match ty {
        TypeDescriptor::Integer(IntSize::U1) => (Dtype::I8, None),
        TypeDescriptor::Integer(IntSize::U2) => (Dtype::I16, None),
        TypeDescriptor::Integer(IntSize::U4) => (Dtype::I32, None),
        TypeDescriptor::Integer(IntSize::U8) => (Dtype::I64, None),
        TypeDescriptor::Unsigned(IntSize::U1) => (Dtype::U8, None),
        TypeDescriptor::Unsigned(IntSize::U2) => (Dtype::U16, None),
        TypeDescriptor::Unsigned(IntSize::U4) => (Dtype::U32, None),
        TypeDescriptor::Unsigned(IntSize::U8) => (Dtype::U64, None),
        TypeDescriptor::Float(FloatSize::U4) => (Dtype::F32, None),
        TypeDescriptor::Float(FloatSize::U8) => (Dtype::F64, None),
        TypeDescriptor::FixedAscii(n) | TypeDescriptor::FixedUnicode(n) => (Dtype::Utf8, Some(*n)),
        other => return Err(unsupported(path, format!("unsupported compound member type {other}"))),
    })
}

fn member_descriptor(field: &TableField) -> TypeDescriptor {
    match field.dtype {
        Dtype::Utf8 => TypeDescriptor::FixedUnicode(field.size.unwrap_or(0)),
        Dtype::I8 => TypeDescriptor::Integer(IntSize::U1),
        Dtype::I16 => TypeDescriptor::Integer(IntSize::U2),
        Dtype::I32 => TypeDescriptor::Integer(IntSize::U4),
        Dtype::I64 => TypeDescriptor::Integer(IntSize::U8),
        Dtype::U8 => TypeDescriptor::Unsigned(IntSize::U1),
        Dtype::U16 => TypeDescriptor::Unsigned(IntSize::U2),
        Dtype::U32 => TypeDescriptor::Unsigned(IntSize::U4),
        Dtype::U64 => TypeDescriptor::Unsigned(IntSize::U8),
        Dtype::F32 => TypeDescriptor::Float(FloatSize::U4),
        Dtype::F64 => TypeDescriptor::Float(FloatSize::U8),
    }
}

fn h5_error(path: &str, what: &str) -> MachineConfigError {
    unsupported(path, format!("{what} failed"))
}

pub(super) fn read_table(ds: &hdf5::Dataset, compound: &CompoundType, path: &str) -> Result<Table> {
    let shape = ds.shape();
    if shape.len() != 1 {
        return Err(unsupported(path, format!("compound datasets must be 1-D, got rank {}", shape.len())));
    }
    let rows = shape[0];

    let mut fields = Vec::with_capacity(compound.fields.len());
    for f in &compound.fields {
        let (dtype, size) = member_dtype(&f.ty, &join(path, &f.name))?;
        fields.push(TableField { name: f.name.clone(), dtype, size });
    }

    let mem = Datatype::from_descriptor(&TypeDescriptor::Compound(compound.clone()))?;
    let mut buf = vec![0u8; rows * compound.size];
    if rows > 0 {
        let status = hdf5::sync::sync(|| unsafe {
            H5Dread(ds.id(), mem.id(), H5S_ALL, H5S_ALL, H5P_DEFAULT, buf.as_mut_ptr().cast())
        });
        if status < 0 {
            return Err(h5_error(path, "reading the compound dataset"));
        }
    }

    let mut columns = Vec::with_capacity(fields.len());
    for (f, member) in fields.iter().zip(&compound.fields) {
        let at = |row: usize| row * compound.size + member.offset;
        macro_rules! numeric {
            ($t:ty, $variant:ident) => {{
                let width = std::mem::size_of::<$t>();
                ColumnData::$variant(
                    (0..rows)
                        .map(|r| <$t>::from_ne_bytes(buf[at(r)..at(r) + width].try_into().expect("width")))
                        .collect(),
                )
            }};
        }
        columns.push(match f.dtype {
            Dtype::I8 => numeric!(i8, I8),
            Dtype::U8 => numeric!(u8, U8),
            Dtype::I16 => numeric!(i16, I16),
            Dtype::U16 => numeric!(u16, U16),
            Dtype::I32 => numeric!(i32, I32),
            Dtype::U32 => numeric!(u32, U32),
            Dtype::I64 => numeric!(i64, I64),
            Dtype::U64 => numeric!(u64, U64),
            Dtype::F32 => numeric!(f32, F32),
            Dtype::F64 => numeric!(f64, F64),
            Dtype::Utf8 => {
                let width = f.size.unwrap_or(0);
                let mut values = Vec::with_capacity(rows);
                for r in 0..rows {
                    let bytes = &buf[at(r)..at(r) + width];
                    let end = bytes.iter().position(|b| *b == 0).unwrap_or(width);
                    let text = std::str::from_utf8(&bytes[..end]).map_err(|_| {
                        unsupported(&format!("{}[{r}]", join(path, &f.name)), "string is not valid UTF-8")
                    })?;
                    values.push(text.to_string());
                }
                ColumnData::Utf8(values)
            }
        });
    }

    Ok(Table { fields, rows, columns, attrs: read_attrs(ds, path)? })
}

pub(super) fn write_table(parent: &Group, name: &str, table: &Table) -> Result<()> {
    let mut members = Vec::with_capacity(table.fields.len());
    let mut offset = 0;
    for (index, f) in table.fields.iter().enumerate() {
        let ty = member_descriptor(f);
        let size = ty.size();
        members.push(CompoundField { name: f.name.clone(), ty, offset, index });
        offset += size;
    }
    let row_size = offset;
    let descriptor = TypeDescriptor::Compound(CompoundType { fields: members.clone(), size: row_size });

    let dataset = parent
        .new_dataset_builder()
        .empty_as(&descriptor)
        .shape(Extents::from(vec![table.rows]))
        .create(name)?;

    if table.rows > 0 {
        let mut buf = vec![0u8; table.rows * row_size];
        for ((member, f), column) in members.iter().zip(&table.fields).zip(&table.columns) {
            macro_rules! numeric {
                ($values:expr) => {{
                    for (r, v) in $values.iter().enumerate() {
                        let bytes = v.to_ne_bytes();
                        let at = r * row_size + member.offset;
                        buf[at..at + bytes.len()].copy_from_slice(&bytes);
                    }
                }};
            }
            match column {
                ColumnData::Utf8(values) => {
                    let width = f.size.unwrap_or(0);
                    for (r, v) in values.iter().enumerate() {
                        let at = r * row_size + member.offset;
                        buf[at..at + v.len()].copy_from_slice(v.as_bytes());
                        debug_assert!(v.len() <= width, "validated: string fits the field");
                    }
                }
                ColumnData::I8(v) => numeric!(v),
                ColumnData::U8(v) => numeric!(v),
                ColumnData::I16(v) => numeric!(v),
                ColumnData::U16(v) => numeric!(v),
                ColumnData::I32(v) => numeric!(v),
                ColumnData::U32(v) => numeric!(v),
                ColumnData::I64(v) => numeric!(v),
                ColumnData::U64(v) => numeric!(v),
                ColumnData::F32(v) => numeric!(v),
                ColumnData::F64(v) => numeric!(v),
            }
        }
        let mem = Datatype::from_descriptor(&descriptor)?;
        let status = hdf5::sync::sync(|| unsafe {
            H5Dwrite(dataset.id(), mem.id(), H5S_ALL, H5S_ALL, H5P_DEFAULT, buf.as_ptr().cast())
        });
        if status < 0 {
            return Err(h5_error(name, "writing the compound dataset"));
        }
    }
    write_attrs(&dataset, &table.attrs)
}
