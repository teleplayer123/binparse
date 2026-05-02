use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::{self, Cursor, Read};

const MH_MAGIC_64: u32 = 0xfeedfacf;
const MH_CIGAM_64: u32 = 0xcffafeed;

// Load command type constants
const LC_SEGMENT_64: u32           = 0x00000019;
const LC_SYMTAB: u32               = 0x00000002;
const LC_DYSYMTAB: u32             = 0x0000000b;
const LC_LOAD_DYLIB: u32           = 0x0000000c;
const LC_ID_DYLIB: u32             = 0x0000000d;
const LC_LOAD_DYLINKER: u32        = 0x0000000e;
const LC_ID_DYLINKER: u32          = 0x0000000f;
const LC_LOAD_WEAK_DYLIB: u32      = 0x80000018;
const LC_UUID: u32                 = 0x0000001b;
const LC_RPATH: u32                = 0x8000001c;
const LC_CODE_SIGNATURE: u32       = 0x0000001d;
const LC_SEGMENT_SPLIT_INFO: u32   = 0x0000001e;
const LC_REEXPORT_DYLIB: u32       = 0x8000001f;
const LC_VERSION_MIN_MACOSX: u32   = 0x00000024;
const LC_VERSION_MIN_IPHONEOS: u32 = 0x00000025;
const LC_FUNCTION_STARTS: u32      = 0x00000026;
const LC_MAIN: u32                 = 0x80000028;
const LC_DATA_IN_CODE: u32         = 0x00000029;
const LC_SOURCE_VERSION: u32       = 0x0000002a;
const LC_ENCRYPTION_INFO_64: u32   = 0x0000002c;
const LC_VERSION_MIN_TVOS: u32     = 0x0000002f;
const LC_VERSION_MIN_WATCHOS: u32  = 0x00000030;
const LC_BUILD_VERSION: u32        = 0x00000032;
const LC_DYLD_INFO: u32            = 0x00000022;
const LC_DYLD_INFO_ONLY: u32       = 0x80000022;
const LC_DYLD_EXPORTS_TRIE: u32    = 0x80000033;
const LC_DYLD_CHAINED_FIXUPS: u32  = 0x80000034;
const LC_LINKER_OPTION: u32        = 0x00000036;

