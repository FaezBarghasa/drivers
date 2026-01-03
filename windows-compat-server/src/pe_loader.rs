//! PE/COFF Binary Loader
//!
//! Parses and loads Windows Portable Executable files for execution on Redox.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::NtStatus;

/// DOS header magic number
const DOS_MAGIC: u16 = 0x5A4D; // "MZ"

/// PE signature
const PE_SIGNATURE: u32 = 0x00004550; // "PE\0\0"

/// PE machine types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Machine {
    Unknown = 0x0,
    I386 = 0x14c,
    Amd64 = 0x8664,
    Arm = 0x1c0,
    Arm64 = 0xaa64,
}

impl From<u16> for Machine {
    fn from(value: u16) -> Self {
        match value {
            0x14c => Machine::I386,
            0x8664 => Machine::Amd64,
            0x1c0 => Machine::Arm,
            0xaa64 => Machine::Arm64,
            _ => Machine::Unknown,
        }
    }
}

/// Section characteristics
#[derive(Debug, Clone, Copy)]
pub struct SectionFlags(u32);

impl SectionFlags {
    pub const CODE: u32 = 0x00000020;
    pub const INITIALIZED_DATA: u32 = 0x00000040;
    pub const UNINITIALIZED_DATA: u32 = 0x00000080;
    pub const EXECUTE: u32 = 0x20000000;
    pub const READ: u32 = 0x40000000;
    pub const WRITE: u32 = 0x80000000;

    pub fn is_executable(&self) -> bool {
        self.0 & Self::EXECUTE != 0
    }

    pub fn is_readable(&self) -> bool {
        self.0 & Self::READ != 0
    }

    pub fn is_writable(&self) -> bool {
        self.0 & Self::WRITE != 0
    }

    pub fn is_code(&self) -> bool {
        self.0 & Self::CODE != 0
    }
}

/// PE section header
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub raw_data_size: u32,
    pub raw_data_ptr: u32,
    pub characteristics: SectionFlags,
}

/// Import directory entry
#[derive(Debug, Clone)]
pub struct Import {
    pub dll_name: String,
    pub functions: Vec<ImportFunction>,
}

/// Imported function
#[derive(Debug, Clone)]
pub struct ImportFunction {
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    pub address_rva: u32,
}

/// Loaded PE information
#[derive(Debug)]
pub struct PeInfo {
    pub machine: Machine,
    pub subsystem: u16,
    pub image_base: usize,
    pub entry_point: usize,
    pub size_of_image: usize,
    pub sections: Vec<Section>,
    pub imports: Vec<Import>,
    pub is_64bit: bool,
}

/// PE loader
pub struct PeLoader {
    /// Root directory for Windows path mapping
    windows_root: String,
}

impl PeLoader {
    pub fn new(windows_root: String) -> Self {
        Self { windows_root }
    }

