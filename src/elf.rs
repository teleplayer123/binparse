use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, Clone, PartialEq)]
pub struct ElfFile {
    pub magic: u32,
    pub data: Vec<u8>,
}

const ELF_MAGIC: u32 = 0x7f454c46; // 0x7F 'E' 'L' 'F'

impl ElfFile {
    pub fn parse(path: &std::path::PathBuf) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut magic_bytes = [0u8; 4];
        file.read_exact(&mut magic_bytes)?;

        // Convert bytes to u32 (little endian)
        let magic = u32::from_le_bytes(magic_bytes);

        if magic != ELF_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not ELF"));
        }

        // Read the rest of the file
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        Ok(ElfFile {
            magic,
            data,
        })
    }
}

pub fn from_elf(path: &std::path::PathBuf) -> io::Result<super::DataFile> {
    let elf_file = ElfFile::parse(path)?;
    Ok(super::DataFile {
        magic: elf_file.magic,
        version: None,
        tensor_count: None,
        metadata_kv_count: None,
        data: Some(elf_file.data),
    })
}