//! NT Syscall Table
//!
//! Windows NT syscall numbers for x86_64 (ntdll.dll).
//! These vary by Windows version; this is based on Windows 10/11.

/// NT syscall numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum NtSyscall {
    // Process/Thread
    NtTerminateProcess = 0x002C,
    NtTerminateThread = 0x0053,
    NtCreateProcess = 0x004D,
    NtCreateProcessEx = 0x004F,
    NtOpenProcess = 0x0026,
    NtCreateThread = 0x004E,
    NtCreateThreadEx = 0x00C1,
    NtOpenThread = 0x012B,
    NtQueryInformationProcess = 0x0019,
    NtSetInformationProcess = 0x001C,
    NtQueryInformationThread = 0x0025,
    NtSetInformationThread = 0x000D,
    NtSuspendThread = 0x01BD,
    NtResumeThread = 0x0052,
    NtGetContextThread = 0x00EB,
    NtSetContextThread = 0x018B,

    // Memory
    NtAllocateVirtualMemory = 0x0018,
    NtFreeVirtualMemory = 0x001E,
    NtProtectVirtualMemory = 0x0050,
    NtQueryVirtualMemory = 0x0023,
    NtReadVirtualMemory = 0x003F,
    NtWriteVirtualMemory = 0x003A,
    NtFlushVirtualMemory = 0x011E,

    // Section (memory mapped files)
    NtCreateSection = 0x004A,
    NtOpenSection = 0x0037,
    NtMapViewOfSection = 0x0028,
    NtUnmapViewOfSection = 0x002A,
    NtExtendSection = 0x0114,

    // File I/O
    NtCreateFile = 0x0055,
    NtOpenFile = 0x0033,
    NtClose = 0x000F,
    NtReadFile = 0x0006,
    NtWriteFile = 0x0008,
    NtFlushBuffersFile = 0x0077,
    NtQueryInformationFile = 0x0011,
    NtSetInformationFile = 0x0027,
    NtQueryDirectoryFile = 0x0035,
    NtQueryVolumeInformationFile = 0x0073,
    NtSetVolumeInformationFile = 0x01AF,
    NtDeleteFile = 0x010D,
    NtDeviceIoControlFile = 0x0007,
    NtFsControlFile = 0x0039,
    NtLockFile = 0x0127,
    NtUnlockFile = 0x01D4,

    // Registry
    NtCreateKey = 0x001D,
    NtOpenKey = 0x0012,
    NtOpenKeyEx = 0x0130,
    NtDeleteKey = 0x0107,
    NtQueryKey = 0x0016,
    NtSetValueKey = 0x0096,
    NtQueryValueKey = 0x0017,
    NtDeleteValueKey = 0x010B,
    NtEnumerateKey = 0x0032,
    NtEnumerateValueKey = 0x0013,
    NtFlushKey = 0x011C,
    NtSaveKey = 0x016D,
    NtRestoreKey = 0x0162,
    NtLoadKey = 0x0125,
    NtUnloadKey = 0x01D2,

    // Events
    NtCreateEvent = 0x0048,
    NtOpenEvent = 0x0040,
    NtSetEvent = 0x000E,
    NtResetEvent = 0x0161,
    NtClearEvent = 0x003D,
    NtPulseEvent = 0x0151,

    // Mutexes
    NtCreateMutant = 0x0085,
    NtOpenMutant = 0x012E,
    NtReleaseMutant = 0x001F,

    // Semaphores
    NtCreateSemaphore = 0x0088,
    NtOpenSemaphore = 0x0133,
    NtReleaseSemaphore = 0x0010,

    // Wait operations
    NtWaitForSingleObject = 0x0004,
    NtWaitForMultipleObjects = 0x005B,
    NtSignalAndWaitForSingleObject = 0x01A2,

    // Time
    NtQuerySystemTime = 0x005A,
    NtSetSystemTime = 0x01A6,
    NtQueryPerformanceCounter = 0x0031,
    NtDelayExecution = 0x0034,

    // System Information
    NtQuerySystemInformation = 0x0036,
    NtSetSystemInformation = 0x01A4,

    // Token/Security
    NtOpenProcessToken = 0x0131,
    NtOpenProcessTokenEx = 0x0132,
    NtOpenThreadToken = 0x0024,
    NtOpenThreadTokenEx = 0x002E,
    NtQueryInformationToken = 0x0021,
    NtSetInformationToken = 0x018F,
    NtAdjustPrivilegesToken = 0x0041,
    NtDuplicateToken = 0x0042,

    // Misc
    NtDuplicateObject = 0x003C,
    NtQueryObject = 0x0010,
    NtSetSecurityObject = 0x018C,
    NtQuerySecurityObject = 0x014F,

    // Unknown/Invalid
    Invalid = 0xFFFFFFFF,
}

