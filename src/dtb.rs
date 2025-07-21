use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

// Magic bytes
pub fn dtb_magic() -> Vec<Vec<u8>> {
    vec![b"\xd0\x0d\xfe\xed".to_vec()]
}

#[allow(dead_code)]  // Will remove directive once the function is used
fn dtb_aligned(size: usize) -> usize {
    const DTB_ALIGN: usize = 4;
    let rem = size % DTB_ALIGN;
    if rem == 0 {
        size
    } else {
        size + DTB_ALIGN - rem
    }
}

// Device Tree Blob (DTB) header struct
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug)]
pub struct DtbReserveEntry {
    pub addr: u64,
    pub size: u64,
}

impl DtbReserveEntry {
    // Parses a reserve entry from a byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }
        Some(Self {
            addr: u64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3],
                                      bytes[4], bytes[5], bytes[6], bytes[7]]),
            size: u64::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11],
                                      bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }
}

const BEGIN_NODE: u32 = 0x00000001;
const END_NODE: u32 = 0x00000002;
const PROP: u32 = 0x00000003;
const NOP: u32 = 0x00000004;
const END: u32 = 0x00000009;

#[derive(Debug, Clone, PartialEq)]
struct FdtProperty {
    len: u32,
    nameoff: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct DtbNode {
    name: String,
    property: Vec<FdtProperty>,
}

impl DtbNode {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Placeholder for parsing logic
        None
    }
}

#[derive(Debug, PartialEq)]
enum DtbBlocks {
    Header(DtbHeader),
    ReserveEntries(HashMap<String, u64>),
    Nodes(Vec<DtbNode>),
}



// Reads a DTB file and parses its header.
fn parse_dtb_header<P: AsRef<Path>>(path: P) -> io::Result<HashMap<String, DtbBlocks>> {
    const DTB_VERSION: u32 = 17;
    const DTB_COMPAT_VERSION: u32 = 16;
    let mut file = File::open(path)?;
    let mut header_bytes = [0u8; 40];
    let mut blocks = HashMap::new();
    let mut nodes: HashMap<String, Vec<FdtProperty>> = HashMap::new();

    // Read the first 40 bytes for the DTB header
    file.read_exact(&mut header_bytes)?;
    let dtb_header = DtbHeader::from_bytes(&header_bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid DTB header"))?;
    blocks.insert("header".to_string(), DtbBlocks::Header(dtb_header.clone()));

    // Check header version is expected value
    if dtb_header.version != DTB_VERSION && dtb_header.last_comp_version != DTB_COMPAT_VERSION {
        panic!("DTB header version is not an excepted value.");
    }

    // Read the reserve entries if any
    let mut reserve_entry_bytes = [0u8; 16];
    let mut reserve_entries = HashMap::new();
    // Seek to reserved memory map offset 
    file.seek(io::SeekFrom::Start(dtb_header.off_mem_rsvmap as u64))?;
    file.read_exact(&mut reserve_entry_bytes).unwrap();
    while let Some(entry) = DtbReserveEntry::from_bytes(&reserve_entry_bytes) {
        reserve_entries.insert(format!("{:x}", entry.addr), entry.size);
        if entry.addr == 0 && entry.size == 0 {
            break; // End of reserve entries addr and size are both zero
        }
    }
    // Add reserved entries to blocks
    if !reserve_entries.is_empty() {
        blocks.insert("reserve_entries".to_string(), DtbBlocks::ReserveEntries(reserve_entries));
    }

    // Parse device tree property nodes
    file.seek(io::SeekFrom::Start(dtb_header.off_dt_struct as u64))?;
    let mut struct_blocks = vec![0u8; dtb_aligned(dtb_header.size_dt_struct as usize)];
    file.read_exact(&mut struct_blocks)?;


    Ok(blocks)
}



// Validates and parses a DTB file, returning its header if valid.
pub fn parse_dtb_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let header = parse_dtb_header(path)
        .expect("Failed to parse DTB header");
    let magic_valid = if let Some(DtbBlocks::Header(dtb_header)) = header.get("header") {
        dtb_magic().iter().any(|magic| magic == &dtb_header.magic.to_be_bytes().to_vec())
    } else {
        false
    };
    if !magic_valid {
        dbg!("Invalid DTB magic");
    }
    //let mut output_file = fs::File::create(outfile)?;
    // Print the header and reserve entries
    header.iter().for_each(|(_key, block)| {
        match block {
            DtbBlocks::Header(header) => {
                println!("DTB Header: {:?}", header);
            }
            DtbBlocks::ReserveEntries(entries) => {
                println!("Reserve Entries:");
                for (addr, size) in entries {
                    println!("  Address: {}, Size: {}", addr, size);
                }
            }
            DtbBlocks::Nodes(nodes) => {
                println!("DTB Nodes:");
                for node in nodes {
                    println!("  Node: {:?}", node);
                }
            }
        }
    });
    Ok(())
}