fn lc_type_name(cmd: u32) -> &'static str {
    match cmd {
        LC_SEGMENT_64           => "LC_SEGMENT_64",
        LC_SYMTAB               => "LC_SYMTAB",
        LC_DYSYMTAB             => "LC_DYSYMTAB",
        LC_LOAD_DYLIB           => "LC_LOAD_DYLIB",
        LC_ID_DYLIB             => "LC_ID_DYLIB",
        LC_LOAD_DYLINKER        => "LC_LOAD_DYLINKER",
        LC_ID_DYLINKER          => "LC_ID_DYLINKER",
        LC_LOAD_WEAK_DYLIB      => "LC_LOAD_WEAK_DYLIB",
        LC_UUID                 => "LC_UUID",
        LC_RPATH                => "LC_RPATH",
        LC_CODE_SIGNATURE       => "LC_CODE_SIGNATURE",
        LC_SEGMENT_SPLIT_INFO   => "LC_SEGMENT_SPLIT_INFO",
        LC_REEXPORT_DYLIB       => "LC_REEXPORT_DYLIB",
        LC_VERSION_MIN_MACOSX   => "LC_VERSION_MIN_MACOSX",
        LC_VERSION_MIN_IPHONEOS => "LC_VERSION_MIN_IPHONEOS",
        LC_FUNCTION_STARTS      => "LC_FUNCTION_STARTS",
        LC_MAIN                 => "LC_MAIN",
        LC_DATA_IN_CODE         => "LC_DATA_IN_CODE",
        LC_SOURCE_VERSION       => "LC_SOURCE_VERSION",
        LC_ENCRYPTION_INFO_64   => "LC_ENCRYPTION_INFO_64",
        LC_VERSION_MIN_TVOS     => "LC_VERSION_MIN_TVOS",
        LC_VERSION_MIN_WATCHOS  => "LC_VERSION_MIN_WATCHOS",
        LC_BUILD_VERSION        => "LC_BUILD_VERSION",
        LC_DYLD_INFO            => "LC_DYLD_INFO",
        LC_DYLD_INFO_ONLY       => "LC_DYLD_INFO_ONLY",
        LC_DYLD_EXPORTS_TRIE    => "LC_DYLD_EXPORTS_TRIE",
        LC_DYLD_CHAINED_FIXUPS  => "LC_DYLD_CHAINED_FIXUPS",
        LC_LINKER_OPTION        => "LC_LINKER_OPTION",
        _                       => "LC_UNKNOWN",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadCommand {
    Segment64 { name: String, vmaddr: u64, vmsize: u64, maxprot: i32, initprot: i32, nsects: u32 },
    Dylib     { cmd: u32, name: String, current_version: u32, compat_version: u32 },
    Dylinker  { cmd: u32, name: String },
    Uuid      { uuid: [u8; 16] },
    Main      { entryoff: u64, stacksize: u64 },
    SourceVersion { version: u64 },
    BuildVersion  { platform: u32, minos: u32, sdk: u32 },
    VersionMin    { cmd: u32, version: u32, sdk: u32 },
    Symtab        { nsyms: u32, strsize: u32 },
    LinkeditData  { cmd: u32, dataoff: u32, datasize: u32 },
    Rpath         { path: String },
    EncryptionInfo64 { cryptoff: u32, cryptsize: u32, cryptid: u32 },
    Unknown       { cmd: u32 },
}

impl LoadCommand {
    pub fn type_name(&self) -> &'static str {
        match self {
            LoadCommand::Segment64 { .. }       => "LC_SEGMENT_64",
            LoadCommand::Dylib { cmd, .. }      => lc_type_name(*cmd),
            LoadCommand::Dylinker { cmd, .. }   => lc_type_name(*cmd),
            LoadCommand::Uuid { .. }            => "LC_UUID",
            LoadCommand::Main { .. }            => "LC_MAIN",
            LoadCommand::SourceVersion { .. }   => "LC_SOURCE_VERSION",
            LoadCommand::BuildVersion { .. }    => "LC_BUILD_VERSION",
            LoadCommand::VersionMin { cmd, .. } => lc_type_name(*cmd),
            LoadCommand::Symtab { .. }          => "LC_SYMTAB",
            LoadCommand::LinkeditData { cmd, .. } => lc_type_name(*cmd),
            LoadCommand::Rpath { .. }           => "LC_RPATH",
            LoadCommand::EncryptionInfo64 { .. } => "LC_ENCRYPTION_INFO_64",
            LoadCommand::Unknown { cmd }        => lc_type_name(*cmd),
        }
    }

    pub fn detail(&self) -> String {
        match self {
            LoadCommand::Segment64 { name, vmaddr, vmsize, maxprot, initprot, nsects } =>
                format!("{:16}  {:#018x}  size: {:#010x}  prot: {}/{} sections: {}",
                    name, vmaddr, vmsize, prot_str(*initprot), prot_str(*maxprot), nsects),

            LoadCommand::Dylib { name, current_version, compat_version, .. } =>
                format!("{}  (current: {}  compat: {})",
                    name, fmt_version(*current_version), fmt_version(*compat_version)),

            LoadCommand::Dylinker { name, .. } => name.clone(),

            LoadCommand::Uuid { uuid } =>
                format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
                    u32::from_be_bytes([uuid[0], uuid[1], uuid[2], uuid[3]]),
                    u16::from_be_bytes([uuid[4], uuid[5]]),
                    u16::from_be_bytes([uuid[6], uuid[7]]),
                    u16::from_be_bytes([uuid[8], uuid[9]]),
                    {
                        let b = &uuid[10..16];
                        (b[0] as u64) << 40 | (b[1] as u64) << 32 | (b[2] as u64) << 24
                            | (b[3] as u64) << 16 | (b[4] as u64) << 8 | b[5] as u64
                    }),

            LoadCommand::Main { entryoff, stacksize } =>
                format!("entry offset: {:#010x}  stack: {} bytes", entryoff, stacksize),

            LoadCommand::SourceVersion { version } =>
                format!("version: {}", fmt_source_version(*version)),

            LoadCommand::BuildVersion { platform, minos, sdk } =>
                format!("platform: {}  min OS: {}  SDK: {}",
                    platform_name(*platform), fmt_version(*minos), fmt_version(*sdk)),

            LoadCommand::VersionMin { version, sdk, .. } =>
                format!("version: {}  SDK: {}", fmt_version(*version), fmt_version(*sdk)),

            LoadCommand::Symtab { nsyms, strsize } =>
                format!("{} symbols  string table: {} bytes", nsyms, strsize),

            LoadCommand::LinkeditData { dataoff, datasize, .. } =>
                format!("offset: {:#010x}  size: {} bytes", dataoff, datasize),

            LoadCommand::Rpath { path } => path.clone(),

            LoadCommand::EncryptionInfo64 { cryptoff, cryptsize, cryptid } =>
                format!("offset: {:#010x}  size: {}  id: {}", cryptoff, cryptsize, cryptid),

            LoadCommand::Unknown { cmd } =>
                format!("cmd: {:#010x}", cmd),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachHeader64 {
    pub magic: u32,
    pub cputype: i32,
    pub cpusubtype: i32,
    pub filetype: u32,
    pub ncmds: u32,
    pub sizeofcmds: u32,
    pub flags: u32,
    pub load_commands: Vec<LoadCommand>,
}

impl MachHeader64 {
    pub fn cpu_type_name(&self) -> &'static str {
        match self.cputype as u32 {
            0x01000007 => "x86-64",
            0x0100000c => "ARM64",
            0x0000000c => "ARM",
            0x00000007 => "x86",
            0x00000012 => "PowerPC",
            0x01000012 => "PowerPC64",
            _          => "Unknown",
        }
    }

    pub fn file_type_name(&self) -> &'static str {
        match self.filetype {
            1  => "MH_OBJECT (Relocatable)",
            2  => "MH_EXECUTE (Executable)",
            3  => "MH_FVMLIB",
            4  => "MH_CORE",
            5  => "MH_PRELOAD",
            6  => "MH_DYLIB (Dynamic library)",
            7  => "MH_DYLINKER",
            8  => "MH_BUNDLE",
            9  => "MH_DYLIB_STUB",
            10 => "MH_DSYM (Debug symbols)",
            11 => "MH_KEXT_BUNDLE",
            _  => "Unknown",
        }
    }

    pub fn parse<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut magic_buf = [0u8; 4];
        reader.read_exact(&mut magic_buf)?;
        let magic_as_le = u32::from_le_bytes(magic_buf);

        let le = match magic_as_le {
            MH_MAGIC_64 => true,
            MH_CIGAM_64 => false,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Mach-O 64-bit magic number")),
        };

        let cputype    = ri32(&mut reader, le)?;
        let cpusubtype = ri32(&mut reader, le)?;
        let filetype   = ru32(&mut reader, le)?;
        let ncmds      = ru32(&mut reader, le)?;
        let sizeofcmds = ru32(&mut reader, le)?;
        let flags      = ru32(&mut reader, le)?;
        let _reserved  = ru32(&mut reader, le)?;

        let mut load_commands = Vec::with_capacity(ncmds as usize);
        for _ in 0..ncmds {
            let cmd     = ru32(&mut reader, le)?;
            let cmdsize = ru32(&mut reader, le)?;
            // read the body of this command (cmdsize includes the 8-byte cmd+cmdsize header)
            let body_len = (cmdsize as usize).saturating_sub(8);
            let mut body = vec![0u8; body_len];
            reader.read_exact(&mut body)?;
            load_commands.push(parse_load_command(cmd, &body, le));
        }

        Ok(MachHeader64 { magic: MH_MAGIC_64, cputype, cpusubtype, filetype, ncmds, sizeofcmds, flags, load_commands })
    }
}

