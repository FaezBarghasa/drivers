use crate::context::{Context, ContextParams};
use crate::device::IntelDevice;
use crate::partition::PartitionTable;
use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use syscall::error::{Error, Result, EINVAL, ENOENT};
use syscall::schemev2::NewFdFlags;

pub struct IntelGalBackend {
    device: Arc<IntelDevice>,
    contexts: Mutex<BTreeMap<usize, Arc<Context>>>,
    next_id: Mutex<usize>,
    partition_table: Mutex<PartitionTable>,
}

impl IntelGalBackend {
    pub fn new(device: Arc<IntelDevice>) -> Self {
        let partition_table = PartitionTable::new(4 * 1024 * 1024 * 1024);
        Self {
            device,
            contexts: Mutex::new(BTreeMap::new()),
            next_id: Mutex::new(0),
            partition_table: Mutex::new(partition_table),
        }
    }

    pub fn register(&self) -> std::result::Result<(), &'static str> {
        log::info!("IntelGalBackend registered scheme: gpu");
        Ok(())
    }
}

impl SchemeSync for IntelGalBackend {
    fn open(&mut self, path: &str, _flags: usize, _ctx: &CallerCtx) -> Result<OpenResult> {
        let parts: Vec<&str> = path.split('/').collect();
        let mut core_mask = 0xFFFF_FFFF_FFFF_FFFF;

        if parts.len() >= 3 && parts[1] == "vcore" {
            if let Ok(mask) = u64::from_str_radix(parts[2], 16) {
                core_mask = mask;
            } else if let Ok(mask) = parts[2].parse::<u64>() {
                core_mask = mask;
            }
        }

        let mut vram_size = 0;
        if parts.len() >= 5 && parts[3] == "vram" {
            if let Ok(size_mb) = parts[4].parse::<u64>() {
                vram_size = size_mb * 1024 * 1024;
            }
        }

        let mut next_id = self.next_id.lock().map_err(|_| Error::new(EINVAL))?;
        let id = *next_id;
        *next_id += 1;

        let pt = self
            .partition_table
            .lock()
            .map_err(|_| Error::new(EINVAL))?;
        drop(pt);

        let params = ContextParams {
            core_mask,
            priority: 0,
        };

        // Attempt to allocate from partition table
        let mut pt = self
            .partition_table
            .lock()
            .map_err(|_| Error::new(EINVAL))?;

        let vram_start = 0; // In a full implementation, `pt` would allocate this

        let context = Arc::new(Context::new(id as u32, params, vram_start, vram_size));
        context.apply_mask(&self.device);

        let _ = pt.add_partition(crate::partition::GpuPartition {
            id,
            vram_start,
            vram_size,
            core_mask,
        });

        self.contexts
            .lock()
            .map_err(|_| Error::new(EINVAL))?
            .insert(id, context);

        Ok(OpenResult::ThisScheme {
            number: id,
            flags: NewFdFlags::empty(),
        })
    }

    fn write(
        &mut self,
        id: usize,
        buf: &[u8],
        _offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let contexts = self.contexts.lock().map_err(|_| Error::new(EINVAL))?;
        let context = contexts.get(&id).ok_or(Error::new(ENOENT))?;

        let batch_start = 0; // Parse these from buf for a real implementation
        let batch_len = 0;

        if let Err(_) = context.validate_submission(batch_start, batch_len) {
            return Err(Error::new(EINVAL));
        }

        crate::execbuf::submit(
            context,
            &self.device,
            &crate::execbuf::ExecBuffer {
                batch_start_offset: batch_start,
                batch_len,
            },
        )
        .map_err(|_| Error::new(EINVAL))?;

        Ok(buf.len())
    }

    fn on_close(&mut self, id: usize) {
        if let Ok(mut contexts) = self.contexts.lock() {
            contexts.remove(&id);
        }
        if let Ok(pt) = self.partition_table.lock() {
            pt.remove_partition(id);
        }
    }
}
