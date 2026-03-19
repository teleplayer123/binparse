use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum GgufValue {
    Uint32(u32), Int32(i32), Float32(f32), Bool(bool), Uint64(u64), Int64(i64), Float64(f64), String(String), StringList(Vec<String>), Unsupported(u32),
}

pub struct MetadataEntry { pub key: String, pub value: GgufValue }
pub struct GgufTensor { pub name: String, pub dimensions: Vec<u64>, pub tensor_type: u32, pub offset: u64 }

pub struct GgufFile {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_kv_count: u32,
    pub metadata: Vec<MetadataEntry>,
    pub tensors: Vec<GgufTensor>,
}

impl GgufFile {
    pub fn parse(path: &PathBuf) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let magic = reader.read_u32::<LittleEndian>()?;
        if magic != 0x46554747 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not GGUF"));
        }

        let version = reader.read_u32::<LittleEndian>()?;
        let tensor_count = reader.read_u64::<LittleEndian>()?;
        let metadata_kv_count = reader.read_u32::<LittleEndian>()?;

        // GGUF v3 header has 4 bytes reserved/padding after kv_count.
        let _reserved = reader.read_u32::<LittleEndian>()?;

        Ok(GgufFile {
            version,
            tensor_count,
            metadata_kv_count,
            metadata: Vec::new(),
            tensors: Vec::new(),
        })
    }
}