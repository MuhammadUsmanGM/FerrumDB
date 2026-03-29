use std::path::Path;
use async_trait::async_trait;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt, Result as IoResult, Error as IoError, ErrorKind};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use rand::RngCore;

/// Abstraction for filesystem operations to support different backends (Disk, Memory, WASM).
#[async_trait]
pub trait AsyncFileSystem: Send + Sync {
    async fn open_append(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>>;
    async fn open_read(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>>;
    async fn open_write(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>>;
    /// Read exactly `len` bytes starting at byte `offset` from the file at `path`.
    async fn read_at(&self, path: &Path, offset: u64, len: usize) -> IoResult<Vec<u8>>;
    async fn create_dir_all(&self, path: &Path) -> IoResult<()>;
    async fn rename(&self, from: &Path, to: &Path) -> IoResult<()>;
    async fn exists(&self, path: &Path) -> bool;
    async fn remove_file(&self, path: &Path) -> IoResult<()>;
}

/// Abstraction for file-like objects.
#[async_trait]
pub trait AsyncFile: Send + Sync {
    async fn write_all(&mut self, buf: &[u8]) -> IoResult<()>;
    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IoResult<()>;
    async fn sync_data(&self) -> IoResult<()>;
    async fn sync_all(&self) -> IoResult<()>;
    async fn metadata_len(&self) -> IoResult<u64>;
}

/// Real disk implementation using tokio::fs.
pub struct DiskFileSystem;

#[async_trait]
impl AsyncFileSystem for DiskFileSystem {
    async fn open_append(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
            .await?;
        Ok(Box::new(DiskFile { file }))
    }

    async fn open_read(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = File::open(path).await?;
        Ok(Box::new(DiskFile { file }))
    }

    async fn open_write(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await?;
        Ok(Box::new(DiskFile { file }))
    }

    async fn read_at(&self, path: &Path, offset: u64, len: usize) -> IoResult<Vec<u8>> {
        let mut file = File::open(path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf).await?;
        Ok(buf)
    }

    async fn create_dir_all(&self, path: &Path) -> IoResult<()> {
        fs::create_dir_all(path).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> IoResult<()> {
        fs::rename(from, to).await
    }

    async fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    async fn remove_file(&self, path: &Path) -> IoResult<()> {
        fs::remove_file(path).await
    }
}

pub struct DiskFile {
    file: File,
}

#[async_trait]
impl AsyncFile for DiskFile {
    async fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.file.write_all(buf).await
    }

    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IoResult<()> {
        self.file.read_to_end(buf).await?;
        Ok(())
    }

    async fn sync_data(&self) -> IoResult<()> {
        self.file.sync_data().await
    }

    async fn sync_all(&self) -> IoResult<()> {
        self.file.sync_all().await
    }

    async fn metadata_len(&self) -> IoResult<u64> {
        Ok(self.file.metadata().await?.len())
    }
}

/// Encrypted implementation using AES-256-GCM.
pub struct EncryptedFileSystem {
    inner: Box<dyn AsyncFileSystem>,
    key: [u8; 32],
}

impl EncryptedFileSystem {
    pub fn new(inner: Box<dyn AsyncFileSystem>, key: [u8; 32]) -> Self {
        Self { inner, key }
    }
}

#[async_trait]
impl AsyncFileSystem for EncryptedFileSystem {
    async fn open_append(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = self.inner.open_append(path).await?;
        Ok(Box::new(EncryptedFile::new(file, self.key)))
    }

    async fn open_read(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = self.inner.open_read(path).await?;
        Ok(Box::new(EncryptedFile::new(file, self.key)))
    }

    async fn open_write(&self, path: &Path) -> IoResult<Box<dyn AsyncFile>> {
        let file = self.inner.open_write(path).await?;
        Ok(Box::new(EncryptedFile::new(file, self.key)))
    }

    async fn read_at(&self, path: &Path, offset: u64, len: usize) -> IoResult<Vec<u8>> {
        // For encrypted FS, offsets stored in the index are positions within the
        // decrypted byte stream (since recovery decrypts everything via read_to_end).
        // We decrypt the entire file and return the requested slice.
        let mut reader = self.open_read(path).await?;
        let mut decrypted = Vec::new();
        reader.read_to_end(&mut decrypted).await?;

        let start = offset as usize;
        let end = start + len;
        if end > decrypted.len() {
            return Err(IoError::new(ErrorKind::UnexpectedEof, "read_at: offset+len exceeds decrypted data"));
        }
        Ok(decrypted[start..end].to_vec())
    }

    async fn create_dir_all(&self, path: &Path) -> IoResult<()> {
        self.inner.create_dir_all(path).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> IoResult<()> {
        self.inner.rename(from, to).await
    }

    async fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path).await
    }

    async fn remove_file(&self, path: &Path) -> IoResult<()> {
        self.inner.remove_file(path).await
    }
}

pub struct EncryptedFile {
    inner: Box<dyn AsyncFile>,
    key: [u8; 32],
}

impl EncryptedFile {
    pub fn new(inner: Box<dyn AsyncFile>, key: [u8; 32]) -> Self {
        Self { inner, key }
    }
}

#[async_trait]
impl AsyncFile for EncryptedFile {
    async fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
        
        // Generate a cryptographically random nonce per write.
        // Reusing a nonce with the same key breaks AES-GCM security entirely.
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, buf)
            .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
        
        // Format: [nonce (12 bytes)] [ciphertext_len (8 bytes)] [ciphertext]
        let len = ciphertext.len() as u64;
        self.inner.write_all(&nonce_bytes).await?;
        self.inner.write_all(&len.to_le_bytes()).await?;
        self.inner.write_all(&ciphertext).await
    }

    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IoResult<()> {
        let mut raw_data = Vec::new();
        self.inner.read_to_end(&mut raw_data).await?;
        
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;

        // Format: [nonce (12 bytes)] [ciphertext_len (8 bytes)] [ciphertext]
        let mut cursor = 0;
        while cursor < raw_data.len() {
            // Read nonce (12 bytes)
            if cursor + 12 > raw_data.len() { break; }
            let nonce_bytes: [u8; 12] = raw_data[cursor..cursor+12].try_into().unwrap();
            let nonce = Nonce::from_slice(&nonce_bytes);
            cursor += 12;

            // Read ciphertext length (8 bytes)
            if cursor + 8 > raw_data.len() { break; }
            let len_bytes: [u8; 8] = raw_data[cursor..cursor+8].try_into().unwrap();
            let len = u64::from_le_bytes(len_bytes) as usize;
            cursor += 8;

            if cursor + len > raw_data.len() { break; }
            let ciphertext = &raw_data[cursor..cursor+len];
            let plaintext = cipher.decrypt(nonce, ciphertext)
                .map_err(|e| IoError::new(ErrorKind::Other, format!("Decryption failed: {}", e)))?;
            
            buf.extend_from_slice(&plaintext);
            cursor += len;
        }

        Ok(())
    }

    async fn sync_data(&self) -> IoResult<()> {
        self.inner.sync_data().await
    }

    async fn sync_all(&self) -> IoResult<()> {
        self.inner.sync_all().await
    }

    async fn metadata_len(&self) -> IoResult<u64> {
        self.inner.metadata_len().await
    }
}
