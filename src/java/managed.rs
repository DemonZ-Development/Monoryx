use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

const ADOPTIUM_API: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptiumRelease {
    pub release_name: String,
    pub binaries: Vec<AdoptiumBinary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptiumBinary {
    pub architecture: String,
    pub os: String,
    pub image_type: String,
    pub package: AdoptiumPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptiumPackage {
    pub link: String,
    pub sha256sum_link: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    pub size: Option<u64>,
}

#[must_use]
pub fn managed_java_exe(runtimes_dir: &std::path::Path, major: u32) -> Option<std::path::PathBuf> {
    let dir = runtimes_dir.join(format!("temurin-{major}"));
    runtime_exe(&dir)
}

fn runtime_exe(dir: &Path) -> Option<PathBuf> {
    let cands = if cfg!(target_os = "windows") {
        vec![dir.join("bin/java.exe"), dir.join("bin/javaw.exe")]
    } else {
        vec![dir.join("bin/java"), dir.join("Contents/Home/bin/java")]
    };
    cands.into_iter().find(|p| p.is_file())
}

pub async fn install_managed(
    dm: &DownloadManager,
    runtimes_dir: &std::path::Path,
    major: u32,
    progress: Option<crate::downloads::manager::ProgressCallback>,
) -> Result<std::path::PathBuf> {
    if let Some(exe) = managed_java_exe(runtimes_dir, major) {
        if validate_runtime(&exe, major).await.is_ok() {
            return Ok(exe);
        }
    }
    let (os, arch) = adoptium_platform(std::env::consts::OS, std::env::consts::ARCH)?;
    let url = format!(
        "{ADOPTIUM_API}/assets/feature_releases/{major}/ga?architecture={arch}&os={os}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC&vendor=eclipse"
    );
    let releases: Vec<AdoptiumRelease> =
        crate::utils::net::get_json_with_retry(dm.client(), &url, None)
            .await
            .map_err(|e| {
                MonoryxError::JavaNotFound(format!(
                    "couldn't query Adoptium for Java {major}: {}",
                    e.user_message()
                ))
            })?;
    let bin = select_adoptium_binary(&releases, os, arch, major)?;
    let expected_sha256 = bin.package.checksum.clone().unwrap_or_default();
    validate_checksum(&expected_sha256)?;
    tokio::fs::create_dir_all(runtimes_dir).await?;
    let workspace = tempfile::Builder::new()
        .prefix(".temurin-install-")
        .tempdir_in(runtimes_dir)?;
    let archive_path = archive_dest(workspace.path(), major, &bin.package.link)?;
    let mut job = DownloadJob::new(
        format!("Temurin Java {major}"),
        &bin.package.link,
        archive_path.clone(),
    );
    if let Some(size) = bin.package.size {
        job = job.with_size(size);
    }
    dm.download(&job, progress).await?;
    let staged = workspace.path().join("runtime");
    let (workspace, staged) = tokio::task::spawn_blocking(move || {
        verify_package_sha256(&archive_path, &expected_sha256)?;
        extract_runtime_archive(&archive_path, &staged)?;
        Ok::<_, MonoryxError>((workspace, staged))
    })
    .await
    .map_err(|e| MonoryxError::Archive(e.to_string()))??;
    let exe = runtime_exe(&staged).ok_or_else(|| {
        MonoryxError::JavaNotFound(
            "managed Java extracted but no java binary was found".to_string(),
        )
    })?;
    validate_runtime(&exe, major).await?;
    let relative_exe = exe
        .strip_prefix(&staged)
        .map_err(|e| MonoryxError::Archive(e.to_string()))?
        .to_path_buf();
    let dest = runtimes_dir.join(format!("temurin-{major}"));
    tokio::task::spawn_blocking(move || {
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        std::fs::rename(&staged, &dest)?;
        drop(workspace);
        Ok::<_, MonoryxError>(dest.join(relative_exe))
    })
    .await
    .map_err(|e| MonoryxError::Archive(e.to_string()))?
}

fn extract_runtime_archive(archive: &std::path::Path, dest: &std::path::Path) -> Result<()> {
    use std::fs::File;
    let name = archive.to_string_lossy().to_string();
    if name.ends_with(".zip") {
        let f = File::open(archive)?;
        let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
        let tmp = dest.with_extension("extract-tmp");
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| MonoryxError::Archive(e.to_string()))?;
            if entry.is_dir() || entry.is_symlink() {
                continue;
            }
            let ename = entry.name().replace('\\', "/");
            let rel = strip_top_level(&ename);
            if rel.is_empty() {
                continue;
            }
            let out = crate::utils::fs::safe_join(&tmp, rel)?;
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut o = File::create(&out)?;
            std::io::copy(&mut entry, &mut o)?;
        }
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }
        std::fs::rename(&tmp, dest)?;
        return Ok(());
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let f = File::open(archive)?;
        let gz = flate2::read::GzDecoder::new(f);
        let mut tar = tar::Archive::new(gz);
        let tmp = dest.with_extension("extract-tmp");
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        for entry in tar
            .entries()
            .map_err(|e| MonoryxError::Archive(e.to_string()))?
        {
            let mut entry = entry.map_err(|e| MonoryxError::Archive(e.to_string()))?;
            let path = entry
                .path()
                .map_err(|e| MonoryxError::Archive(e.to_string()))?
                .to_string_lossy()
                .to_string();
            let rel = strip_top_level(&path);
            if rel.is_empty() {
                continue;
            }
            if entry.header().entry_type().is_symlink() {
                continue;
            }
            if entry.header().entry_type().is_dir() {
                continue;
            }
            let out = crate::utils::fs::safe_join(&tmp, rel)?;
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            entry
                .unpack(&out)
                .map_err(|e| MonoryxError::Archive(e.to_string()))?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let java_bin = tmp.join("bin").join("java");
            if java_bin.exists() {
                let mut perms = std::fs::metadata(&java_bin)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&java_bin, perms)?;
            }
        }
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }
        std::fs::rename(&tmp, dest)?;
        return Ok(());
    }
    Err(MonoryxError::Archive(
        "unsupported managed runtime archive".to_string(),
    ))
}

