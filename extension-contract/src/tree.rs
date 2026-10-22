use std::collections::BTreeMap;

pub const EXTENSION_CONTRACT_VERSION: &str = "0.1.0";

/// Fixed-width storage types; each maps 1:1 to an HDF5 type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dtype {
    Utf8,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Dtype {
    pub const ALL: [Dtype; 11] = [
        Dtype::Utf8,
        Dtype::I8,
        Dtype::U8,
        Dtype::I16,
        Dtype::U16,
        Dtype::I32,
        Dtype::U32,
        Dtype::I64,
        Dtype::U64,
        Dtype::F32,
        Dtype::F64,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Dtype::Utf8 => "utf8",
            Dtype::I8 => "i8",
            Dtype::U8 => "u8",
            Dtype::I16 => "i16",
            Dtype::U16 => "u16",
            Dtype::I32 => "i32",
            Dtype::U32 => "u32",
            Dtype::I64 => "i64",
            Dtype::U64 => "u64",
            Dtype::F32 => "f32",
            Dtype::F64 => "f64",
        }
    }

    pub fn from_name(name: &str) -> Option<Dtype> {
        Dtype::ALL.into_iter().find(|d| d.name() == name)
    }

    pub fn is_numeric(self) -> bool {
        self != Dtype::Utf8
    }

    /// Tag byte used by the canonical digest.
    pub(crate) fn tag(self) -> u8 {
        match self {
            Dtype::Utf8 => 1,
            Dtype::I8 => 2,
            Dtype::U8 => 3,
            Dtype::I16 => 4,
            Dtype::U16 => 5,
            Dtype::I32 => 6,
            Dtype::U32 => 7,
            Dtype::I64 => 8,
            Dtype::U64 => 9,
            Dtype::F32 => 10,
            Dtype::F64 => 11,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Utf8(String),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl Scalar {
    pub fn dtype(&self) -> Dtype {
        match self {
            Scalar::Utf8(_) => Dtype::Utf8,
            Scalar::I8(_) => Dtype::I8,
            Scalar::U8(_) => Dtype::U8,
            Scalar::I16(_) => Dtype::I16,
            Scalar::U16(_) => Dtype::U16,
            Scalar::I32(_) => Dtype::I32,
            Scalar::U32(_) => Dtype::U32,
            Scalar::I64(_) => Dtype::I64,
            Scalar::U64(_) => Dtype::U64,
            Scalar::F32(_) => Dtype::F32,
            Scalar::F64(_) => Dtype::F64,
        }
    }
}

macro_rules! scalar_from {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(impl From<$ty> for Scalar {
            fn from(v: $ty) -> Self { Scalar::$variant(v) }
        })*
    };
}

scalar_from!(
    String => Utf8, i8 => I8, u8 => U8, i16 => I16, u16 => U16, i32 => I32, u32 => U32,
    i64 => I64, u64 => U64, f32 => F32, f64 => F64,
);

impl From<&str> for Scalar {
    fn from(v: &str) -> Self {
        Scalar::Utf8(v.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Scalar(Scalar),
    /// Every element's dtype must equal the owning [`Attr::dtype`].
    Array(Vec<Scalar>),
}

/// A group or dataset attribute: a scalar or a 1-D array of one dtype.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub dtype: Dtype,
    pub value: AttrValue,
}

impl Attr {
    pub fn scalar(value: impl Into<Scalar>) -> Self {
        let value = value.into();
        Attr { dtype: value.dtype(), value: AttrValue::Scalar(value) }
    }

    pub fn array(dtype: Dtype, values: Vec<Scalar>) -> Self {
        Attr { dtype, value: AttrValue::Array(values) }
    }

    /// The string value of a scalar `utf8` attribute.
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            AttrValue::Scalar(Scalar::Utf8(s)) => Some(s),
            _ => None,
        }
    }

    /// A scalar numeric attribute as `i64`; floats only when integral, `u64` only when it fits.
    pub fn as_i64(&self) -> Option<i64> {
        let AttrValue::Scalar(v) = &self.value else { return None };
        match v {
            Scalar::I8(x) => Some(i64::from(*x)),
            Scalar::U8(x) => Some(i64::from(*x)),
            Scalar::I16(x) => Some(i64::from(*x)),
            Scalar::U16(x) => Some(i64::from(*x)),
            Scalar::I32(x) => Some(i64::from(*x)),
            Scalar::U32(x) => Some(i64::from(*x)),
            Scalar::I64(x) => Some(*x),
            Scalar::U64(x) => i64::try_from(*x).ok(),
            Scalar::F32(x) if x.fract() == 0.0 => Some(*x as i64),
            Scalar::F64(x) if x.fract() == 0.0 => Some(*x as i64),
            _ => None,
        }
    }

    /// A scalar numeric attribute as `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        let AttrValue::Scalar(v) = &self.value else { return None };
        match v {
            Scalar::F32(x) => Some(f64::from(*x)),
            Scalar::F64(x) => Some(*x),
            Scalar::Utf8(_) => None,
            other => self.as_i64().map(|n| n as f64).or(match other {
                Scalar::U64(x) => Some(*x as f64),
                _ => None,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatasetData {
    I8(Vec<i8>),
    U8(Vec<u8>),
    I16(Vec<i16>),
    U16(Vec<u16>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl DatasetData {
    pub fn dtype(&self) -> Dtype {
        match self {
            DatasetData::I8(_) => Dtype::I8,
            DatasetData::U8(_) => Dtype::U8,
            DatasetData::I16(_) => Dtype::I16,
            DatasetData::U16(_) => Dtype::U16,
            DatasetData::I32(_) => Dtype::I32,
            DatasetData::U32(_) => Dtype::U32,
            DatasetData::I64(_) => Dtype::I64,
            DatasetData::U64(_) => Dtype::U64,
            DatasetData::F32(_) => Dtype::F32,
            DatasetData::F64(_) => Dtype::F64,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            DatasetData::I8(v) => v.len(),
            DatasetData::U8(v) => v.len(),
            DatasetData::I16(v) => v.len(),
            DatasetData::U16(v) => v.len(),
            DatasetData::I32(v) => v.len(),
            DatasetData::U32(v) => v.len(),
            DatasetData::I64(v) => v.len(),
            DatasetData::U64(v) => v.len(),
            DatasetData::F32(v) => v.len(),
            DatasetData::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// N-d numeric array stored row-major in `data`; `data.len()` equals the product of `shape`
/// (empty shape = scalar).
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub shape: Vec<usize>,
    pub data: DatasetData,
    pub attrs: BTreeMap<String, Attr>,
}

impl Dataset {
    pub fn dtype(&self) -> Dtype {
        self.data.dtype()
    }
}

/// Order of attrs/children/datasets is not significant; the maps sort by UTF-8 bytes, which is
/// also the canonical order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtensionTree {
    pub attrs: BTreeMap<String, Attr>,
    pub children: BTreeMap<String, ExtensionTree>,
    pub datasets: BTreeMap<String, Dataset>,
}
