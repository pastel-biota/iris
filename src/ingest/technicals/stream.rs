// Safety: The file names are entirely controlled at here
#![allow(clippy::disallowed_types)]

use std::{io::{ErrorKind, Write}, os::unix::fs::{MetadataExt, PermissionsExt}, path::PathBuf};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use tokio::{pin, sync::{Semaphore, SemaphorePermit}};

// Maximum 1G
pub const MAX_OVERALL_SIZE_BYTES: usize = 1024 * 1024 * 1024 * 1;

// Maximum 300M
pub const OPENED_OVERALL_SIZE_BYTES: usize = 1024 * 1024 * 300;

// Maximum 100 MB
pub const MAX_PER_STREAM_SIZE_BYTES: usize = 1024 * 1024 * 100;

pub struct SizedStream {
    io_sem: Semaphore,
    mem_sem: Semaphore,
}

impl SizedStream {
    pub fn new() -> Self {
        Self {
            io_sem: Semaphore::new(MAX_OVERALL_SIZE_BYTES),
            mem_sem: Semaphore::new(OPENED_OVERALL_SIZE_BYTES),
        }
    }

    pub async fn receive_stream<'s, S, E>(
        &'s self,
        requested_size: usize,
        stream: S
    ) -> Result<StreamPayloadHandle<'s>, std::io::Error>
    where E: std::error::Error + Send + Sync + 'static,
          S: Stream<Item = Result<Bytes, E>>,
    {
        if requested_size >= MAX_PER_STREAM_SIZE_BYTES {
            return Err(std::io::Error::new(ErrorKind::FileTooLarge, format!("The payload must be within {MAX_PER_STREAM_SIZE_BYTES} bytes")));
        }

        let _semaphore = self.io_sem.acquire_many(requested_size.try_into().unwrap());
        tracing::debug!(
            "Acquired I/O Sem: {}, now {} left",
            requested_size,
            self.io_sem.available_permits(),
        );

        pin!(stream);
        let payload = StreamPayloadHandle::new(&self.mem_sem)?;

        let mut file = std::fs::File::create_new(&payload.ephemeral_path)?;

        let mut size = 0usize;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

            let new_size = size + chunk.len();

            if new_size > requested_size {
                return Err(std::io::Error::new(ErrorKind::FileTooLarge, "The payload is exceeding the early reported length of the content"));
            }

            file.write_all(&chunk)?;
            size = new_size
        }

        file.flush()?;

        if size != requested_size {
            tracing::warn!("The received stream's size does not match to the requested size: req={} v.s. actual={}", requested_size, size)
        }

        Ok(payload)
    }
}

pub struct StreamPayloadHandle<'s> {
    ephemeral_path: PathBuf,
    semaphore: &'s Semaphore,
}

impl<'s> StreamPayloadHandle<'s> {
    pub fn new(semaphore: &'s Semaphore) -> Result<Self, std::io::Error> {
        let random = rand::random::<u32>();
        let path = Self::ensure_tmp_dir()?.join(format!("payload-{random}"));

        Ok(Self {
            ephemeral_path: path,
            semaphore,
        })
    }

    pub async fn read(self) -> Result<OpenedPayload<'s>, std::io::Error> {
        let meta = self.ephemeral_path.metadata()?;

        let size = meta.size().try_into().unwrap();
        let permit = self.semaphore.acquire_many(size).await.unwrap();
        tracing::debug!(
            "Acquired MEM Sem: {}, now {} left",
            size,
            self.semaphore.available_permits(),
        );

        Ok(OpenedPayload {
            bytes: std::fs::read(&self.ephemeral_path)?.into(),
            _permit: permit,
        })
    }

    pub fn ensure_tmp_dir() -> Result<PathBuf, std::io::Error> {
        let dir = std::env::temp_dir().join("iris").join("payload");

        if !dir.is_dir() {
            std::fs::create_dir_all(&dir)?;
        }

        let meta = std::fs::metadata(&dir)?;
        let mut permission = meta.permissions();

        #[cfg(unix)]
        permission.set_mode(0o700);

        Ok(dir)
    }
}

pub struct OpenedPayload<'s> {
    pub bytes: Bytes,
    _permit: SemaphorePermit<'s>
}

impl Drop for StreamPayloadHandle<'_> {
    fn drop(&mut self) {
        std::fs::remove_file(&self.ephemeral_path).ok();
    }
}