// --- Load command parser ---

fn parse_load_command(cmd: u32, body: &[u8], le: bool) -> LoadCommand {
    // All errors fall through to Unknown — a malformed LC doesn't abort the whole parse.
    parse_load_command_inner(cmd, body, le).unwrap_or(LoadCommand::Unknown { cmd })
}

fn parse_load_command_inner(cmd: u32, body: &[u8], le: bool) -> io::Result<LoadCommand> {
    let mut r = Cursor::new(body);
    match cmd {
        LC_SEGMENT_64 => {
            let mut seg_bytes = [0u8; 16];
            r.read_exact(&mut seg_bytes)?;
            let name = String::from_utf8_lossy(&seg_bytes)
                .trim_end_matches('\0')
                .to_owned();
            let vmaddr   = ru64(&mut r, le)?;
            let vmsize   = ru64(&mut r, le)?;
            let _fileoff  = ru64(&mut r, le)?;
            let _filesz   = ru64(&mut r, le)?;
            let maxprot  = ri32(&mut r, le)?;
            let initprot = ri32(&mut r, le)?;
            let nsects   = ru32(&mut r, le)?;
            Ok(LoadCommand::Segment64 { name, vmaddr, vmsize, maxprot, initprot, nsects })
        }

        LC_LOAD_DYLIB | LC_ID_DYLIB | LC_LOAD_WEAK_DYLIB | LC_REEXPORT_DYLIB => {
            // lc_str_union: offset from LC start (includes 8-byte header) → subtract 8
            let name_off        = ru32(&mut r, le)? as usize;
            let _timestamp      = ru32(&mut r, le)?;
            let current_version = ru32(&mut r, le)?;
            let compat_version  = ru32(&mut r, le)?;
            let name = lc_str(body, name_off.saturating_sub(8));
            Ok(LoadCommand::Dylib { cmd, name, current_version, compat_version })
        }

        LC_LOAD_DYLINKER | LC_ID_DYLINKER => {
            let name_off = ru32(&mut r, le)? as usize;
            let name = lc_str(body, name_off.saturating_sub(8));
            Ok(LoadCommand::Dylinker { cmd, name })
        }

        LC_UUID => {
            let mut uuid = [0u8; 16];
            r.read_exact(&mut uuid)?;
            Ok(LoadCommand::Uuid { uuid })
        }

        LC_MAIN => {
            let entryoff  = ru64(&mut r, le)?;
            let stacksize = ru64(&mut r, le)?;
            Ok(LoadCommand::Main { entryoff, stacksize })
        }

        LC_SOURCE_VERSION => {
            let version = ru64(&mut r, le)?;
            Ok(LoadCommand::SourceVersion { version })
        }

        LC_BUILD_VERSION => {
            let platform = ru32(&mut r, le)?;
            let minos    = ru32(&mut r, le)?;
            let sdk      = ru32(&mut r, le)?;
            Ok(LoadCommand::BuildVersion { platform, minos, sdk })
        }

        LC_VERSION_MIN_MACOSX | LC_VERSION_MIN_IPHONEOS |
        LC_VERSION_MIN_TVOS   | LC_VERSION_MIN_WATCHOS => {
            let version = ru32(&mut r, le)?;
            let sdk     = ru32(&mut r, le)?;
            Ok(LoadCommand::VersionMin { cmd, version, sdk })
        }

        LC_SYMTAB => {
            let _symoff  = ru32(&mut r, le)?;
            let nsyms    = ru32(&mut r, le)?;
            let _stroff  = ru32(&mut r, le)?;
            let strsize  = ru32(&mut r, le)?;
            Ok(LoadCommand::Symtab { nsyms, strsize })
        }

        LC_CODE_SIGNATURE | LC_FUNCTION_STARTS | LC_DATA_IN_CODE |
        LC_SEGMENT_SPLIT_INFO | LC_DYLD_EXPORTS_TRIE | LC_DYLD_CHAINED_FIXUPS => {
            let dataoff  = ru32(&mut r, le)?;
            let datasize = ru32(&mut r, le)?;
            Ok(LoadCommand::LinkeditData { cmd, dataoff, datasize })
        }

        LC_RPATH => {
            let path_off = ru32(&mut r, le)? as usize;
            let path = lc_str(body, path_off.saturating_sub(8));
            Ok(LoadCommand::Rpath { path })
        }

        LC_ENCRYPTION_INFO_64 => {
            let cryptoff  = ru32(&mut r, le)?;
            let cryptsize = ru32(&mut r, le)?;
            let cryptid   = ru32(&mut r, le)?;
            Ok(LoadCommand::EncryptionInfo64 { cryptoff, cryptsize, cryptid })
        }

        _ => Ok(LoadCommand::Unknown { cmd })
    }
}

