use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct SectionHeader {
    pub name: String,
    pub sh_type: u32,
    pub flags: u64,
    pub addr: u64,
    pub offset: u64,
    pub size: u64,
}

impl SectionHeader {
    pub fn type_name(&self) -> &'static str {
        match self.sh_type {
            0  => "NULL",
            1  => "PROGBITS",
            2  => "SYMTAB",
            3  => "STRTAB",
            4  => "RELA",
            5  => "HASH",
            6  => "DYNAMIC",
            7  => "NOTE",
            8  => "NOBITS",
            9  => "REL",
            10 => "SHLIB",
            11 => "DYNSYM",
            14 => "INIT_ARRAY",
            15 => "FINI_ARRAY",
            16 => "PREINIT_ARRAY",
            17 => "GROUP",
            18 => "SYMTAB_SHNDX",
            _  => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgramHeader {
    pub ph_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
}

impl ProgramHeader {
    pub fn type_name(&self) -> &'static str {
        match self.ph_type {
            0          => "NULL",
            1          => "LOAD",
            2          => "DYNAMIC",
            3          => "INTERP",
            4          => "NOTE",
            5          => "SHLIB",
            6          => "PHDR",
            7          => "TLS",
            0x6474e550 => "GNU_EH_FRAME",
            0x6474e551 => "GNU_STACK",
            0x6474e552 => "GNU_RELRO",
            0x6474e553 => "GNU_PROPERTY",
            _          => "UNKNOWN",
        }
    }

    pub fn flags_str(&self) -> String {
        let r = if self.flags & 4 != 0 { 'R' } else { '-' };
        let w = if self.flags & 2 != 0 { 'W' } else { '-' };
        let x = if self.flags & 1 != 0 { 'X' } else { '-' };
        format!("{}{}{}", r, w, x)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElfFile {
    pub magic: u32,
    pub class: u8,
    pub data_encoding: u8,
    pub os_abi: u8,
    pub e_type: u16,
    pub machine: u16,
    pub entry: u64,
    pub flags: u32,
    pub file_size: u64,
    pub sections: Vec<SectionHeader>,
    pub segments: Vec<ProgramHeader>,
}

impl ElfFile {
    pub fn class_name(&self) -> &'static str {
        match self.class {
            1 => "32-bit",
            2 => "64-bit",
            _ => "Unknown",
        }
    }

    pub fn encoding_name(&self) -> &'static str {
        match self.data_encoding {
            1 => "Little-endian",
            2 => "Big-endian",
            _ => "Unknown",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.e_type {
            0 => "ET_NONE",
            1 => "ET_REL (Relocatable)",
            2 => "ET_EXEC (Executable)",
            3 => "ET_DYN (Shared object)",
            4 => "ET_CORE",
            _ => "Unknown",
        }
    }

    pub fn machine_name(&self) -> &'static str {
        match self.machine {
            0x00 => "None",
            0x02 => "SPARC",
            0x03 => "x86",
            0x08 => "MIPS",
            0x14 => "PowerPC",
            0x15 => "PowerPC64",
            0x16 => "S390",
            0x28 => "ARM",
            0x32 => "IA-64",
            0x3E => "x86-64",
            0xB7 => "AArch64",
            0xF3 => "RISC-V",
            0xF7 => "BPF",
            _    => "Unknown",
        }
    }

    pub fn os_abi_name(&self) -> &'static str {
        match self.os_abi {
            0  => "System V",
            1  => "HP-UX",
            2  => "NetBSD",
            3  => "Linux",
            6  => "Solaris",
            7  => "AIX",
            8  => "IRIX",
            9  => "FreeBSD",
            12 => "OpenBSD",
            _  => "Unknown",
        }
    }
}

fn ru16(r: &mut impl Read, le: bool) -> io::Result<u16> {
    if le { r.read_u16::<LittleEndian>() } else { r.read_u16::<BigEndian>() }
}

fn ru32(r: &mut impl Read, le: bool) -> io::Result<u32> {
    if le { r.read_u32::<LittleEndian>() } else { r.read_u32::<BigEndian>() }
}

fn ru64(r: &mut impl Read, le: bool) -> io::Result<u64> {
    if le { r.read_u64::<LittleEndian>() } else { r.read_u64::<BigEndian>() }
}

// Read a pointer-sized value: u64 for 64-bit ELF, u32 zero-extended for 32-bit
fn raddr(r: &mut impl Read, is64: bool, le: bool) -> io::Result<u64> {
    if is64 { ru64(r, le) } else { Ok(ru32(r, le)? as u64) }
}

fn strtab_lookup(strtab: &[u8], offset: usize) -> String {
    if offset >= strtab.len() {
        return String::new();
    }
    let end = strtab[offset..].iter().position(|&b| b == 0).unwrap_or(strtab.len() - offset);
    String::from_utf8_lossy(&strtab[offset..offset + end]).into_owned()
}

