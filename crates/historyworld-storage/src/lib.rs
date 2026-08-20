//! Narrow content-addressed storage boundary.
//!
//! The filesystem implementation is for local/single-node operation. An S3
//! adapter can implement the same digest contract; semantic identities and
//! manifests do not change when storage topology changes.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use historyworld_domain::ContentHash;
use thiserror::Error;

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoredObject {
    pub hash: ContentHash,
    pub bytes: u64,
    pub already_present: bool,
}

/// Minimal synchronous port used by offline compilers and local tooling.
pub trait ObjectStore {
    fn put(&self, bytes: &[u8]) -> Result<StoredObject, StorageError>;
    fn get(&self, hash: ContentHash, max_bytes: u64) -> Result<Vec<u8>, StorageError>;
    fn verify(&self, hash: ContentHash) -> Result<u64, StorageError>;
}

/// Filesystem CAS layout: `objects/blake3/ab/cd/<64-char digest>`.
#[derive(Clone, Debug)]
pub struct FilesystemObjectStore {
    root: PathBuf,
    max_object_bytes: u64,
    durable_writes: bool,
}

impl FilesystemObjectStore {
    pub fn open(root: impl Into<PathBuf>, max_object_bytes: u64) -> Result<Self, StorageError> {
        if max_object_bytes == 0 {
            return Err(StorageError::InvalidObjectLimit);
        }
        let root = root.into();
        let object_root = root.join("objects/blake3");
        fs::create_dir_all(&object_root)?;
        let store = Self {
            root,
            max_object_bytes,
            durable_writes: true,
        };
        Self::sync_directory(&object_root)?;
        if let Some(objects) = object_root.parent() {
            Self::sync_directory(objects)?;
        }
        Self::sync_directory(&store.root)?;
        if let Some(parent) = store
            .root
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            Self::sync_directory(parent)?;
        }
        Ok(store)
    }

    /// Disables `sync_data` for disposable caches. Archival writes default on.
    pub const fn with_durable_writes(mut self, durable_writes: bool) -> Self {
        self.durable_writes = durable_writes;
        self
    }

    pub fn object_path(&self, hash: ContentHash) -> PathBuf {
        let digest = hash.to_string();
        self.root
            .join("objects")
            .join("blake3")
            .join(&digest[0..2])
            .join(&digest[2..4])
            .join(digest)
    }

    fn check_size(&self, actual: u64, request_limit: u64) -> Result<(), StorageError> {
        let limit = request_limit.min(self.max_object_bytes);
        if actual > limit {
            return Err(StorageError::ObjectTooLarge { actual, limit });
        }
        Ok(())
    }

    fn open_regular(path: &Path) -> Result<(File, u64), StorageError> {
        let before = fs::symlink_metadata(path)?;
        if !before.file_type().is_file() || before.file_type().is_symlink() {
            return Err(StorageError::NonRegularObject);
        }
        let file = File::open(path)?;
        let after = file.metadata()?;
        if !after.file_type().is_file() {
            return Err(StorageError::NonRegularObject);
        }
        #[cfg(unix)]
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err(StorageError::ObjectChangedDuringOpen);
        }
        Ok((file, after.len()))
    }

    fn create_temporary(final_path: &Path) -> Result<(PathBuf, File), StorageError> {
        for _ in 0..16 {
            let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let boot_nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos());
            let temporary_path = final_path.with_extension(format!(
                "tmp-{}-{boot_nonce}-{sequence}",
                std::process::id()
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
            {
                Ok(file) => return Ok((temporary_path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(StorageError::TemporaryNameExhausted)
    }

    fn ensure_shard_directories(&self, final_path: &Path) -> Result<(), StorageError> {
        let leaf = final_path.parent().ok_or(StorageError::InvalidObjectPath)?;
        let shard = leaf.parent().ok_or(StorageError::InvalidObjectPath)?;
        for directory in [shard, leaf] {
            match fs::create_dir(directory) {
                Ok(()) => {
                    if self.durable_writes {
                        Self::sync_directory(directory)?;
                        if let Some(parent) = directory.parent() {
                            Self::sync_directory(parent)?;
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn sync_directory(path: &Path) -> Result<(), StorageError> {
        File::open(path)?.sync_all()?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn sync_directory(_path: &Path) -> Result<(), StorageError> {
        Ok(())
    }
}

impl ObjectStore for FilesystemObjectStore {
    fn put(&self, bytes: &[u8]) -> Result<StoredObject, StorageError> {
        let byte_count = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        self.check_size(byte_count, self.max_object_bytes)?;
        let hash = ContentHash::from_bytes(*blake3::hash(bytes).as_bytes());
        let final_path = self.object_path(hash);
        match fs::symlink_metadata(&final_path) {
            Ok(_) => {
                let existing_bytes = self.verify(hash)?;
                return Ok(StoredObject {
                    hash,
                    bytes: existing_bytes,
                    already_present: true,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        self.ensure_shard_directories(&final_path)?;
        let parent = final_path.parent().ok_or(StorageError::InvalidObjectPath)?;
        let (temporary_path, mut temporary_file) = Self::create_temporary(&final_path)?;
        let write_result = (|| -> Result<bool, StorageError> {
            temporary_file.write_all(bytes)?;
            if self.durable_writes {
                temporary_file.sync_data()?;
            }
            drop(temporary_file);
            match fs::hard_link(&temporary_path, &final_path) {
                Ok(()) => {
                    if self.durable_writes {
                        Self::sync_directory(parent)?;
                    }
                    Ok(true)
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    self.verify(hash).map(|_| false)
                }
                Err(error) => Err(error.into()),
            }
        })();
        let _ = fs::remove_file(&temporary_path);
        let link_created = write_result?;

        Ok(StoredObject {
            hash,
            bytes: byte_count,
            already_present: !link_created,
        })
    }

    fn get(&self, hash: ContentHash, max_bytes: u64) -> Result<Vec<u8>, StorageError> {
        let path = self.object_path(hash);
        let (file, size) = Self::open_regular(&path)?;
        self.check_size(size, max_bytes)?;
        let capacity = usize::try_from(size).map_err(|_| StorageError::AddressSpaceExceeded)?;
        let mut bytes = Vec::with_capacity(capacity);
        let limit = max_bytes.min(self.max_object_bytes);
        file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
        let actual_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        self.check_size(actual_size, max_bytes)?;
        let actual_hash = ContentHash::from_bytes(*blake3::hash(&bytes).as_bytes());
        if actual_hash != hash {
            return Err(StorageError::DigestMismatch {
                expected: hash,
                actual: actual_hash,
            });
        }
        Ok(bytes)
    }

    fn verify(&self, hash: ContentHash) -> Result<u64, StorageError> {
        let path = self.object_path(hash);
        let (file, size) = Self::open_regular(&path)?;
        self.check_size(size, self.max_object_bytes)?;
        let file = file.take(self.max_object_bytes.saturating_add(1));
        let mut reader = BufReader::with_capacity(64 * 1024, file);
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut hasher = blake3::Hasher::new();
        let mut actual_size = 0_u64;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            actual_size = actual_size.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        }
        self.check_size(actual_size, self.max_object_bytes)?;
        let actual = ContentHash::from_bytes(*hasher.finalize().as_bytes());
        if actual != hash {
            return Err(StorageError::DigestMismatch {
                expected: hash,
                actual,
            });
        }
        Ok(actual_size)
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("maximum object size must be non-zero")]
    InvalidObjectLimit,
    #[error("object is {actual} bytes, above the {limit}-byte budget")]
    ObjectTooLarge { actual: u64, limit: u64 },
    #[error("object cannot be addressed in this process")]
    AddressSpaceExceeded,
    #[error("content digest mismatch: expected {expected}, calculated {actual}")]
    DigestMismatch {
        expected: ContentHash,
        actual: ContentHash,
    },
    #[error("content-addressed object path had no parent")]
    InvalidObjectPath,
    #[error("content-addressed path is not a regular file")]
    NonRegularObject,
    #[error("content-addressed object changed while it was being opened")]
    ObjectChangedDuringOpen,
    #[error("could not allocate a unique temporary object name")]
    TemporaryNameExhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "historyworld-storage-{name}-{}-{}",
            std::process::id(),
            TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn writes_deduplicate_and_verify_by_digest() {
        let root = temporary_root("deduplicate");
        let store = FilesystemObjectStore::open(&root, 1_024)
            .unwrap()
            .with_durable_writes(false);
        let first = store.put(b"immutable history").unwrap();
        let second = store.put(b"immutable history").unwrap();
        assert!(!first.already_present);
        assert!(second.already_present);
        assert_eq!(first.hash, second.hash);
        assert_eq!(store.get(first.hash, 1_024).unwrap(), b"immutable history");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_corruption_and_enforces_read_budget() {
        let root = temporary_root("corruption");
        let store = FilesystemObjectStore::open(&root, 1_024)
            .unwrap()
            .with_durable_writes(false);
        let object = store.put(b"correct bytes").unwrap();
        assert!(matches!(
            store.get(object.hash, 1),
            Err(StorageError::ObjectTooLarge { .. })
        ));
        fs::write(store.object_path(object.hash), b"corrupt bytes").unwrap();
        assert!(matches!(
            store.verify(object.hash),
            Err(StorageError::DigestMismatch { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_objects_in_the_content_addressed_namespace() {
        use std::os::unix::fs::symlink;

        let root = temporary_root("symlink");
        let store = FilesystemObjectStore::open(&root, 1_024)
            .unwrap()
            .with_durable_writes(false);
        let bytes = b"external bytes";
        let hash = ContentHash::from_bytes(*blake3::hash(bytes).as_bytes());
        let external = root.join("external");
        fs::write(&external, bytes).unwrap();
        let object_path = store.object_path(hash);
        fs::create_dir_all(object_path.parent().unwrap()).unwrap();
        symlink(external, &object_path).unwrap();
        assert!(matches!(
            store.get(hash, 1_024),
            Err(StorageError::NonRegularObject)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