impl NtSyscall {
    /// Convert from syscall number
    pub fn from_number(num: u32) -> Self {
        match num {
            0x002C => Self::NtTerminateProcess,
            0x0053 => Self::NtTerminateThread,
            0x0018 => Self::NtAllocateVirtualMemory,
            0x001E => Self::NtFreeVirtualMemory,
            0x0050 => Self::NtProtectVirtualMemory,
            0x0055 => Self::NtCreateFile,
            0x0033 => Self::NtOpenFile,
            0x000F => Self::NtClose,
            0x0006 => Self::NtReadFile,
            0x0008 => Self::NtWriteFile,
            0x0011 => Self::NtQueryInformationFile,
            0x0027 => Self::NtSetInformationFile,
            0x0035 => Self::NtQueryDirectoryFile,
            0x001D => Self::NtCreateKey,
            0x0012 => Self::NtOpenKey,
            0x0017 => Self::NtQueryValueKey,
            0x0096 => Self::NtSetValueKey,
            0x0048 => Self::NtCreateEvent,
            0x000E => Self::NtSetEvent,
            0x0004 => Self::NtWaitForSingleObject,
            0x005B => Self::NtWaitForMultipleObjects,
            0x0034 => Self::NtDelayExecution,
            0x0036 => Self::NtQuerySystemInformation,
            _ => Self::Invalid,
        }
    }

    /// Get the syscall number
    pub fn number(&self) -> u32 {
        *self as u32
    }

    /// Get syscall name
    pub fn name(&self) -> &'static str {
        match self {
            Self::NtTerminateProcess => "NtTerminateProcess",
            Self::NtTerminateThread => "NtTerminateThread",
            Self::NtCreateProcess => "NtCreateProcess",
            Self::NtCreateProcessEx => "NtCreateProcessEx",
            Self::NtOpenProcess => "NtOpenProcess",
            Self::NtAllocateVirtualMemory => "NtAllocateVirtualMemory",
            Self::NtFreeVirtualMemory => "NtFreeVirtualMemory",
            Self::NtProtectVirtualMemory => "NtProtectVirtualMemory",
            Self::NtQueryVirtualMemory => "NtQueryVirtualMemory",
            Self::NtCreateFile => "NtCreateFile",
            Self::NtOpenFile => "NtOpenFile",
            Self::NtClose => "NtClose",
            Self::NtReadFile => "NtReadFile",
            Self::NtWriteFile => "NtWriteFile",
            Self::NtQueryInformationFile => "NtQueryInformationFile",
            Self::NtSetInformationFile => "NtSetInformationFile",
            Self::NtQueryDirectoryFile => "NtQueryDirectoryFile",
            Self::NtCreateKey => "NtCreateKey",
            Self::NtOpenKey => "NtOpenKey",
            Self::NtQueryValueKey => "NtQueryValueKey",
            Self::NtSetValueKey => "NtSetValueKey",
            Self::NtCreateEvent => "NtCreateEvent",
            Self::NtSetEvent => "NtSetEvent",
            Self::NtWaitForSingleObject => "NtWaitForSingleObject",
            Self::NtWaitForMultipleObjects => "NtWaitForMultipleObjects",
            Self::NtDelayExecution => "NtDelayExecution",
            Self::NtQuerySystemInformation => "NtQuerySystemInformation",
            _ => "Unknown",
        }
    }
}
