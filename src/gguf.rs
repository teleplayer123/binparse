use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum GgufValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Float32(f32),
    Bool(bool),
    String(String),
    Array(Vec<GgufValue>),
    Uint64(u64),
    Int64(i64),
    Float64(f64),
}

impl std::fmt::Display for GgufValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GgufValue::Uint8(v)   => write!(f, "{}", v),
            GgufValue::Int8(v)    => write!(f, "{}", v),
            GgufValue::Uint16(v)  => write!(f, "{}", v),
            GgufValue::Int16(v)   => write!(f, "{}", v),
            GgufValue::Uint32(v)  => write!(f, "{}", v),
            GgufValue::Int32(v)   => write!(f, "{}", v),
            GgufValue::Float32(v) => write!(f, "{:.6}", v),
            GgufValue::Bool(v)    => write!(f, "{}", v),
            GgufValue::String(v)  => write!(f, "{}", v),
            GgufValue::Array(v)   => write!(f, "[{} items]", v.len()),
            GgufValue::Uint64(v)  => write!(f, "{}", v),
            GgufValue::Int64(v)   => write!(f, "{}", v),
            GgufValue::Float64(v) => write!(f, "{:.6}", v),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataEntry {
    pub key: String,
    pub value: GgufValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GgufTensor {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub tensor_type: u32,
    pub offset: u64,
}

impl GgufTensor {
    pub fn type_name(&self) -> &'static str {
        match self.tensor_type {
            0  => "F32",
            1  => "F16",
            2  => "Q4_0",
            3  => "Q4_1",
            6  => "Q5_0",
            7  => "Q5_1",
            8  => "Q8_0",
            9  => "Q8_1",
            10 => "Q2_K",
            11 => "Q3_K",
            12 => "Q4_K",
            13 => "Q5_K",
            14 => "Q6_K",
            15 => "Q8_K",
            16 => "IQ2_XXS",
            17 => "IQ2_XS",
            18 => "IQ3_XXS",
            19 => "IQ1_S",
            20 => "IQ4_NL",
            21 => "IQ3_S",
            22 => "IQ2_S",
            23 => "IQ4_XS",
            24 => "I8",
            25 => "I16",
            26 => "I32",
            27 => "I64",
            28 => "F64",
            29 => "IQ1_M",
            _  => "UNKNOWN",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct GgufFile {
    pub magic: u32,
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_kv_count: u64,
    pub metadata: Vec<MetadataEntry>,
    pub tensors: Vec<GgufTensor>,
}

const GGUF_MAGIC: u32 = 0x46554747; // 'G' 'G' 'U' 'F'

fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let len = reader.read_u64::<LittleEndian>()?;
    let mut bytes = vec![0u8; len as usize];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_value<R: Read>(reader: &mut R, value_type: u32) -> io::Result<GgufValue> {
    match value_type {
        0  => Ok(GgufValue::Uint8(reader.read_u8()?)),
        1  => Ok(GgufValue::Int8(reader.read_i8()?)),
        2  => Ok(GgufValue::Uint16(reader.read_u16::<LittleEndian>()?)),
        3  => Ok(GgufValue::Int16(reader.read_i16::<LittleEndian>()?)),
        4  => Ok(GgufValue::Uint32(reader.read_u32::<LittleEndian>()?)),
        5  => Ok(GgufValue::Int32(reader.read_i32::<LittleEndian>()?)),
        6  => Ok(GgufValue::Float32(reader.read_f32::<LittleEndian>()?)),
        7  => Ok(GgufValue::Bool(reader.read_u8()? != 0)),
        8  => Ok(GgufValue::String(read_string(reader)?)),
        9  => {
            let elem_type = reader.read_u32::<LittleEndian>()?;
            let count = reader.read_u64::<LittleEndian>()?;
            let mut values = Vec::with_capacity(count.min(1_000_000) as usize);
            for _ in 0..count {
                values.push(read_value(reader, elem_type)?);
            }
            Ok(GgufValue::Array(values))
        }
        10 => Ok(GgufValue::Uint64(reader.read_u64::<LittleEndian>()?)),
        11 => Ok(GgufValue::Int64(reader.read_i64::<LittleEndian>()?)),
        12 => Ok(GgufValue::Float64(reader.read_f64::<LittleEndian>()?)),
        t  => Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown value type: {}", t))),
    }
}

impl GgufFile {
    pub fn parse(path: &PathBuf) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let magic = reader.read_u32::<LittleEndian>()?;
        if magic != GGUF_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not GGUF"));
        }

        let version = reader.read_u32::<LittleEndian>()?;

        // v1 used u32 for both counts; v2+ uses u64
        let (tensor_count, metadata_kv_count) = if version == 1 {
            (
                reader.read_u32::<LittleEndian>()? as u64,
                reader.read_u32::<LittleEndian>()? as u64,
            )
        } else {
            (
                reader.read_u64::<LittleEndian>()?,
                reader.read_u64::<LittleEndian>()?,
            )
        };

        let mut metadata = Vec::with_capacity(metadata_kv_count as usize);
        for _ in 0..metadata_kv_count {
            let key = read_string(&mut reader)?;
            let value_type = reader.read_u32::<LittleEndian>()?;
            let value = read_value(&mut reader, value_type)?;
            metadata.push(MetadataEntry { key, value });
        }

        let mut tensors = Vec::with_capacity(tensor_count as usize);
        for _ in 0..tensor_count {
            let name = read_string(&mut reader)?;
            let n_dims = reader.read_u32::<LittleEndian>()?;
            let mut dimensions = Vec::with_capacity(n_dims as usize);
            for _ in 0..n_dims {
                dimensions.push(reader.read_u64::<LittleEndian>()?);
            }
            let tensor_type = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u64::<LittleEndian>()?;
            tensors.push(GgufTensor { name, dimensions, tensor_type, offset });
        }

        Ok(GgufFile { magic, version, tensor_count, metadata_kv_count, metadata, tensors })
    }
}
