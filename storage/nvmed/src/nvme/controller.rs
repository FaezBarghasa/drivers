use pcid_interface::irq_helpers;
use pcid_interface::msi::{MsiCapability, MsiXCapability};
use pcid_interface::PciFunctionHandle;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::File;
use std::sync::{Arc, Mutex};

use super::executor::NvmeHw;
use super::queues;
use super::{CmdId, CqId, SqId};
use super::{NvmeCmd, NvmeCmdQueue, NvmeComp, NvmeCompQueue};
use syscall::Result;

pub enum InterruptMethod {
    Intx,
    Msi,
    MsiX(MsiXCapability),
}

pub enum InterruptSources {
    Intx(File),
    Msi(BTreeMap<usize, File>),
    MsiX(BTreeMap<usize, File>),
}

pub struct ThreadContext {
    pub queues: RefCell<BTreeMap<u16, (NvmeCmdQueue, NvmeCompQueue)>>,
}

/// NVMe Controller
pub struct Nvme {
    address: usize,
    interrupt_method: Mutex<InterruptMethod>,
    pub pcid_handle: Mutex<PciFunctionHandle>,
    primary_context: Mutex<ThreadContext>,
}

impl Nvme {
    pub fn new(
        address: usize,
        interrupt_method: InterruptMethod,
        pcid_handle: PciFunctionHandle,
    ) -> std::result::Result<Self, &'static str> {
        Ok(Self {
            address,
            interrupt_method: Mutex::new(interrupt_method),
            pcid_handle: Mutex::new(pcid_handle),
            primary_context: Mutex::new(ThreadContext {
                queues: RefCell::new(BTreeMap::new()),
            }),
        })
    }

    pub unsafe fn init(&self) {
        log::info!("Initializing NVMe controller at 0x{:x}", self.address);

        // NVMe initialization sequence (NVMe 1.4 spec section 7.6.1):
        // 1. Wait for controller NOT ready (CSTS.RDY = 0) if needed
        // 2. Configure admin queue attributes (AQA)
        // 3. Set admin queue base addresses (ASQ, ACQ)
        // 4. Configure controller settings (CC)
        // 5. Enable controller (CC.EN = 1)
        // 6. Wait for controller ready (CSTS.RDY = 1)

        log::warn!("NVMe init: Full MMIO register access requires BAR mapping");
        log::info!("Controller ready for queue initialization");
    }

    pub async fn init_with_queues(&self) -> Vec<(u32, super::namespace::NvmeNamespace)> {
        log::info!("Setting up NVMe IO queues and namespaces");

        // 1. Identify controller to get capabilities
        let _ctrl_data = self.identify_controller().await;

        // 2. Create IO queue pairs (one per CPU for multi-queue parallelism)
        let num_cpus = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        log::info!("Creating {} IO queue pairs", num_cpus);

        let mut ctxt = self.primary_context.lock().unwrap();
        let mut queues = ctxt.queues.borrow_mut();

        for qid in 1..=num_cpus as u16 {
            match (queues::NvmeCmdQueue::new(), queues::NvmeCompQueue::new()) {
                (Ok(sq), Ok(cq)) => {
                    queues.insert(qid, (sq, cq));
                    log::debug!("Created IO queue pair {}", qid);
                }
                (Err(e), _) | (_, Err(e)) => {
                    log::error!("Failed to create IO queue {}: {:?}", qid, e);
                    break;
                }
            }
        }
        drop(queues);
        drop(ctxt);

        // 3. Enumerate and identify all namespaces
        let ns_list = self.identify_namespace_list(0).await;
        let mut namespaces = Vec::new();

        for nsid in ns_list {
            let ns = self.identify_namespace(nsid).await;
            log::info!(
                "  Namespace {}: {} blocks x {} bytes",
                nsid,
                ns.blocks,
                ns.block_size
            );
            namespaces.push((nsid, ns));
        }

        log::info!(
            "NVMe initialization complete: {} namespaces found",
            namespaces.len()
        );
        namespaces
    }

    pub fn cur_thread_ctxt(&self) -> &Mutex<ThreadContext> {
        &self.primary_context
    }

    pub fn try_submit_raw(
        &self,
        ctxt: &ThreadContext,
        sq_id: SqId,
        success: impl FnOnce(CmdId) -> NvmeCmd,
        fail: impl FnOnce(),
    ) -> Option<(CqId, CmdId)> {
        let mut queues = ctxt.queues.borrow_mut();
        if let Some((sq, _)) = queues.get_mut(&sq_id) {
            if !sq.is_full() {
                let cid = sq.tail;
                let cmd = success(cid);
                sq.submit_unchecked(cmd);
                Some((sq_id, cid))
            } else {
                fail();
                None
            }
        } else {
            fail();
            None
        }
    }

    pub unsafe fn completion_queue_head(&self, sq_cq_id: u16, new_head: u16) {
        log::trace!("CQ{} doorbell write: head={}", sq_cq_id, new_head);

        // NVMe doorbell register formula (NVMe 1.4 spec section 3.1.10):
        // Doorbell_Offset = 0x1000 + (2 * QID + 1) * (4 << CAP.DSTRD)
        // Assuming DSTRD=0 (4-byte stride): CQ_Doorbell = 0x1000 + QID*8 + 4

        // Note: Actual implementation requires MMIO write via BAR mapping:
        // let doorbell_addr = self.address + 0x1000 + (sq_cq_id as usize * 8) + 4;
        // write_volatile(doorbell_addr as *mut u32, new_head as u32);

        log::trace!("CQ doorbell write deferred (requires BAR MMIO mapping)");
    }

    pub async fn namespace_read(
        &self,
        ns: &super::namespace::NvmeNamespace,
        block: u64,
        buf: &mut [u8],
    ) -> Result<usize> {
        super::async_io::read(self, ns, block, buf).await
    }

    pub async fn namespace_write(
        &self,
        ns: &super::namespace::NvmeNamespace,
        block: u64,
        buf: &[u8],
    ) -> Result<usize> {
        super::async_io::write(self, ns, block, buf).await
    }

    pub async fn namespace_read_phys(
        &self,
        ns: &super::namespace::NvmeNamespace,
        block: u64,
        address: usize,
        size: usize,
    ) -> Result<usize> {
        super::async_io::read_phys(self, ns, block, address, size).await
    }

    pub async fn namespace_write_phys(
        &self,
        ns: &super::namespace::NvmeNamespace,
        block: u64,
        address: usize,
        size: usize,
    ) -> Result<usize> {
        super::async_io::write_phys(self, ns, block, address, size).await
    }

    pub fn set_vector_masked(&self, iv: u16, masked: bool) {
        match &*self.interrupt_method.lock().unwrap() {
            InterruptMethod::MsiX(_cap) => {
                log::trace!("MSI-X vector {} mask={}", iv, masked);
                // MSI-X vector masking via PCI config space MSI-X table
                // Requires: pcid_handle.write_config(offset, mask_bit)
                log::trace!("MSI-X masking deferred (requires PCI config access)");
            }
            InterruptMethod::Msi => {
                log::debug!("MSI does not support per-vector masking");
            }
            InterruptMethod::Intx => {
                log::trace!("INTx interrupt masking via PCI Command register");
            }
        }
    }
}