impl ElfFile {
    pub fn parse(path: &PathBuf) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();

        // ELF ident (16 bytes)
        let mut ident = [0u8; 16];
        file.read_exact(&mut ident)?;

        if &ident[0..4] != b"\x7fELF" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not ELF"));
        }

        let magic = u32::from_be_bytes([ident[0], ident[1], ident[2], ident[3]]);
        let class = ident[4];
        let data_encoding = ident[5];
        let os_abi = ident[6];

        let is64 = class == 2;
        let le = data_encoding == 1;

        // ELF header fields (after the 16-byte ident)
        let e_type    = ru16(&mut file, le)?;
        let machine   = ru16(&mut file, le)?;
        let _version  = ru32(&mut file, le)?;
        let entry     = raddr(&mut file, is64, le)?;
        let phoff     = raddr(&mut file, is64, le)?;
        let shoff     = raddr(&mut file, is64, le)?;
        let flags     = ru32(&mut file, le)?;
        let _ehsize   = ru16(&mut file, le)?;
        let _phentsz  = ru16(&mut file, le)?;
        let phnum     = ru16(&mut file, le)?;
        let _shentsz  = ru16(&mut file, le)?;
        let shnum     = ru16(&mut file, le)?;
        let shstrndx  = ru16(&mut file, le)?;

        // --- Section headers ---
        // First pass: read raw entries so we can locate the string table.
        struct RawShdr { name_off: u32, sh_type: u32, flags: u64, addr: u64, offset: u64, size: u64 }
        let mut raw_shdrs: Vec<RawShdr> = Vec::new();

        if shoff > 0 && shnum > 0 {
            file.seek(SeekFrom::Start(shoff))?;
            for _ in 0..shnum {
                let name_off = ru32(&mut file, le)?;
                let sh_type  = ru32(&mut file, le)?;
                let flags    = raddr(&mut file, is64, le)?;
                let addr     = raddr(&mut file, is64, le)?;
                let offset   = raddr(&mut file, is64, le)?;
                let size     = raddr(&mut file, is64, le)?;
                let _link    = ru32(&mut file, le)?;
                let _info    = ru32(&mut file, le)?;
                let _align   = raddr(&mut file, is64, le)?;
                let _entsz   = raddr(&mut file, is64, le)?;
                raw_shdrs.push(RawShdr { name_off, sh_type, flags, addr, offset, size });
            }
        }

        // Read the section-name string table
        let strtab: Vec<u8> = if (shstrndx as usize) < raw_shdrs.len() {
            let s = &raw_shdrs[shstrndx as usize];
            if s.offset > 0 && s.size > 0 {
                file.seek(SeekFrom::Start(s.offset))?;
                let mut buf = vec![0u8; s.size as usize];
                file.read_exact(&mut buf)?;
                buf
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let sections: Vec<SectionHeader> = raw_shdrs
            .iter()
            .map(|rs| SectionHeader {
                name:    strtab_lookup(&strtab, rs.name_off as usize),
                sh_type: rs.sh_type,
                flags:   rs.flags,
                addr:    rs.addr,
                offset:  rs.offset,
                size:    rs.size,
            })
            .collect();

        // --- Program headers (segments) ---
        // Note: in 64-bit ELF, `flags` comes right after `type`;
        //       in 32-bit ELF, `flags` comes after `memsz`.
        let mut segments: Vec<ProgramHeader> = Vec::new();
        if phoff > 0 && phnum > 0 {
            file.seek(SeekFrom::Start(phoff))?;
            for _ in 0..phnum {
                let seg = if is64 {
                    let ph_type = ru32(&mut file, le)?;
                    let flags   = ru32(&mut file, le)?;
                    let offset  = ru64(&mut file, le)?;
                    let vaddr   = ru64(&mut file, le)?;
                    let _paddr  = ru64(&mut file, le)?;
                    let filesz  = ru64(&mut file, le)?;
                    let memsz   = ru64(&mut file, le)?;
                    let _align  = ru64(&mut file, le)?;
                    ProgramHeader { ph_type, flags, offset, vaddr, filesz, memsz }
                } else {
                    let ph_type = ru32(&mut file, le)?;
                    let offset  = ru32(&mut file, le)? as u64;
                    let vaddr   = ru32(&mut file, le)? as u64;
                    let _paddr  = ru32(&mut file, le)?;
                    let filesz  = ru32(&mut file, le)? as u64;
                    let memsz   = ru32(&mut file, le)? as u64;
                    let flags   = ru32(&mut file, le)?;
                    let _align  = ru32(&mut file, le)?;
                    ProgramHeader { ph_type, flags, offset, vaddr, filesz, memsz }
                };
                segments.push(seg);
            }
        }

        Ok(ElfFile { magic, class, data_encoding, os_abi, e_type, machine, entry, flags, file_size, sections, segments })
    }
}
