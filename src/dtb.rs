use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// Magic bytes
pub fn dtb_magic() -> Vec<Vec<u8>> {
    vec![b"\xd0\x0d\xfe\xed".to_vec()]
}

// Device Tree Blob (DTB) header struct
#[derive(Debug)]
pub struct DtbHeader {
    pub magic: u32,
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

impl DtbHeader {
    // Parses a DTB header from a byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 40 {
            return None;
        }
        Some(Self {
            magic: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            totalsize: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            off_dt_struct: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            off_dt_strings: u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            off_mem_rsvmap: u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            version: u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            last_comp_version: u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            boot_cpuid_phys: u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            size_dt_strings: u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
            size_dt_struct: u32::from_be_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]),
        })
    }
}

// Reads a DTB file and parses its header.
fn parse_dtb_header<P: AsRef<Path>>(path: P) -> io::Result<DtbHeader> {
    let mut file = File::open(path)?;
    let mut header_bytes = [0u8; 40];
    file.read_exact(&mut header_bytes)?;
    DtbHeader::from_bytes(&header_bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid DTB header"))
}

// Validates and parses a DTB file, returning its header if valid.
pub fn parse_dtb_file<P: AsRef<Path>>(path: P) -> io::Result<DtbHeader> {
    let header = parse_dtb_header(path)?;
    if !dtb_magic().iter().any(|magic| magic == &header.magic.to_be_bytes().to_vec()) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid DTB magic"));
    }
    Ok(header)
}