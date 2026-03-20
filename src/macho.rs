use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::mem;

// Mach-O Magic Numbers
const MH_MAGIC_64: u32 = 0xfeedfacf; // Standard
const MH_CIGAM_64: u32 = 0xcffafeed; // Byte-swapped (CIGAM is MAGIC backwards)

#[repr(C)]
#[derive(Debug, Default)]
struct MachHeader64 {
    magic: u32,
    cputype: i32,
    cpusubtype: i32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    flags: u32,
    reserved: u32,
}

impl MachHeader64 {
    /// Attempts to parse a MachHeader64 from a reader (like a File or Cursor).
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut buffer = [0u8; mem::size_of::<MachHeader64>()];
        reader.read_exact(&mut buffer)?;

        // SAFETY: We use std::ptr::read to copy the bytes into the struct.
        // We have verified the buffer size matches the struct size exactly.
        let mut header: Self = unsafe {
            std::ptr::read(buffer.as_ptr() as *const MachHeader64)
        };

        // Handle Endianness
        if header.magic == MH_CIGAM_64 {
            header.swap_endianness();
        } else if header.magic != MH_MAGIC_64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Mach-O 64-bit magic number",
            ));
        }

        Ok(header)
    }

    /// Swaps the byte order for all multi-byte fields in the struct.
    fn swap_endianness(&mut self) {
        self.magic = self.magic.swap_bytes();
        self.cputype = self.cputype.swap_bytes();
        self.cpusubtype = self.cpusubtype.swap_bytes();
        self.filetype = self.filetype.swap_bytes();
        self.ncmds = self.ncmds.swap_bytes();
        self.sizeofcmds = self.sizeofcmds.swap_bytes();
        self.flags = self.flags.swap_bytes();
        self.reserved = self.reserved.swap_bytes();
    }
}