    /// Convert a Windows path to Redox path
    fn map_path(&self, path: &str) -> String {
        // C:\Windows\System32 -> /windows/Windows/System32
        // Remove drive letter and colon
        let path = path.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ':');
        // Replace backslashes
        let path = path.replace('\\', "/");
        format!("{}{}", self.windows_root, path)
    }

    /// Load a PE file
    pub fn load(&self, path: &str) -> Result<PeInfo, NtStatus> {
        let redox_path = self.map_path(path);
        let mut file = File::open(&redox_path).map_err(|_| NtStatus::ObjectNameNotFound)?;

        // Read and validate DOS header
        let dos_magic = self.read_u16(&mut file)?;
        if dos_magic != DOS_MAGIC {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Get PE header offset from DOS header
        file.seek(SeekFrom::Start(0x3C))
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        let pe_offset = self.read_u32(&mut file)?;

        // Read PE signature
        file.seek(SeekFrom::Start(pe_offset as u64))
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        let pe_sig = self.read_u32(&mut file)?;
        if pe_sig != PE_SIGNATURE {
            return Err(NtStatus::InvalidImageFormat);
        }

        // Read COFF header
        let machine = Machine::from(self.read_u16(&mut file)?);
        let num_sections = self.read_u16(&mut file)?;
        let _time_date_stamp = self.read_u32(&mut file)?;
        let _symbol_table_ptr = self.read_u32(&mut file)?;
        let _num_symbols = self.read_u32(&mut file)?;
        let opt_header_size = self.read_u16(&mut file)?;
        let _characteristics = self.read_u16(&mut file)?;

        // Check if 32-bit or 64-bit
        let opt_magic = self.read_u16(&mut file)?;
        let is_64bit = opt_magic == 0x20b; // PE32+ = 64-bit

        // Skip to entry point
        let _major_linker = self.read_u8(&mut file)?;
        let _minor_linker = self.read_u8(&mut file)?;
        let _size_of_code = self.read_u32(&mut file)?;
        let _size_of_init = self.read_u32(&mut file)?;
        let _size_of_uninit = self.read_u32(&mut file)?;
        let entry_point_rva = self.read_u32(&mut file)?;
        let _base_of_code = self.read_u32(&mut file)?;

        // Image base
        let image_base = if is_64bit {
            file.seek(SeekFrom::Current(4))
                .map_err(|_| NtStatus::InvalidImageFormat)?;
            self.read_u64(&mut file)? as usize
        } else {
            let _base_of_data = self.read_u32(&mut file)?;
            self.read_u32(&mut file)? as usize
        };

        let _section_alignment = self.read_u32(&mut file)?;
        let _file_alignment = self.read_u32(&mut file)?;

        // Skip to subsystem
        file.seek(SeekFrom::Current(8))
            .map_err(|_| NtStatus::InvalidImageFormat)?;

        let _major_image_ver = self.read_u16(&mut file)?;
        let _minor_image_ver = self.read_u16(&mut file)?;
        let _major_subsys_ver = self.read_u16(&mut file)?;
        let _minor_subsys_ver = self.read_u16(&mut file)?;
        let _win32_ver_val = self.read_u32(&mut file)?;
        let size_of_image = self.read_u32(&mut file)?;
        let _size_of_headers = self.read_u32(&mut file)?;
        let _checksum = self.read_u32(&mut file)?;
        let subsystem = self.read_u16(&mut file)?;

        // Skip to end of optional header and read sections
        let opt_header_end = pe_offset as u64 + 24 + opt_header_size as u64;
        file.seek(SeekFrom::Start(opt_header_end))
            .map_err(|_| NtStatus::InvalidImageFormat)?;

        let mut sections = Vec::new();
        for _ in 0..num_sections {
            let section = self.read_section(&mut file)?;
            sections.push(section);
        }

        Ok(PeInfo {
            machine,
            subsystem,
            image_base,
            entry_point: image_base + entry_point_rva as usize,
            size_of_image: size_of_image as usize,
            sections,
            imports: Vec::new(), // TODO: Parse import directory
            is_64bit,
        })
    }

    fn read_section(&self, file: &mut File) -> Result<Section, NtStatus> {
        let mut name_bytes = [0u8; 8];
        file.read_exact(&mut name_bytes)
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .to_string();

        let virtual_size = self.read_u32(file)?;
        let virtual_address = self.read_u32(file)?;
        let raw_data_size = self.read_u32(file)?;
        let raw_data_ptr = self.read_u32(file)?;
        let _relocs_ptr = self.read_u32(file)?;
        let _linenums_ptr = self.read_u32(file)?;
        let _num_relocs = self.read_u16(file)?;
        let _num_linenums = self.read_u16(file)?;
        let characteristics = SectionFlags(self.read_u32(file)?);

        Ok(Section {
            name,
            virtual_size,
            virtual_address,
            raw_data_size,
            raw_data_ptr,
            characteristics,
        })
    }

    fn read_u8(&self, file: &mut File) -> Result<u8, NtStatus> {
        let mut buf = [0u8; 1];
        file.read_exact(&mut buf)
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        Ok(buf[0])
    }

    fn read_u16(&self, file: &mut File) -> Result<u16, NtStatus> {
        let mut buf = [0u8; 2];
        file.read_exact(&mut buf)
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&self, file: &mut File) -> Result<u32, NtStatus> {
        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&self, file: &mut File) -> Result<u64, NtStatus> {
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf)
            .map_err(|_| NtStatus::InvalidImageFormat)?;
        Ok(u64::from_le_bytes(buf))
    }
}