fn strip_top_level(name: &str) -> &str {
    match name.find('/') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

fn adoptium_platform(os: &str, arch: &str) -> Result<(&'static str, &'static str)> {
    match (os, arch) {
        ("windows", "x86_64") => Ok(("windows", "x64")),
        ("windows", "aarch64") => Ok(("windows", "aarch64")),
        ("macos", "x86_64") => Ok(("mac", "x64")),
        ("macos", "aarch64") => Ok(("mac", "aarch64")),
        ("linux", "x86_64") => Ok(("linux", "x64")),
        ("linux", "aarch64") => Ok(("linux", "aarch64")),
        _ => Err(MonoryxError::JavaNotFound(format!(
            "no Adoptium binaries are published for {os}/{arch}"
        ))),
    }
}

fn archive_dest(runtimes_dir: &Path, major: u32, link: &str) -> Result<PathBuf> {
    let url = url::Url::parse(link).map_err(|e| MonoryxError::Download(e.to_string()))?;
    let base = runtimes_dir.join(format!(".temurin-{major}.download"));
    if url.path().ends_with(".zip") {
        Ok(base.with_extension("zip"))
    } else if url.path().ends_with(".tar.gz") || url.path().ends_with(".tgz") {
        Ok(base.with_extension("tar.gz"))
    } else {
        Err(MonoryxError::Archive(
            "unsupported managed runtime archive".to_string(),
        ))
    }
}

fn select_adoptium_binary(
    releases: &[AdoptiumRelease],
    os: &str,
    arch: &str,
    major: u32,
) -> Result<AdoptiumBinary> {
    let mut candidates = releases.iter().flat_map(|r| r.binaries.iter());
    let exact_jre = candidates
        .find(|b| {
            b.os.eq_ignore_ascii_case(os)
                && b.architecture.eq_ignore_ascii_case(arch)
                && b.image_type.eq_ignore_ascii_case("jre")
        })
        .cloned();
    if let Some(b) = exact_jre {
        return Ok(b);
    }
    let jdk = releases
        .iter()
        .flat_map(|r| r.binaries.iter())
        .find(|b| {
            b.os.eq_ignore_ascii_case(os)
                && b.architecture.eq_ignore_ascii_case(arch)
                && b.image_type.eq_ignore_ascii_case("jdk")
        })
        .cloned();
    jdk.ok_or_else(|| {
        MonoryxError::JavaNotFound(format!(
            "no Adoptium JRE or JDK found for Java {major} on {os}/{arch}"
        ))
    })
}

fn validate_checksum(expected: &str) -> Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(MonoryxError::Download(
            "Adoptium package has a missing or invalid SHA256 checksum".to_string(),
        ));
    }
    Ok(())
}