// --- Helpers ---

fn ru32(r: &mut impl Read, le: bool) -> io::Result<u32> {
    if le { r.read_u32::<LittleEndian>() } else { r.read_u32::<BigEndian>() }
}

fn ru64(r: &mut impl Read, le: bool) -> io::Result<u64> {
    if le { r.read_u64::<LittleEndian>() } else { r.read_u64::<BigEndian>() }
}

fn ri32(r: &mut impl Read, le: bool) -> io::Result<i32> {
    if le { r.read_i32::<LittleEndian>() } else { r.read_i32::<BigEndian>() }
}

// Reads a null-terminated string from `buf` at byte `offset`.
// Offsets in Mach-O LC string unions are from the LC start (including cmd+cmdsize),
// so callers subtract 8 before passing here.
fn lc_str(buf: &[u8], offset: usize) -> String {
    if offset >= buf.len() {
        return String::new();
    }
    let end = buf[offset..].iter().position(|&b| b == 0).unwrap_or(buf.len() - offset);
    String::from_utf8_lossy(&buf[offset..offset + end]).into_owned()
}

fn prot_str(prot: i32) -> String {
    let r = if prot & 1 != 0 { 'r' } else { '-' };
    let w = if prot & 2 != 0 { 'w' } else { '-' };
    let x = if prot & 4 != 0 { 'x' } else { '-' };
    format!("{}{}{}", r, w, x)
}

// Mach-O packed version: major.minor.patch in top 16 / next 8 / bottom 8 bits
fn fmt_version(v: u32) -> String {
    format!("{}.{}.{}", v >> 16, (v >> 8) & 0xff, v & 0xff)
}

// Source version: A.B.C.D.E packed as 40/10/10/10/10 bits
fn fmt_source_version(v: u64) -> String {
    format!("{}.{}.{}.{}.{}",
        v >> 40,
        (v >> 30) & 0x3ff,
        (v >> 20) & 0x3ff,
        (v >> 10) & 0x3ff,
        v & 0x3ff,
    )
}

fn platform_name(p: u32) -> &'static str {
    match p {
        1  => "macOS",
        2  => "iOS",
        3  => "tvOS",
        4  => "watchOS",
        5  => "bridgeOS",
        6  => "macCatalyst",
        7  => "iOSSimulator",
        8  => "tvOSSimulator",
        9  => "watchOSSimulator",
        10 => "DriverKit",
        _  => "Unknown",
    }
}
