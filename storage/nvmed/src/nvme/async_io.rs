//! Async I/O Interface

use super::queue::CompletionQueueEntry;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Async I/O operation
pub struct AsyncIo {
    command_id: u16,
    completion: Arc<Mutex<Option<CompletionQueueEntry>>>,
}

impl Future for AsyncIo {
    type Output = Result<CompletionQueueEntry, &'static str>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut completion = self.completion.lock().unwrap();

        if let Some(entry) = completion.take() {
            Poll::Ready(Ok(entry))
        } else {
            Poll::Pending
        }
    }
}

impl AsyncIo {
    /// Create new async I/O operation
    pub fn new(command_id: u16) -> Self {
        Self {
            command_id,
            completion: Arc::new(Mutex::new(None)),
        }
    }

    /// Complete the operation
    pub fn complete(&self, entry: CompletionQueueEntry) {
        *self.completion.lock().unwrap() = Some(entry);
    }
}

/// Async read operation
pub async fn read_async(
    lba: u64,
    num_blocks: u16,
    buffer: &mut [u8],
) -> Result<usize, &'static str> {
    log::debug!("Async read: LBA={}, blocks={}", lba, num_blocks);

    // Submit read command
    // Wait for completion

    Ok(num_blocks as usize * 512)
}

/// Async write operation
pub async fn write_async(lba: u64, num_blocks: u16, buffer: &[u8]) -> Result<usize, &'static str> {
    log::debug!("Async write: LBA={}, blocks={}", lba, num_blocks);

    // Submit write command
    // Wait for completion

    Ok(num_blocks as usize * 512)
}
