//! ELF loader for Linux binaries
//!
//! This module loads and parses Linux ELF binaries.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::errno::LinuxErrno;

/// Loaded ELF information
#[derive(Debug)]
pub struct LoadedElf {
    /// Entry point address
    pub entry_point: u64,
    /// Program headers
    pub program_headers: Vec<ProgramHeader>,
    /// Interpreter path (for dynamic executables)
    pub interpreter: Option<String>,
    /// Is PIE (position independent executable)
    pub is_pie: bool,
    /// Preferred load address
    pub load_addr: u64,
    /// Total memory size needed
    pub mem_size: u64,
    /// ELF data
    pub data: Vec<u8>,
}

/// Program header
#[derive(Debug, Clone)]
pub struct ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Offset in file
    pub p_offset: u64,
    /// Virtual address
    pub p_vaddr: u64,
    /// Physical address
    pub p_paddr: u64,
    /// Size in file
    pub p_filesz: u64,
    /// Size in memory
    pub p_memsz: u64,
    /// Alignment
    pub p_align: u64,
}

/// ELF program header types
pub mod pt_type {
    pub const PT_NULL: u32 = 0;
    pub const PT_LOAD: u32 = 1;
    pub const PT_DYNAMIC: u32 = 2;
    pub const PT_INTERP: u32 = 3;
    pub const PT_NOTE: u32 = 4;
    pub const PT_SHLIB: u32 = 5;
    pub const PT_PHDR: u32 = 6;
    pub const PT_TLS: u32 = 7;
    pub const PT_GNU_EH_FRAME: u32 = 0x6474e550;
    pub const PT_GNU_STACK: u32 = 0x6474e551;
    pub const PT_GNU_RELRO: u32 = 0x6474e552;
}

/// ELF program header flags
pub mod pf_flags {
    pub const PF_X: u32 = 1; // Execute
    pub const PF_W: u32 = 2; // Write
    pub const PF_R: u32 = 4; // Read
}

/// ELF header magic
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class (32/64 bit)
const ELFCLASS64: u8 = 2;

/// ELF data encoding (little/big endian)
const ELFDATA2LSB: u8 = 1;

/// ELF type
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;

/// ELF machine type
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;

/// Load an ELF file
pub fn load_elf(path: &str) -> Result<LoadedElf, LinuxErrno> {
    let mut file = File::open(path).map_err(|_| LinuxErrno::ENOENT)?;

    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|_| LinuxErrno::EIO)?;

    parse_elf(&data)
}

/// Load an ELF from memory
pub fn load_elf_from_memory(data: &[u8]) -> Result<LoadedElf, LinuxErrno> {
    parse_elf(data)
}

