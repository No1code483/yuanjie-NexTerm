use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;

const CONTENT_DIR: &str = "editor_content";
const SESSIONS_DIR: &str = "editor_sessions";
const CONTENT_FILE: &str = "content.enc";
const VERSIONS_SUBDIR: &str = "versions";
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

pub struct EditorStorageService {
    base_dir: PathBuf,
}

impl EditorStorageService {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    pub fn init_dirs(&self) -> Result<(), AppError> {
        let content_dir = self.base_dir.join(CONTENT_DIR);
        let sessions_dir = self.base_dir.join(SESSIONS_DIR);
        fs::create_dir_all(&content_dir).map_err(AppError::FileSystem)?;
        fs::create_dir_all(&sessions_dir).map_err(AppError::FileSystem)?;
        Ok(())
    }

    pub fn doc_dir(&self, doc_uuid: &str) -> PathBuf {
        self.base_dir.join(CONTENT_DIR).join(doc_uuid)
    }

    pub fn content_path(&self, doc_uuid: &str) -> PathBuf {
        self.doc_dir(doc_uuid).join(CONTENT_FILE)
    }

    pub fn versions_dir(&self, doc_uuid: &str) -> PathBuf {
        self.doc_dir(doc_uuid).join(VERSIONS_SUBDIR)
    }

    pub fn version_path(&self, doc_uuid: &str, version_num: i64) -> PathBuf {
        self.versions_dir(doc_uuid).join(format!("v{:06}.enc", version_num))
    }

    pub fn session_path(&self, doc_uuid: &str) -> PathBuf {
        self.base_dir.join(SESSIONS_DIR).join(format!("{}.enc", doc_uuid))
    }

    pub fn read_encrypted(&self, file_path: &PathBuf, mek: &[u8; 32]) -> Result<String, AppError> {
        Self::do_read_encrypted(file_path, mek)
    }

    pub fn read_encrypted_from(&self, file_path: &PathBuf, mek: &[u8; 32]) -> Result<String, AppError> {
        Self::do_read_encrypted(file_path, mek)
    }

    pub fn write_encrypted(
        &self,
        file_path: &PathBuf,
        content: &str,
        mek: &[u8; 32],
    ) -> Result<i64, AppError> {
        Self::do_write_encrypted(file_path, content, mek)
    }

    pub fn write_encrypted_to(
        &self,
        file_path: &PathBuf,
        content: &str,
        mek: &[u8; 32],
    ) -> Result<i64, AppError> {
        Self::do_write_encrypted(file_path, content, mek)
    }

    fn do_read_encrypted(file_path: &PathBuf, mek: &[u8; 32]) -> Result<String, AppError> {
        let metadata = fs::metadata(file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound
            } else {
                AppError::FileSystem(e)
            }
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(AppError::Validation(format!(
                "文件过大: {} bytes (最大 {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        let mut file = fs::File::open(file_path).map_err(AppError::FileSystem)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(AppError::FileSystem)?;

        if buf.len() < 12 {
            return Err(AppError::Crypto("文件数据不完整".into()));
        }

        let (nonce_bytes, ciphertext) = buf.split_at(12);
        let nonce: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| AppError::Crypto("nonce 长度错误".into()))?;

        let plaintext = aes_gcm::decrypt_bytes(ciphertext, mek, &nonce)?;
        String::from_utf8(plaintext).map_err(|e| AppError::Crypto(format!("UTF-8 解码失败: {}", e)))
    }

    fn do_write_encrypted(
        file_path: &PathBuf,
        content: &str,
        mek: &[u8; 32],
    ) -> Result<i64, AppError> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(AppError::FileSystem)?;
        }

        let (ciphertext, nonce) = aes_gcm::encrypt_bytes(content.as_bytes(), mek)?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext);

        let mut file = fs::File::create(file_path).map_err(AppError::FileSystem)?;
        file.write_all(&combined).map_err(AppError::FileSystem)?;

        Ok(combined.len() as i64)
    }

    pub fn delete_document_dir(&self, doc_uuid: &str) -> Result<(), AppError> {
        let dir = self.doc_dir(doc_uuid);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(AppError::FileSystem)?;
        }
        let session = self.session_path(doc_uuid);
        if session.exists() {
            fs::remove_file(&session).map_err(AppError::FileSystem)?;
        }
        Ok(())
    }

    pub fn delete_session(&self, doc_uuid: &str) -> Result<(), AppError> {
        let session = self.session_path(doc_uuid);
        if session.exists() {
            fs::remove_file(&session).map_err(AppError::FileSystem)?;
        }
        Ok(())
    }
}