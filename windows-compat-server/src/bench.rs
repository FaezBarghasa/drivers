//! Runtime Performance Benchmarks for the WAC Server
//!
//! Measures the overhead of key WAC server operations:
//! - SxS manifest parsing
//! - Registry key path translation
//! - DLL search path construction
//! - Named pipe path translation
//!
//! These benchmarks use a simple wall-clock timer (no external bench harness
//! needed) so they can run as regular `#[test]` functions with `--nocapture`.
//!
//! # Running
//!
//! ```sh
//! cargo test --bin windows-compat-server bench -- --nocapture
//! ```

use std::time::{Duration, Instant};

/// Result of a single benchmark run
#[derive(Debug)]
pub struct BenchResult {
    pub name: &'static str,
    pub iterations: u64,
    pub total: Duration,
    pub mean_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
}

impl BenchResult {
    pub fn print(&self) {
        println!(
            "[BENCH] {}: {} iters | mean={} ns | min={} ns | max={} ns",
            self.name, self.iterations, self.mean_ns, self.min_ns, self.max_ns
        );
    }
}

/// Run a closure `iterations` times and collect timing statistics.
pub fn bench<F: FnMut()>(name: &'static str, iterations: u64, mut f: F) -> BenchResult {
    let mut times = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        times.push(start.elapsed().as_nanos() as u64);
    }
    let total_ns: u64 = times.iter().sum();
    let mean_ns = total_ns / iterations;
    let min_ns = *times.iter().min().unwrap_or(&0);
    let max_ns = *times.iter().max().unwrap_or(&0);
    BenchResult {
        name,
        iterations,
        total: Duration::from_nanos(total_ns),
        mean_ns,
        min_ns,
        max_ns,
    }
}

/// Performance target: manifest parsing should complete in under 1 ms per call
pub const MANIFEST_PARSE_TARGET_NS: u64 = 1_000_000;

/// Performance target: registry key translation should complete in under 1 µs
pub const REGISTRY_TRANSLATE_TARGET_NS: u64 = 1_000;

/// Performance target: named pipe translation should complete in under 500 ns
pub const PIPE_TRANSLATE_TARGET_NS: u64 = 500;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proton;
    use crate::sxs::Manifest;

    const SAMPLE_MANIFEST: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="X86"
    name="BenchmarkApp" type="win32"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0" processorArchitecture="X86"
        publicKeyToken="6595b64144ccf1df"/>
    </dependentAssembly>
  </dependency>
</assembly>"#;

    #[test]
    fn bench_manifest_parse() {
        let result = bench("manifest_parse", 1000, || {
            let _ = Manifest::parse(SAMPLE_MANIFEST).expect("parse failed");
        });
        result.print();
        assert!(
            result.mean_ns < MANIFEST_PARSE_TARGET_NS,
            "Manifest parsing too slow: {} ns mean (target: {} ns)",
            result.mean_ns,
            MANIFEST_PARSE_TARGET_NS
        );
    }

    #[test]
    fn bench_registry_key_translation() {
        let result = bench("registry_key_translation", 10_000, || {
            let _ = proton::windows_registry_key_to_scheme_path(
                "HKEY_CURRENT_USER\\Software\\Valve\\Steam\\ActiveProcess",
            );
        });
        result.print();
        assert!(
            result.mean_ns < REGISTRY_TRANSLATE_TARGET_NS,
            "Registry translation too slow: {} ns mean (target: {} ns)",
            result.mean_ns,
            REGISTRY_TRANSLATE_TARGET_NS
        );
    }

    #[test]
    fn bench_named_pipe_translation() {
        let result = bench("named_pipe_translation", 10_000, || {
            let _ = proton::translate_named_pipe(r"\\.\pipe\SteamClient");
        });
        result.print();
        assert!(
            result.mean_ns < PIPE_TRANSLATE_TARGET_NS,
            "Pipe translation too slow: {} ns mean (target: {} ns)",
            result.mean_ns,
            PIPE_TRANSLATE_TARGET_NS
        );
    }

    #[test]
    fn bench_winsxs_dir_name() {
        use crate::sxs::AssemblyIdentity;
        let id = AssemblyIdentity {
            name: "Microsoft.Windows.Common-Controls".to_string(),
            version: "6.0.0.0".to_string(),
            processor_architecture: "amd64".to_string(),
            public_key_token: "6595b64144ccf1df".to_string(),
            assembly_type: "win32".to_string(),
        };
        let result = bench("winsxs_dir_name", 10_000, || {
            let _ = id.winsxs_dir_name();
        });
        result.print();
        // No strict target — just ensure it completes and prints timing
        assert!(
            result.mean_ns < 100_000,
            "winsxs_dir_name too slow: {} ns",
            result.mean_ns
        );
    }

    #[test]
    fn bench_steam_dll_verification() {
        let result = bench("steam_dll_verification", 1_000, || {
            let _ = proton::verify_steam_dll_handling();
        });
        result.print();
        assert!(
            result.mean_ns < 100_000,
            "Steam DLL verification too slow: {} ns",
            result.mean_ns
        );
    }
}
