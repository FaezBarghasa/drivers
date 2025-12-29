//! Shader Translation and Caching

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderLanguage {
    GLSL,
    HLSL,
    SPIRV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    RayGen,
    Miss,
    ClosestHit,
    AnyHit,
}

pub struct CompiledShader {
    pub spirv: Vec<u32>,
    pub hash: u64,
    pub stage: ShaderStage,
}

pub struct ShaderCache {
    cache_dir: PathBuf,
    memory_cache: Arc<Mutex<HashMap<u64, Arc<CompiledShader>>>>,
}

impl ShaderCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self, &'static str> {
        std::fs::create_dir_all(&cache_dir).ok();
        Ok(Self {
            cache_dir,
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn compile(
        &self,
        source: &str,
        lang: ShaderLanguage,
        stage: ShaderStage,
    ) -> Result<Arc<CompiledShader>, &'static str> {
        let hash = Self::hash_source(source);

        if let Some(cached) = self.memory_cache.lock().unwrap().get(&hash) {
            return Ok(cached.clone());
        }

        let spirv = vec![]; // Actual compilation here
        let shader = Arc::new(CompiledShader { spirv, hash, stage });
        self.memory_cache
            .lock()
            .unwrap()
            .insert(hash, shader.clone());
        Ok(shader)
    }

    fn hash_source(source: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        source.hash(&mut h);
        h.finish()
    }
}

pub fn init_shader_cache() -> Result<(), &'static str> {
    log::info!("Shader cache initialized");
    Ok(())
}
