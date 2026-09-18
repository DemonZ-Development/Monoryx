use crate::error::Result;

pub fn atomic_write_bytes(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    crate::utils::fs::atomic_write(path, bytes)
}

pub fn atomic_write_str(path: &std::path::Path, content: &str) -> Result<()> {
    crate::utils::fs::atomic_write_str(path, content)
}
