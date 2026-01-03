//! Path Translator
//!
//! Handles path translation between foreign and native formats.

use std::path::{Path, PathBuf};

/// Path translation utilities
pub struct PathTranslator {
    case_insensitive: bool,
}

impl PathTranslator {
    pub fn new(case_insensitive: bool) -> Self {
        Self { case_insensitive }
    }

    /// Normalize a Windows path
    pub fn normalize_windows(&self, path: &str) -> String {
        let mut normalized = path.replace('/', "\\");

        // Ensure drive letter is uppercase
        if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
            let mut chars: Vec<char> = normalized.chars().collect();
            chars[0] = chars[0].to_ascii_uppercase();
            normalized = chars.into_iter().collect();
        }

        // Remove trailing backslash (except for root)
        if normalized.len() > 3 && normalized.ends_with('\\') {
            normalized.pop();
        }

        normalized
    }

    /// Normalize a Unix path
    pub fn normalize_unix(&self, path: &str) -> String {
        let mut normalized = path.replace('\\', "/");

        // Remove double slashes
        while normalized.contains("//") {
            normalized = normalized.replace("//", "/");
        }

        // Remove trailing slash (except for root)
        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }

        normalized
    }

    /// Check if two paths match (respecting case sensitivity)
    pub fn paths_match(&self, a: &str, b: &str) -> bool {
        if self.case_insensitive {
            a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    }

    /// Get the parent path
    pub fn parent(&self, path: &str) -> Option<String> {
        let path = Path::new(path);
        path.parent().map(|p| p.to_string_lossy().to_string())
    }

    /// Get the file name
    pub fn filename(&self, path: &str) -> Option<String> {
        let path = Path::new(path);
        path.file_name().map(|n| n.to_string_lossy().to_string())
    }

    /// Join paths
    pub fn join(&self, base: &str, relative: &str) -> String {
        let base_path = PathBuf::from(base);
        base_path.join(relative).to_string_lossy().to_string()
    }

    /// Check if path is absolute
    pub fn is_absolute(&self, path: &str) -> bool {
        // Unix absolute
        if path.starts_with('/') {
            return true;
        }

        // Windows absolute (drive letter)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            return true;
        }

        // Windows UNC path
        if path.starts_with("\\\\") {
            return true;
        }

        false
    }

    /// Convert Windows path separators to Unix
    pub fn to_unix_separators(&self, path: &str) -> String {
        path.replace('\\', "/")
    }

    /// Convert Unix path separators to Windows
    pub fn to_windows_separators(&self, path: &str) -> String {
        path.replace('/', "\\")
    }
}

impl Default for PathTranslator {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_windows() {
        let translator = PathTranslator::new(false);
        assert_eq!(
            translator.normalize_windows("c:\\users\\test"),
            "C:\\users\\test"
        );
        assert_eq!(
            translator.normalize_windows("c:/users/test"),
            "C:\\users\\test"
        );
    }

    #[test]
    fn test_normalize_unix() {
        let translator = PathTranslator::new(false);
        assert_eq!(translator.normalize_unix("/home//user/"), "/home/user");
    }

    #[test]
    fn test_is_absolute() {
        let translator = PathTranslator::new(false);
        assert!(translator.is_absolute("/home/user"));
        assert!(translator.is_absolute("C:\\Windows"));
        assert!(translator.is_absolute("\\\\server\\share"));
        assert!(!translator.is_absolute("relative/path"));
    }
}
