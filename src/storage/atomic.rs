use crate::error::Result;

pub fn atomic_write_str(path: &std::path::Path, content: &str) -> Result<()> {
    crate::utils::fs::atomic_write_str(path, content)
}