/// Parse ELF data
fn parse_elf(data: &[u8]) -> Result<LoadedElf, LinuxErrno> {
    if data.len() < 64 {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Check magic
    if &data[0..4] != ELF_MAGIC {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Check class (64-bit)
    if data[4] != ELFCLASS64 {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Check endianness (little endian)
    if data[5] != ELFDATA2LSB {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Parse ELF header
    let e_type = u16::from_le_bytes([data[16], data[17]]);
    let e_machine = u16::from_le_bytes([data[18], data[19]]);

    // Check machine type
    #[cfg(target_arch = "x86_64")]
    if e_machine != EM_X86_64 {
        return Err(LinuxErrno::ENOEXEC);
    }

    #[cfg(target_arch = "aarch64")]
    if e_machine != EM_AARCH64 {
        return Err(LinuxErrno::ENOEXEC);
    }

    let is_pie = e_type == ET_DYN;

    if e_type != ET_EXEC && e_type != ET_DYN {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Entry point
    let e_entry = u64::from_le_bytes([
        data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
    ]);

    // Program header offset
    let e_phoff = u64::from_le_bytes([
        data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
    ]);

    // Program header entry size
    let e_phentsize = u16::from_le_bytes([data[54], data[55]]);

    // Number of program headers
    let e_phnum = u16::from_le_bytes([data[56], data[57]]);

    // Parse program headers
    let mut program_headers = Vec::with_capacity(e_phnum as usize);
    let mut interpreter = None;
    let mut load_addr = u64::MAX;
    let mut mem_end = 0u64;

    for i in 0..e_phnum as usize {
        let offset = e_phoff as usize + i * e_phentsize as usize;

        if offset + 56 > data.len() {
            return Err(LinuxErrno::ENOEXEC);
        }

        let p_type = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        let p_flags = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        let p_offset = u64::from_le_bytes([
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15],
        ]);
        let p_vaddr = u64::from_le_bytes([
            data[offset + 16],
            data[offset + 17],
            data[offset + 18],
            data[offset + 19],
            data[offset + 20],
            data[offset + 21],
            data[offset + 22],
            data[offset + 23],
        ]);
        let p_paddr = u64::from_le_bytes([
            data[offset + 24],
            data[offset + 25],
            data[offset + 26],
            data[offset + 27],
            data[offset + 28],
            data[offset + 29],
            data[offset + 30],
            data[offset + 31],
        ]);
        let p_filesz = u64::from_le_bytes([
            data[offset + 32],
            data[offset + 33],
            data[offset + 34],
            data[offset + 35],
            data[offset + 36],
            data[offset + 37],
            data[offset + 38],
            data[offset + 39],
        ]);
        let p_memsz = u64::from_le_bytes([
            data[offset + 40],
            data[offset + 41],
            data[offset + 42],
            data[offset + 43],
            data[offset + 44],
            data[offset + 45],
            data[offset + 46],
            data[offset + 47],
        ]);
        let p_align = u64::from_le_bytes([
            data[offset + 48],
            data[offset + 49],
            data[offset + 50],
            data[offset + 51],
            data[offset + 52],
            data[offset + 53],
            data[offset + 54],
            data[offset + 55],
        ]);

        // Track load address and memory size
        if p_type == pt_type::PT_LOAD {
            if p_vaddr < load_addr {
                load_addr = p_vaddr;
            }
            let end = p_vaddr + p_memsz;
            if end > mem_end {
                mem_end = end;
            }
        }

        // Extract interpreter path
        if p_type == pt_type::PT_INTERP {
            let interp_start = p_offset as usize;
            let interp_end = (p_offset + p_filesz) as usize;
            if interp_end <= data.len() {
                if let Ok(path) = std::str::from_utf8(&data[interp_start..interp_end - 1]) {
                    interpreter = Some(path.to_string());
                }
            }
        }

        program_headers.push(ProgramHeader {
            p_type,
            p_flags,
            p_offset,
            p_vaddr,
            p_paddr,
            p_filesz,
            p_memsz,
            p_align,
        });
    }

    if load_addr == u64::MAX {
        load_addr = 0;
    }

    let mem_size = if mem_end > load_addr {
        mem_end - load_addr
    } else {
        0
    };

    Ok(LoadedElf {
        entry_point: e_entry,
        program_headers,
        interpreter,
        is_pie,
        load_addr,
        mem_size,
        data: data.to_vec(),
    })
}

/// Auxiliary vector entries for ELF loading
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum AuxvType {
    AtNull = 0,
    AtIgnore = 1,
    AtExecfd = 2,
    AtPhdr = 3,
    AtPhent = 4,
    AtPhnum = 5,
    AtPagesz = 6,
    AtBase = 7,
    AtFlags = 8,
    AtEntry = 9,
    AtNotelf = 10,
    AtUid = 11,
    AtEuid = 12,
    AtGid = 13,
    AtEgid = 14,
    AtPlatform = 15,
    AtHwcap = 16,
    AtClktck = 17,
    AtSecure = 23,
    AtBasePlatform = 24,
    AtRandom = 25,
    AtHwcap2 = 26,
    AtExecfn = 31,
    AtSysinfo = 32,
    AtSysinfoEhdr = 33,
}

/// Build auxiliary vector for process startup
pub fn build_auxv(
    elf: &LoadedElf,
    base_addr: u64,
    phdr_addr: u64,
    random_addr: u64,
) -> Vec<(u64, u64)> {
    let mut auxv = Vec::new();

    auxv.push((AuxvType::AtPhdr as u64, phdr_addr));
    auxv.push((AuxvType::AtPhent as u64, 56)); // sizeof(Elf64_Phdr)
    auxv.push((AuxvType::AtPhnum as u64, elf.program_headers.len() as u64));
    auxv.push((AuxvType::AtPagesz as u64, 4096));
    auxv.push((AuxvType::AtBase as u64, base_addr));
    auxv.push((AuxvType::AtFlags as u64, 0));
    auxv.push((AuxvType::AtEntry as u64, elf.entry_point));
    auxv.push((AuxvType::AtUid as u64, 1000));
    auxv.push((AuxvType::AtEuid as u64, 1000));
    auxv.push((AuxvType::AtGid as u64, 1000));
    auxv.push((AuxvType::AtEgid as u64, 1000));
    auxv.push((AuxvType::AtSecure as u64, 0));
    auxv.push((AuxvType::AtRandom as u64, random_addr));
    auxv.push((AuxvType::AtClktck as u64, 100)); // sysconf(_SC_CLK_TCK)
    auxv.push((AuxvType::AtNull as u64, 0));

    auxv
}

/// Validate an ELF file without fully loading it
pub fn validate_elf(path: &str) -> Result<(), LinuxErrno> {
    let mut file = File::open(path).map_err(|_| LinuxErrno::ENOENT)?;

    let mut header = [0u8; 64];
    file.read_exact(&mut header).map_err(|_| LinuxErrno::EIO)?;

    // Check magic
    if &header[0..4] != ELF_MAGIC {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Check class (64-bit)
    if header[4] != ELFCLASS64 {
        return Err(LinuxErrno::ENOEXEC);
    }

    // Check type
    let e_type = u16::from_le_bytes([header[16], header[17]]);
    if e_type != ET_EXEC && e_type != ET_DYN {
        return Err(LinuxErrno::ENOEXEC);
    }

    Ok(())
}
