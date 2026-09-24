use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

const RESOURCE_BASE: &str = "https://resources.download.minecraft.net";

pub async fn install_assets(
    dm: &DownloadManager,
    assets_dir: &Path,
    index_id: &str,
    index_url: &str,
    index_sha1: &str,
    index_size: u64,
    progress: Option<Arc<dyn Fn(usize, usize) + Send + Sync>>,
) -> Result<(usize, usize)> {
    crate::utils::fs::safe_file_name(index_id)?;
    let indexes_dir = assets_dir.join("indexes");
    std::fs::create_dir_all(&indexes_dir)?;
    let index_path = indexes_dir.join(format!("{index_id}.json"));
    let job = DownloadJob::new(
        format!("assets index {index_id}"),
        index_url,
        index_path.clone(),
    )
    .with_sha1(index_sha1)
    .with_size(index_size);
    dm.download(&job, None).await?;

    let bytes = std::fs::read(&index_path)?;
    let index: AssetIndex = serde_json::from_slice(&bytes)?;

    let objects_dir = assets_dir.join("objects");
    let mut jobs: Vec<(String, DownloadJob)> = Vec::new();
    for (name, obj) in &index.objects {
        if obj.hash.len() != 40 || !obj.hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(crate::error::MonoryxError::UnsafePath(obj.hash.clone()));
        }
        let prefix = obj.hash.get(0..2).unwrap_or("xx").to_string();
        let dest: PathBuf = objects_dir.join(&prefix).join(&obj.hash);
        if dest.exists()
            && std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) == obj.size
            && crate::utils::hash::sha1_file(&dest)
                .is_ok_and(|hash| hash.eq_ignore_ascii_case(&obj.hash))
        {
            continue;
        }
        let url = format!("{RESOURCE_BASE}/{prefix}/{}", obj.hash);
        let job = DownloadJob::new(format!("asset {name}"), url, dest).with_size(obj.size);

        let job = job.with_sha1(obj.hash.clone());
        jobs.push((name.clone(), job));
    }

    let total = jobs.len();
    let mut done = 0usize;
    let mut skipped = index.objects.len().saturating_sub(total);

    let mut handles = Vec::new();
    for (_, job) in jobs {
        let dm = dm.clone();
        handles.push(tokio::spawn(async move { dm.download(&job, None).await }));
    }
    for h in handles {
        h.await
            .map_err(|e| crate::error::MonoryxError::Download(e.to_string()))??;
        done += 1;
        if let Some(cb) = &progress {
            cb(done, total);
        }
    }
    skipped += 0;
    Ok((done, skipped))
}
