//! NT Status Codes
//!
//! Windows NT status codes mapped to error handling.

/// NT Status code type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NtStatus {
    Success = 0x00000000,

    // Information
    Pending = 0x00000103,

    // Warning
    BufferOverflow = 0x80000005,
    NoMoreEntries = 0x8000001A,

    // Error - common
    Unsuccessful = 0xC0000001,
    NotImplemented = 0xC0000002,
    InvalidInfoClass = 0xC0000003,
    InfoLengthMismatch = 0xC0000004,
    AccessViolation = 0xC0000005,
    InvalidHandle = 0xC0000008,
    InvalidParameter = 0xC000000D,
    NoSuchFile = 0xC000000F,
    EndOfFile = 0xC0000011,
    MoreProcessingRequired = 0xC0000016,
    AccessDenied = 0xC0000022,
    BufferTooSmall = 0xC0000023,
    ObjectTypeMismatch = 0xC0000024,
    ObjectNameInvalid = 0xC0000033,
    ObjectNameNotFound = 0xC0000034,
    ObjectNameCollision = 0xC0000035,
    ObjectPathInvalid = 0xC0000039,
    ObjectPathNotFound = 0xC000003A,
    ObjectPathSyntaxBad = 0xC000003B,

    // Process/Thread errors
    ProcessIsTerminating = 0xC000010A,
    ThreadNotInProcess = 0xC000010B,

    // Memory errors
    NoMemory = 0xC0000017,
    ConflictingAddresses = 0xC0000018,
    UnableToFreeVM = 0xC000001A,
    UnableToDeleteSection = 0xC000001B,
    InvalidSystemService = 0xC000001C,
    CommitmentLimit = 0xC000012D,

    // File errors
    FileInvalid = 0xC0000098,
    FileLockConflict = 0xC0000054,

    // Image errors
    InvalidImageFormat = 0xC000007B,
    ImageMachineTypeMismatch = 0xC000007C,
}

impl NtStatus {
    /// Check if status indicates success
    pub fn is_success(&self) -> bool {
        (*self as u32) < 0x80000000
    }

    /// Check if status is an information status
    pub fn is_info(&self) -> bool {
        (*self as u32) >> 30 == 0
    }

    /// Check if status is a warning
    pub fn is_warning(&self) -> bool {
        (*self as u32) >> 30 == 1
    }

    /// Check if status is an error
    pub fn is_error(&self) -> bool {
        (*self as u32) >> 30 >= 2
    }

    /// Convert from raw u32
    pub fn from_raw(code: u32) -> Self {
        // Note: This only handles known codes
        match code {
            0x00000000 => NtStatus::Success,
            0x00000103 => NtStatus::Pending,
            0xC0000001 => NtStatus::Unsuccessful,
            0xC0000002 => NtStatus::NotImplemented,
            0xC0000005 => NtStatus::AccessViolation,
            0xC0000008 => NtStatus::InvalidHandle,
            0xC000000D => NtStatus::InvalidParameter,
            0xC000000F => NtStatus::NoSuchFile,
            0xC0000017 => NtStatus::NoMemory,
            0xC0000022 => NtStatus::AccessDenied,
            0xC0000034 => NtStatus::ObjectNameNotFound,
            0xC000003A => NtStatus::ObjectPathNotFound,
            0xC000007B => NtStatus::InvalidImageFormat,
            _ => NtStatus::Unsuccessful,
        }
    }
}

impl From<std::io::Error> for NtStatus {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => NtStatus::NoSuchFile,
            ErrorKind::PermissionDenied => NtStatus::AccessDenied,
            ErrorKind::AlreadyExists => NtStatus::ObjectNameCollision,
            ErrorKind::InvalidInput => NtStatus::InvalidParameter,
            ErrorKind::OutOfMemory => NtStatus::NoMemory,
            ErrorKind::UnexpectedEof => NtStatus::EndOfFile,
            _ => NtStatus::Unsuccessful,
        }
    }
}