fn verify_package_sha256(archive: &Path, expected: &str) -> Result<()> {
    use std::io::Read as _;
    validate_checksum(expected)?;
    let mut file = std::fs::File::open(archive)?;
    let mut h = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        h.update(&buffer[..count]);
    }
    let actual = hex::encode(h.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(MonoryxError::HashMismatch {
            file: archive.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

async fn validate_runtime(exe: &Path, expected_major: u32) -> Result<()> {
    let rt = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        crate::java::discovery::probe(exe, "managed"),
    )
    .await
    .map_err(|_| {
        MonoryxError::JavaNotFound(format!(
            "timed out probing managed Java at {}",
            exe.display()
        ))
    })?
    .ok_or_else(|| {
        MonoryxError::JavaNotFound(format!(
            "managed Java at {} did not produce a version output",
            exe.display()
        ))
    })?;
    if rt.major != expected_major {
        return Err(MonoryxError::JavaNotFound(format!(
            "managed runtime reported Java {}, expected {expected_major}",
            rt.major
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(binaries: Vec<AdoptiumBinary>) -> Vec<AdoptiumRelease> {
        vec![AdoptiumRelease {
            release_name: "test".to_string(),
            binaries,
        }]
    }

    fn binary(os: &str, arch: &str, image_type: &str, link: &str) -> AdoptiumBinary {
        AdoptiumBinary {
            os: os.to_string(),
            architecture: arch.to_string(),
            image_type: image_type.to_string(),
            package: AdoptiumPackage {
                link: link.to_string(),
                sha256sum_link: None,
                checksum: None,
                size: None,
            },
        }
    }

    #[test]
    fn platform_maps_host_or_errors() {
        if let Ok((os, arch)) = adoptium_platform(std::env::consts::OS, std::env::consts::ARCH) {
            assert!(!os.is_empty() && !arch.is_empty());
        }
        assert!(adoptium_platform("plan9", "m68k").is_err());
    }

    #[test]
    fn selects_matching_jre_binary() {
        let releases = release(vec![
            binary("linux", "aarch64", "jdk", "https://x/jdk.tar.gz"),
            binary("linux", "x64", "jre", "https://x/jre.zip"),
        ]);
        let sel = select_adoptium_binary(&releases, "linux", "x64", 21).unwrap();
        assert_eq!(sel.package.link, "https://x/jre.zip");
    }

    #[test]
    fn falls_back_to_jdk_when_no_jre() {
        let releases = release(vec![binary(
            "mac",
            "aarch64",
            "jdk",
            "https://x/jdk.tar.gz",
        )]);
        let sel = select_adoptium_binary(&releases, "mac", "aarch64", 25).unwrap();
        assert_eq!(sel.image_type, "jdk");
    }

    #[test]
    fn never_selects_wrong_os_or_arch() {
        let releases = release(vec![
            binary("windows", "x64", "jre", "https://x/win.zip"),
            binary("linux", "aarch64", "jre", "https://x/arm.tar.gz"),
        ]);
        assert!(select_adoptium_binary(&releases, "linux", "x64", 21).is_err());
    }

    #[test]
    fn rejects_unsupported_archive_link() {
        let releases = release(vec![binary("linux", "x64", "jre", "https://x/jre.bin")]);
        let bin = select_adoptium_binary(&releases, "linux", "x64", 21).unwrap();
        assert!(archive_dest(Path::new("/tmp"), 21, &bin.package.link).is_err());
    }

    #[test]
    fn checksum_metadata_is_required_and_hex64() {
        assert!(validate_checksum("").is_err());
        assert!(validate_checksum("abc123").is_err());
        assert!(validate_checksum(&"g".repeat(64)).is_err());
        assert!(validate_checksum(&"a".repeat(64)).is_ok());
        assert!(validate_checksum(&"A".repeat(64)).is_ok());
    }

    #[test]
    fn checksum_mismatch_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pkg.zip");
        std::fs::write(&file, b"payload").unwrap();
        let good = {
            let mut h = Sha256::new();
            h.update(b"payload");
            hex::encode(h.finalize())
        };
        assert!(verify_package_sha256(&file, &good).is_ok());
        let err = verify_package_sha256(&file, &"0".repeat(64)).unwrap_err();
        assert!(matches!(err, MonoryxError::HashMismatch { .. }));
    }

    #[test]
    fn missing_checksum_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pkg.zip");
        std::fs::write(&file, b"payload").unwrap();
        assert!(verify_package_sha256(&file, "").is_err());
    }

    #[test]
    fn extracts_zip_stripping_top_level() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("rt.zip");
        {
            let f = std::fs::File::create(&archive).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            zw.start_file(
                "jdk-21/bin/java.exe",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            std::io::Write::write_all(&mut zw, b"MZ").unwrap();
            zw.finish().unwrap();
        }
        let dest = dir.path().join("out");
        extract_runtime_archive(&archive, &dest).unwrap();
        assert!(dest.join("bin/java.exe").is_file());
    }

    #[test]
    fn extracts_tar_gz_stripping_top_level() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("rt.tar.gz");
        {
            let f = std::fs::File::create(&archive).unwrap();
            let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
            let mut tw = tar::Builder::new(enc);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_cksum();
            tw.append_data(&mut header, "jdk-21/bin/java", &b"ELF"[..])
                .unwrap();
            let enc = tw.into_inner().unwrap();
            enc.finish().unwrap();
        }
        let dest = dir.path().join("out");
        extract_runtime_archive(&archive, &dest).unwrap();
        assert!(dest.join("bin/java").is_file());
    }
}
