use crate::context::{Context, ContextParams};
use crate::device::IntelDevice;
use redox_scheme::{Result, Scheme, SchemeMut};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use syscall::error::{Error, EINVAL, ENOENT};

pub struct IntelGalBackend {
    device: Arc<IntelDevice>,
    contexts: Mutex<BTreeMap<usize, Arc<Context>>>,
    next_id: Mutex<usize>,
}

impl IntelGalBackend {
    pub fn new(device: Arc<IntelDevice>) -> Self {
        Self {
            device,
            contexts: Mutex::new(BTreeMap::new()),
            next_id: Mutex::new(0),
        }
    }

    pub fn register(&self) -> std::result::Result<(), &'static str> {
        // In a real driver, we would register the "gpu" scheme here via syscalls
        // socket.register("gpu")
        log::info!("IntelGalBackend registered scheme: gpu");
        Ok(())
    }
}

impl SchemeMut for IntelGalBackend {
    fn open(&mut self, path: &str, _flags: usize, _uid: u32, _gid: u32) -> Result<usize> {
        // Parse URL: gpu://0/vcore/MASK
        // path comes in as "0/vcore/1" (scheme: is stripped)

        let parts: Vec<&str> = path.split('/').collect();
        let mut core_mask = 0xFFFF_FFFF_FFFF_FFFF;

        if parts.len() >= 3 && parts[1] == "vcore" {
            // Parse mask
            if let Ok(mask) = u64::from_str_radix(parts[2], 16) {
                core_mask = mask;
            } else if let Ok(mask) = parts[2].parse::<u64>() {
                core_mask = mask;
            }
        }

        let mut next_id = self.next_id.lock().map_err(|_| Error::new(EINVAL))?;
        let id = *next_id;
        *next_id += 1;

        let params = ContextParams {
            core_mask,
            priority: 0,
        };

        let context = Arc::new(Context::new(id as u32, params));

        self.contexts
            .lock()
            .map_err(|_| Error::new(EINVAL))?
            .insert(id, context);

        Ok(id)
    }

    fn close(&mut self, id: usize) -> Result<usize> {
        self.contexts
            .lock()
            .map_err(|_| Error::new(EINVAL))?
            .remove(&id)
            .ok_or(Error::new(ENOENT))?;
        Ok(0)
    }

    fn write(&mut self, id: usize, _buf: &[u8]) -> Result<usize> {
        // Placeholder for command submission via write
        // In reality, use ioctl or specific packet format
        let contexts = self.contexts.lock().map_err(|_| Error::new(EINVAL))?;
        let context = contexts.get(&id).ok_or(Error::new(ENOENT))?;

        // Simulate submission
        crate::execbuf::submit(
            context,
            &crate::execbuf::ExecBuffer {
                batch_start_offset: 0,
                batch_len: 0,
            },
        )
        .map_err(|_| Error::new(EINVAL))?;

        Ok(_buf.len())
    }
}
