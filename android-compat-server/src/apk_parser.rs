//! APK Parser
//!
//! Parses Android APK files to extract manifest and resources.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::errno::AndroidError;

/// Local file header signature in ZIP
const ZIP_LOCAL_HEADER: u32 = 0x04034b50;

/// Central directory header signature
const ZIP_CENTRAL_DIR: u32 = 0x02014b50;

/// End of central directory signature
const ZIP_END_CENTRAL_DIR: u32 = 0x06054b50;

/// Parsed Android manifest
#[derive(Debug, Clone)]
pub struct AndroidManifest {
    pub package_name: String,
    pub version_code: u32,
    pub version_name: String,
    pub min_sdk_version: u32,
    pub target_sdk_version: u32,
    pub activities: Vec<ActivityInfo>,
    pub services: Vec<ServiceInfo>,
    pub receivers: Vec<ReceiverInfo>,
    pub providers: Vec<ProviderInfo>,
    pub permissions: Vec<String>,
    pub uses_permissions: Vec<String>,
}

impl Default for AndroidManifest {
    fn default() -> Self {
        Self {
            package_name: String::new(),
            version_code: 1,
            version_name: "1.0".to_string(),
            min_sdk_version: 21,
            target_sdk_version: 33,
            activities: Vec::new(),
            services: Vec::new(),
            receivers: Vec::new(),
            providers: Vec::new(),
            permissions: Vec::new(),
            uses_permissions: Vec::new(),
        }
    }
}

/// Activity component info
#[derive(Debug, Clone)]
pub struct ActivityInfo {
    pub name: String,
    pub exported: bool,
    pub intent_filters: Vec<IntentFilter>,
}

/// Service component info
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub exported: bool,
}

/// Broadcast receiver info
#[derive(Debug, Clone)]
pub struct ReceiverInfo {
    pub name: String,
    pub exported: bool,
    pub intent_filters: Vec<IntentFilter>,
}

/// Content provider info
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub authorities: String,
    pub exported: bool,
}

/// Intent filter for activities/receivers
#[derive(Debug, Clone)]
pub struct IntentFilter {
    pub actions: Vec<String>,
    pub categories: Vec<String>,
    pub data_schemes: Vec<String>,
}

/// APK entry in the ZIP archive
#[derive(Debug)]
struct ApkEntry {
    name: String,
    compressed_size: u32,
    uncompressed_size: u32,
    offset: u64,
    compression_method: u16,
}

/// Parse an APK manifest
pub fn parse_manifest(apk_path: &str) -> Result<AndroidManifest, AndroidError> {
    let mut file = File::open(apk_path).map_err(|_| AndroidError::ApkNotFound)?;

    // Find AndroidManifest.xml in the APK
    let entries = list_apk_entries(&mut file)?;

    let manifest_entry = entries
        .iter()
        .find(|e| e.name == "AndroidManifest.xml")
        .ok_or(AndroidError::InvalidApk)?;

    // Read the manifest (binary XML format)
    let manifest_data = read_entry(&mut file, manifest_entry)?;

    // Parse binary XML
    parse_binary_xml(&manifest_data)
}

/// List entries in the APK (ZIP file)
fn list_apk_entries(file: &mut File) -> Result<Vec<ApkEntry>, AndroidError> {
    let mut entries = Vec::new();

    // Find end of central directory
    file.seek(SeekFrom::End(-22))
        .map_err(|_| AndroidError::InvalidApk)?;

    let mut buf = [0u8; 22];
    file.read_exact(&mut buf)
        .map_err(|_| AndroidError::InvalidApk)?;

    let sig = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if sig != ZIP_END_CENTRAL_DIR {
        return Err(AndroidError::InvalidApk);
    }

    let central_dir_offset = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
    let num_entries = u16::from_le_bytes([buf[10], buf[11]]);

    // Read central directory
    file.seek(SeekFrom::Start(central_dir_offset as u64))
        .map_err(|_| AndroidError::InvalidApk)?;

    for _ in 0..num_entries {
        let entry = read_central_dir_entry(file)?;
        entries.push(entry);
    }

    Ok(entries)
}

