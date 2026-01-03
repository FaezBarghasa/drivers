//! Android Error Codes

/// Android error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidError {
    /// Success
    Ok = 0,
    /// Unknown error
    Unknown = -1,
    /// No memory
    NoMemory = -12,
    /// Invalid operation
    InvalidOperation = -38,
    /// Bad value
    BadValue = -22,
    /// Name not found
    NameNotFound = -2,
    /// Permission denied
    PermissionDenied = -1,
    /// No init
    NoInit = -19,
    /// Already exists
    AlreadyExists = -17,
    /// Dead object
    DeadObject = -32,
    /// Failed transaction
    FailedTransaction = -129,
    /// Binder: bad type
    BadType = -130,
    /// APK not found
    ApkNotFound = -200,
    /// Invalid APK format
    InvalidApk = -201,
    /// Package not found
    PackageNotFound = -202,
    /// Activity not found
    ActivityNotFound = -203,
    /// Service not found
    ServiceNotFound = -204,
    /// Intent not resolved
    IntentNotResolved = -205,
}

impl AndroidError {
    pub fn is_ok(&self) -> bool {
        *self == AndroidError::Ok
    }

    pub fn message(&self) -> &'static str {
        match self {
            AndroidError::Ok => "Success",
            AndroidError::Unknown => "Unknown error",
            AndroidError::NoMemory => "Out of memory",
            AndroidError::InvalidOperation => "Invalid operation",
            AndroidError::BadValue => "Bad value",
            AndroidError::NameNotFound => "Name not found",
            AndroidError::PermissionDenied => "Permission denied",
            AndroidError::NoInit => "Not initialized",
            AndroidError::AlreadyExists => "Already exists",
            AndroidError::DeadObject => "Dead object",
            AndroidError::FailedTransaction => "Transaction failed",
            AndroidError::BadType => "Bad type",
            AndroidError::ApkNotFound => "APK not found",
            AndroidError::InvalidApk => "Invalid APK format",
            AndroidError::PackageNotFound => "Package not found",
            AndroidError::ActivityNotFound => "Activity not found",
            AndroidError::ServiceNotFound => "Service not found",
            AndroidError::IntentNotResolved => "Intent could not be resolved",
        }
    }
}

impl std::fmt::Display for AndroidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for AndroidError {}
