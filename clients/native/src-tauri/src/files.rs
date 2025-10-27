use crate::error::Error;
use aes_gcm::{Aes256Gcm, Key};
use anyhow::anyhow;
use crypto::CryptoFileDesc;
use dtos::file::FileMetadata;
use strum::{EnumString, IntoStaticStr};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, IntoStaticStr, EnumString, Debug)]
pub enum FileStatus {
    New,
    UploadInProgress,
    Synced,
}

pub struct Metadata {
    pub uuid: Uuid,
    pub date: OffsetDateTime,
    pub sha256: String,
    pub key: String,
}

pub enum File {
    Local {
        path: String,
        metadata: Metadata,
        status: FileStatus,
    },
    Remote {
        metadata: Metadata,
    },
}

#[derive(Clone, Debug)]
pub struct FileDescriptor {
    pub path: String,
    pub uuid: Uuid,
    pub date: OffsetDateTime,
    pub sha256: String,
    pub key: String,
    pub status: FileStatus,
}

impl TryFrom<File> for FileDescriptor {
    type Error = Error;

    fn try_from(value: File) -> Result<Self, Self::Error> {
        match value {
            File::Local {
                path,
                metadata,
                status,
            } => Ok(FileDescriptor {
                path,
                uuid: metadata.uuid,
                date: metadata.date,
                sha256: metadata.sha256,
                key: metadata.key,
                status,
            }),
            File::Remote { .. } => Err(Error::Runtime(anyhow!(
                "Cannot convert remote file to file descriptor"
            ))),
        }
    }
}

impl From<&FileDescriptor> for FileMetadata {
    fn from(desc: &FileDescriptor) -> Self {
        Self {
            path: desc.path.to_owned(),
            uuid: desc.uuid,
            date: desc.date,
            sha256: desc.sha256.to_owned(),
            key: desc.key.to_owned(),
        }
    }
}

impl CryptoFileDesc for FileDescriptor {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }
}

pub struct FileDescriptorWithDecodedKey(FileDescriptor, Key<Aes256Gcm>);

impl FileDescriptorWithDecodedKey {
    pub fn new(desc: FileDescriptor, key: Key<Aes256Gcm>) -> Self {
        Self(desc, key)
    }

    pub fn descriptor(&self) -> &FileDescriptor {
        &self.0
    }

    pub fn key(&self) -> &Key<Aes256Gcm> {
        &self.1
    }
}