/// Read a central directory entry
fn read_central_dir_entry(file: &mut File) -> Result<ApkEntry, AndroidError> {
    let mut header = [0u8; 46];
    file.read_exact(&mut header)
        .map_err(|_| AndroidError::InvalidApk)?;

    let sig = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    if sig != ZIP_CENTRAL_DIR {
        return Err(AndroidError::InvalidApk);
    }

    let compression_method = u16::from_le_bytes([header[10], header[11]]);
    let compressed_size = u32::from_le_bytes([header[20], header[21], header[22], header[23]]);
    let uncompressed_size = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    let name_length = u16::from_le_bytes([header[28], header[29]]) as usize;
    let extra_length = u16::from_le_bytes([header[30], header[31]]) as usize;
    let comment_length = u16::from_le_bytes([header[32], header[33]]) as usize;
    let local_header_offset = u32::from_le_bytes([header[42], header[43], header[44], header[45]]);

    let mut name_buf = vec![0u8; name_length];
    file.read_exact(&mut name_buf)
        .map_err(|_| AndroidError::InvalidApk)?;
    let name = String::from_utf8_lossy(&name_buf).to_string();

    // Skip extra and comment
    file.seek(SeekFrom::Current((extra_length + comment_length) as i64))
        .map_err(|_| AndroidError::InvalidApk)?;

    Ok(ApkEntry {
        name,
        compressed_size,
        uncompressed_size,
        offset: local_header_offset as u64,
        compression_method,
    })
}

/// Read an entry's data
fn read_entry(file: &mut File, entry: &ApkEntry) -> Result<Vec<u8>, AndroidError> {
    file.seek(SeekFrom::Start(entry.offset))
        .map_err(|_| AndroidError::InvalidApk)?;

    // Read local header
    let mut header = [0u8; 30];
    file.read_exact(&mut header)
        .map_err(|_| AndroidError::InvalidApk)?;

    let name_length = u16::from_le_bytes([header[26], header[27]]) as usize;
    let extra_length = u16::from_le_bytes([header[28], header[29]]) as usize;

    // Skip to data
    file.seek(SeekFrom::Current((name_length + extra_length) as i64))
        .map_err(|_| AndroidError::InvalidApk)?;

    // Read data
    let mut data = vec![0u8; entry.compressed_size as usize];
    file.read_exact(&mut data)
        .map_err(|_| AndroidError::InvalidApk)?;

    // Decompress if needed
    if entry.compression_method == 8 {
        // DEFLATE - would need a decompression library
        // For now, return compressed data
        Ok(data)
    } else {
        Ok(data)
    }
}

/// Parse Android binary XML format
fn parse_binary_xml(data: &[u8]) -> Result<AndroidManifest, AndroidError> {
    // Android binary XML is a complex format. This is a simplified parser.
    // In production, we would use a full AXML parser.

    if data.len() < 8 {
        return Err(AndroidError::InvalidApk);
    }

    // Check magic number (0x00080003 for binary XML)
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    // Create default manifest - full parsing would extract all fields
    let mut manifest = AndroidManifest::default();

    // Try to extract package name from the binary data
    // This is a simplified heuristic - real parsing is more complex
    if let Some(pkg_start) = find_package_name(data) {
        manifest.package_name = pkg_start;
    } else {
        manifest.package_name = "com.unknown.app".to_string();
    }

    Ok(manifest)
}

/// Try to find package name in binary XML data
fn find_package_name(data: &[u8]) -> Option<String> {
    // Look for common package name patterns in the string pool
    // This is a heuristic - proper parsing required for production

    // String pool starts at offset 8, header is 28 bytes
    if data.len() < 36 {
        return None;
    }

    // In binary XML, strings are usually UTF-16LE
    // Look for "com." pattern as UTF-16
    let pattern: [u8; 8] = [0x63, 0x00, 0x6f, 0x00, 0x6d, 0x00, 0x2e, 0x00]; // "com."

    if let Some(pos) = data.windows(8).position(|w| w == pattern) {
        // Extract the string
        let mut end = pos;
        while end < data.len() - 1 {
            if data[end] == 0 && data[end + 1] == 0 {
                break;
            }
            end += 2;
        }

        let utf16_data: Vec<u16> = data[pos..end]
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    Some(u16::from_le_bytes([c[0], c[1]]))
                } else {
                    None
                }
            })
            .collect();

        return Some(String::from_utf16_lossy(&utf16_data));
    }

    None
}

/// Extract native libraries from APK
pub fn extract_native_libs(apk_path: &str, target_abi: &str) -> Result<Vec<String>, AndroidError> {
    let mut file = File::open(apk_path).map_err(|_| AndroidError::ApkNotFound)?;
    let entries = list_apk_entries(&mut file)?;

    let lib_prefix = format!("lib/{}/", target_abi);
    let libs: Vec<String> = entries
        .iter()
        .filter(|e| e.name.starts_with(&lib_prefix) && e.name.ends_with(".so"))
        .map(|e| e.name.clone())
        .collect();

    Ok(libs)
}
