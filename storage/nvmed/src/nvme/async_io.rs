use common::dma::Dma;
use std::sync::Arc;
use syscall::Result;

use super::cmd::NvmeCmd;
use super::controller::Nvme;
use super::executor::NvmeExecutor;
use super::namespace::NvmeNamespace; // LocalExecutor<NvmeHw>

pub async fn read(nvme: &Nvme, ns: &NvmeNamespace, lba: u64, buf: &mut [u8]) -> Result<usize> {
    let len = buf.len();
    if len == 0 {
        return Ok(0);
    }

    // Allocate DMA buffer
    let mut dma = unsafe {
        Dma::zeroed_slice(len)
            .map_err(|_| syscall::Error::new(syscall::ENOMEM))?
            .assume_init()
    };

    // Calculate PRPs
    let ptr = dma.physical();
    let blocks = (len.div_ceil(ns.block_size as usize)) as u16;

    // Create command
    // TODO: Handle PRP list if > 2 pages. For now assume small transfers or contiguous (Dma is contiguous).
    // Dma from common is virtually contiguous and physically contiguous usually?
    // Redox common::dma uses `physmap`. It is physically contiguous.
    // So one PRP is enough? Or two if it crosses page boundary?
    // Actually Dma.physical() returns the start phys addr. Can we assume it is contiguous?
    // Usually yes for `Dma` allocation in drivers.

    let cmd = NvmeCmd::io_read(
        1, // CID (will be replaced by executor)
        ns.id,
        lba,
        blocks.saturating_sub(1), // 0-based count
        ptr as u64,
        0, // PRP2
    );

    // Submit
    // We need to match the SQ ID. Default IO SQ is 1?
    let sq_id = 1;

    let cqe = NvmeExecutor::current().submit(sq_id, cmd).await;

    if (cqe.status >> 1) == 0 {
        // Success
        buf.copy_from_slice(&dma);
        Ok(len)
    } else {
        Err(syscall::Error::new(syscall::EIO))
    }
}

pub async fn write(nvme: &Nvme, ns: &NvmeNamespace, lba: u64, buf: &[u8]) -> Result<usize> {
    let len = buf.len();
    if len == 0 {
        return Ok(0);
    }

    let mut dma = unsafe {
        Dma::zeroed_slice(len)
            .map_err(|_| syscall::Error::new(syscall::ENOMEM))?
            .assume_init()
    };
    dma.copy_from_slice(buf);

    let ptr = dma.physical();
    let blocks = (len.div_ceil(ns.block_size as usize)) as u16;

    let cmd = NvmeCmd::io_write(1, ns.id, lba, blocks.saturating_sub(1), ptr as u64, 0);

    let sq_id = 1;
    let cqe = NvmeExecutor::current().submit(sq_id, cmd).await;

    if (cqe.status >> 1) == 0 {
        Ok(len)
    } else {
        Err(syscall::Error::new(syscall::EIO))
    }
}

pub async fn read_phys(
    nvme: &Nvme,
    ns: &NvmeNamespace,
    lba: u64,
    phys_addr: usize,
    size: usize,
) -> Result<usize> {
    if size == 0 {
        return Ok(0);
    }

    let blocks = (size.div_ceil(ns.block_size as usize)) as u16;

    let cmd = NvmeCmd::io_read(1, ns.id, lba, blocks.saturating_sub(1), phys_addr as u64, 0);

    let sq_id = 1;
    let cqe = NvmeExecutor::current().submit(sq_id, cmd).await;

    if (cqe.status >> 1) == 0 {
        Ok(size)
    } else {
        Err(syscall::Error::new(syscall::EIO))
    }
}

pub async fn write_phys(
    nvme: &Nvme,
    ns: &NvmeNamespace,
    lba: u64,
    phys_addr: usize,
    size: usize,
) -> Result<usize> {
    if size == 0 {
        return Ok(0);
    }

    let blocks = (size.div_ceil(ns.block_size as usize)) as u16;

    let cmd = NvmeCmd::io_write(1, ns.id, lba, blocks.saturating_sub(1), phys_addr as u64, 0);

    let sq_id = 1;
    let cqe = NvmeExecutor::current().submit(sq_id, cmd).await;

    if (cqe.status >> 1) == 0 {
        Ok(size)
    } else {
        Err(syscall::Error::new(syscall::EIO))
    }
}
