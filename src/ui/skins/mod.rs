use base64::Engine;
use egui::ColorImage;
use serde::Deserialize;

pub mod viewer;

const TEXTURE_HOST: &str = "textures.minecraft.net";
const MAX_TEXTURE_BYTES: usize = 256 * 1024;
const MAX_TEXTURE_SIZE: u32 = 256;

#[derive(Clone, Deserialize)]
pub struct Cape {
    pub id: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub state: String,
}

pub struct LoadedSkin {
    pub skin: ColorImage,
    pub cape: Option<ColorImage>,
    pub skin_url: String,
    pub cape_url: Option<String>,
    pub slim: bool,
}

#[derive(Deserialize)]
struct Player {
    id: String,
}

#[derive(Deserialize)]
struct Session {
    properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct Textures {
    textures: TextureSet,
}

#[derive(Deserialize)]
struct TextureSet {
    #[serde(rename = "SKIN")]
    skin: Option<Texture>,
    #[serde(rename = "CAPE")]
    cape: Option<Texture>,
}

#[derive(Deserialize)]
struct Texture {
    url: String,
    metadata: Option<TextureMetadata>,
}

#[derive(Deserialize)]
struct TextureMetadata {
    model: Option<String>,
}

#[derive(Deserialize)]
struct Profile {
    #[serde(default)]
    capes: Vec<Cape>,
}

async fn send(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();

    if status.is_success() {
        return Ok(response);
    }

    Err(match status.as_u16() {
        401 | 403 => "Authentication failed. Sign in again.".to_string(),
        404 => "Player or profile not found.".to_string(),
        429 => "Rate limited by Mojang. Try again in a minute.".to_string(),
        code => format!("Request failed (HTTP {code})."),
    })
}

pub(crate) async fn load_skin(
    http: &reqwest::Client,
    username: &str,
) -> Result<LoadedSkin, String> {
    let valid_length = (3..=16).contains(&username.len());
    let valid_chars = username
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_');

    if !valid_length || !valid_chars {
        return Err("Invalid Minecraft username.".into());
    }

    let lookup_url = format!("https://api.mojang.com/users/profiles/minecraft/{username}");
    let response = send(http.get(lookup_url)).await?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Err("Player not found.".into());
    }

    let player = response.json::<Player>().await.map_err(|e| e.to_string())?;

    let session_url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}",
        player.id
    );

    let session = send(http.get(session_url))
        .await?
        .json::<Session>()
        .await
        .map_err(|e| e.to_string())?;

    let property = session
        .properties
        .iter()
        .find(|property| property.name == "textures")
        .ok_or("Missing textures property")?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&property.value)
        .map_err(|e| e.to_string())?;

    let textures: Textures = serde_json::from_slice(&decoded).map_err(|e| e.to_string())?;

    let skin = textures.textures.skin.ok_or("Player has no skin texture")?;

    let slim = skin
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.model.as_deref())
        == Some("slim");

    let secure = |url: &str| url.replacen("http://", "https://", 1);

    let skin_url = secure(&skin.url);
    let skin_image = download_image(http, &skin_url, true).await?;

    let cape_url = textures.textures.cape.map(|cape| secure(&cape.url));

    let cape_image = match &cape_url {
        Some(url) => download_image(http, url, false).await.ok(),
        None => None,
    };

    Ok(LoadedSkin {
        skin: skin_image,
        cape: cape_image,
        skin_url,
        cape_url,
        slim,
    })
}

pub(crate) async fn download_image(
    http: &reqwest::Client,
    url: &str,
    skin: bool,
) -> Result<ColorImage, String> {
    let mut parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;

    if parsed.host_str() != Some(TEXTURE_HOST) || !matches!(parsed.scheme(), "http" | "https") {
        return Err("Untrusted texture URL.".into());
    }

    let _ = parsed.set_scheme("https");

    let bytes = send(http.get(parsed))
        .await?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err("Texture exceeds size limit.".into());
    }

    let mut image = image::load_from_memory(&bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    let (width, height) = image.dimensions();

    if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
        return Err("Texture dimensions exceed limit.".into());
    }

    if skin && (width, height) == (64, 32) {
        image = upgrade_legacy_skin(&image);
    }

    Ok(ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

pub(crate) fn upgrade_legacy_skin(image: &image::RgbaImage) -> image::RgbaImage {
    const MIRRORS: [[u32; 6]; 12] = [
        [4, 16, 4, 4, 20, 48],
        [8, 16, 4, 4, 24, 48],
        [0, 20, 4, 12, 24, 52],
        [4, 20, 4, 12, 20, 52],
        [8, 20, 4, 12, 16, 52],
        [12, 20, 4, 12, 28, 52],
        [44, 16, 4, 4, 36, 48],
        [48, 16, 4, 4, 40, 48],
        [40, 20, 4, 12, 40, 52],
        [44, 20, 4, 12, 36, 52],
        [48, 20, 4, 12, 32, 52],
        [52, 20, 4, 12, 44, 52],
    ];

    let mut upgraded = image::RgbaImage::new(64, 64);
    image::imageops::replace(&mut upgraded, image, 0, 0);

    for (x, y, pixel) in upgraded.enumerate_pixels_mut() {
        if y < 32 && (y >= 16 || x < 32) {
            pixel.0[3] = 255;
        }
    }

    for [sx, sy, w, h, dx, dy] in MIRRORS {
        let patch = image::imageops::crop_imm(&upgraded, sx, sy, w, h).to_image();
        let mirrored = image::imageops::flip_horizontal(&patch);

        image::imageops::replace(&mut upgraded, &mirrored, i64::from(dx), i64::from(dy));
    }

    upgraded
}

pub(crate) async fn upload_skin(
    http: &reqwest::Client,
    token: &str,
    mut png: Vec<u8>,
    slim: bool,
) -> Result<(), String> {
    if png.len() > MAX_TEXTURE_BYTES {
        return Err("Skin file is too large.".into());
    }

    if image::guess_format(&png).ok() != Some(image::ImageFormat::Png) {
        return Err("Skin must be a PNG file.".into());
    }

    let decoded = image::load_from_memory(&png).map_err(|e| e.to_string())?;

    if decoded.width() != 64 || !matches!(decoded.height(), 32 | 64) {
        return Err("Skin must be a 64x64 or 64x32 PNG.".into());
    }

    if decoded.height() == 32 {
        let rgba = decoded.to_rgba8();
        let upgraded = upgrade_legacy_skin(&rgba);
        let mut buffer = std::io::Cursor::new(Vec::with_capacity(4096));
        upgraded
            .write_to(&mut buffer, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        png = buffer.into_inner();
    }

    let part = reqwest::multipart::Part::bytes(png)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("variant", if slim { "slim" } else { "classic" })
        .part("file", part);

    let request = http
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(token)
        .multipart(form);

    send(request).await?;

    Ok(())
}

pub(crate) fn load_local_skin_file(path: &std::path::Path) -> Result<ColorImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err("Skin file exceeds size limit.".into());
    }
    let mut image = image::load_from_memory(&bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width != 64 || !matches!(height, 32 | 64) {
        return Err("Skin must be a 64x64 or 64x32 PNG.".into());
    }
    if height == 32 {
        image = upgrade_legacy_skin(&image);
    }
    Ok(ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

pub(crate) fn save_offline_skin(
    skins_dir: &std::path::Path,
    username: &str,
    source_path: &std::path::Path,
) -> Result<ColorImage, String> {
    let bytes = std::fs::read(source_path).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err("Skin file is too large.".into());
    }
    let decoded = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    if decoded.width() != 64 || !matches!(decoded.height(), 32 | 64) {
        return Err("Skin must be a 64x64 or 64x32 PNG.".into());
    }
    let mut image = decoded.to_rgba8();
    if decoded.height() == 32 {
        image = upgrade_legacy_skin(&image);
    }
    std::fs::create_dir_all(skins_dir).map_err(|e| e.to_string())?;
    let dest = skins_dir.join(format!("{username}.png"));
    image.save(&dest).map_err(|e| e.to_string())?;

    Ok(ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

pub(crate) fn remove_offline_skin(skins_dir: &std::path::Path, username: &str) {
    let path = skins_dir.join(format!("{username}.png"));
    if path.is_file() {
        let _ = std::fs::remove_file(path);
    }
}

pub(crate) fn has_offline_skin(skins_dir: &std::path::Path, username: &str) -> bool {
    skins_dir.join(format!("{username}.png")).is_file()
}

pub(crate) fn default_steve_skin() -> ColorImage {
    let bytes = include_bytes!("../../../assets/skins/steve.png");
    let image = image::load_from_memory(bytes)
        .expect("steve skin")
        .to_rgba8();
    ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}

pub(crate) fn default_alex_skin() -> ColorImage {
    let bytes = include_bytes!("../../../assets/skins/alex.png");
    let image = image::load_from_memory(bytes)
        .expect("alex skin")
        .to_rgba8();
    ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}

pub(crate) fn default_skin_for_player(username: &str) -> (ColorImage, bool) {
    let hash = username
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
    if (hash & 1) == 1 {
        (default_alex_skin(), true)
    } else {
        (default_steve_skin(), false)
    }
}

pub(crate) fn save_offline_preset(
    skins_dir: &std::path::Path,
    username: &str,
    is_alex: bool,
) -> Result<ColorImage, String> {
    let bytes: &[u8] = if is_alex {
        include_bytes!("../../../assets/skins/alex.png")
    } else {
        include_bytes!("../../../assets/skins/steve.png")
    };
    let decoded = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    std::fs::create_dir_all(skins_dir).map_err(|e| e.to_string())?;
    let dest = skins_dir.join(format!("{username}.png"));
    decoded.save(&dest).map_err(|e| e.to_string())?;
    Ok(ColorImage::from_rgba_unmultiplied(
        [decoded.width() as usize, decoded.height() as usize],
        decoded.as_raw(),
    ))
}

pub(crate) async fn load_capes(http: &reqwest::Client, token: &str) -> Result<Vec<Cape>, String> {
    let request = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(token);

    let profile = send(request)
        .await?
        .json::<Profile>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(profile.capes)
}

pub(crate) async fn apply_cape(
    http: &reqwest::Client,
    token: &str,
    id: &str,
) -> Result<(), String> {
    let request = http
        .put("https://api.minecraftservices.com/minecraft/profile/capes/active")
        .bearer_auth(token)
        .json(&serde_json::json!({ "capeId": id }));

    send(request).await?;

    Ok(())
}

#[must_use]
pub fn build_skinsrestorer_command(skin_url: &str, slim: bool) -> String {
    let variant = if slim { "slim" } else { "classic" };
    format!("/skin url {skin_url} {variant}")
}

pub async fn upload_offline_skin(
    http: &reqwest::Client,
    endpoint: &str,
    username: &str,
    mut png: Vec<u8>,
    slim: bool,
) -> Result<String, String> {
    if png.len() > MAX_TEXTURE_BYTES {
        return Err("Skin file is too large.".into());
    }

    if image::guess_format(&png).ok() != Some(image::ImageFormat::Png) {
        return Err("Skin must be a PNG file.".into());
    }

    let decoded = image::load_from_memory(&png).map_err(|e| e.to_string())?;

    if decoded.width() != 64 || !matches!(decoded.height(), 32 | 64) {
        return Err("Skin must be a 64x64 or 64x32 PNG.".into());
    }

    if decoded.height() == 32 {
        let rgba = decoded.to_rgba8();
        let upgraded = upgrade_legacy_skin(&rgba);
        let mut buffer = std::io::Cursor::new(Vec::with_capacity(4096));
        upgraded
            .write_to(&mut buffer, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        png = buffer.into_inner();
    }

    let target_url = if endpoint.trim().is_empty() {
        crate::utils::links::DEFAULT_SKIN_SERVER_URL
    } else {
        endpoint.trim()
    };

    let model_str = if slim { "slim" } else { "classic" };
    let filename = format!(
        "{}.png",
        if username.trim().is_empty() {
            "skin"
        } else {
            username.trim()
        }
    );

    let part = reqwest::multipart::Part::bytes(png)
        .file_name(filename)
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("username", username.trim().to_string())
        .text("model", model_str.to_string())
        .text("variant", model_str.to_string())
        .part("file", part);

    let response = http
        .post(target_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Upload to skin server failed (HTTP {status}): {body}"));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if let Some(url) = extract_url_from_json(&json) {
            return Ok(url);
        }
    }

    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    if text.starts_with("http://") || text.starts_with("https://") {
        return Ok(text);
    }

    Err(format!(
        "Skin server responded successfully but did not provide a URL: {text}"
    ))
}

fn extract_url_from_json(json: &serde_json::Value) -> Option<String> {
    for key in [
        "url",
        "skin_url",
        "direct_url",
        "file_url",
        "download_url",
        "imageUrl",
        "image_url",
    ] {
        if let Some(val) = json.get(key).and_then(|v| v.as_str()) {
            if val.starts_with("http://") || val.starts_with("https://") {
                return Some(val.to_string());
            }
        }
    }

    for ptr in ["/data/url", "/data/skin_url", "/data/texture/url"] {
        if let Some(val) = json.pointer(ptr).and_then(|v| v.as_str()) {
            if val.starts_with("http://") || val.starts_with("https://") {
                return Some(val.to_string());
            }
        }
    }

    None
}

pub fn sync_instance_skins(
    skins_dir: &std::path::Path,
    game_dir: &std::path::Path,
    username: &str,
) -> Result<(), String> {
    let clean_user = username.trim();
    if clean_user.is_empty() {
        return Ok(());
    }

    let source_skin = skins_dir.join(format!("{clean_user}.png"));
    let csl_dir = game_dir.join("CustomSkinLoader");
    let csl_local_skins = csl_dir.join("LocalSkin").join("skins");
    let csl_legacy_skins = csl_dir.join("skins");
    let generic_skins = game_dir.join("skins");
    let cached_skins = game_dir.join("cachedImages").join("skins");

    if source_skin.is_file() {
        let skin_bytes = std::fs::read(&source_skin).map_err(|e| e.to_string())?;

        for dir in [&csl_local_skins, &csl_legacy_skins, &generic_skins, &cached_skins] {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let dest = dir.join(format!("{clean_user}.png"));
            let _ = std::fs::write(dest, &skin_bytes);
        }

        let source_cape = skins_dir.join(format!("{clean_user}_cape.png"));
        if source_cape.is_file() {
            if let Ok(cape_bytes) = std::fs::read(&source_cape) {
                let csl_capes = csl_dir.join("LocalSkin").join("capes");
                if std::fs::create_dir_all(&csl_capes).is_ok() {
                    let _ = std::fs::write(csl_capes.join(format!("{clean_user}.png")), cape_bytes);
                }
            }
        }

        ensure_custom_skin_loader_config(&csl_dir)?;
    } else {
        for dir in [&csl_local_skins, &csl_legacy_skins, &generic_skins, &cached_skins] {
            let target = dir.join(format!("{clean_user}.png"));
            if target.is_file() {
                let _ = std::fs::remove_file(target);
            }
        }
        let csl_cape = csl_dir
            .join("LocalSkin")
            .join("capes")
            .join(format!("{clean_user}.png"));
        if csl_cape.is_file() {
            let _ = std::fs::remove_file(csl_cape);
        }
    }

    Ok(())
}

fn ensure_custom_skin_loader_config(csl_dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(csl_dir).map_err(|e| e.to_string())?;
    let config_path = csl_dir.join("CustomSkinLoader.json");

    if !config_path.exists() {
        let default_config = serde_json::json!({
            "version": "14.12",
            "loadlist": [
                {
                    "name": "LocalSkin",
                    "type": "Legacy",
                    "checkPNG": false,
                    "skin": "LocalSkin/skins/{USERNAME}.png",
                    "model": "auto",
                    "cape": "LocalSkin/capes/{USERNAME}.png",
                    "elytra": "LocalSkin/elytras/{USERNAME}.png"
                },
                {
                    "name": "MojangAPI",
                    "type": "MojangAPI"
                }
            ]
        });
        let formatted = serde_json::to_string_pretty(&default_config).map_err(|e| e.to_string())?;
        std::fs::write(&config_path, formatted).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let existing_text = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&existing_text) {
        if let Some(list) = parsed.get_mut("loadlist").and_then(|v| v.as_array_mut()) {
            let has_local_skin = list.iter().any(|entry| {
                entry.get("name").and_then(|n| n.as_str()) == Some("LocalSkin")
            });
            if !has_local_skin {
                let local_entry = serde_json::json!({
                    "name": "LocalSkin",
                    "type": "Legacy",
                    "checkPNG": false,
                    "skin": "LocalSkin/skins/{USERNAME}.png",
                    "model": "auto",
                    "cape": "LocalSkin/capes/{USERNAME}.png",
                    "elytra": "LocalSkin/elytras/{USERNAME}.png"
                });
                list.insert(0, local_entry);
                if let Ok(updated) = serde_json::to_string_pretty(&parsed) {
                    let _ = std::fs::write(&config_path, updated);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skinsrestorer_command_formatting() {
        assert_eq!(
            build_skinsrestorer_command("https://demonz.org/skins/Steve.png", false),
            "/skin url https://demonz.org/skins/Steve.png classic"
        );
        assert_eq!(
            build_skinsrestorer_command("https://demonz.org/skins/Alex.png", true),
            "/skin url https://demonz.org/skins/Alex.png slim"
        );
    }

    #[test]
    fn test_extract_url_from_various_json_structures() {
        let v1 = serde_json::json!({ "url": "https://demonz.org/api/skins/download/123.png" });
        assert_eq!(
            extract_url_from_json(&v1).as_deref(),
            Some("https://demonz.org/api/skins/download/123.png")
        );

        let v2 = serde_json::json!({ "direct_url": "https://cdn.demonz.org/skin.png" });
        assert_eq!(
            extract_url_from_json(&v2).as_deref(),
            Some("https://cdn.demonz.org/skin.png")
        );

        let v3 = serde_json::json!({
            "data": {
                "texture": {
                    "url": "https://textures.minecraft.net/texture/abc"
                }
            }
        });
        assert_eq!(
            extract_url_from_json(&v3).as_deref(),
            Some("https://textures.minecraft.net/texture/abc")
        );

        let invalid = serde_json::json!({ "status": "ok" });
        assert_eq!(extract_url_from_json(&invalid), None);
    }

    #[test]
    fn test_sync_instance_skins_creates_expected_tree_and_config() {
        let temp = tempfile::tempdir().unwrap();
        let skins_dir = temp.path().join("skins");
        let game_dir = temp.path().join("game");
        std::fs::create_dir_all(&skins_dir).unwrap();

        let dummy_skin = vec![137, 80, 78, 71, 13, 10, 26, 10];
        std::fs::write(skins_dir.join("TestPlayer.png"), &dummy_skin).unwrap();

        let sync_result = sync_instance_skins(&skins_dir, &game_dir, "TestPlayer");
        assert!(sync_result.is_ok());

        let csl_skin = game_dir
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join("TestPlayer.png");
        assert!(csl_skin.is_file());
        assert_eq!(std::fs::read(csl_skin).unwrap(), dummy_skin);

        let generic_skin = game_dir.join("skins").join("TestPlayer.png");
        assert!(generic_skin.is_file());
        assert_eq!(std::fs::read(generic_skin).unwrap(), dummy_skin);

        let csl_config = game_dir
            .join("CustomSkinLoader")
            .join("CustomSkinLoader.json");
        assert!(csl_config.is_file());
        let content = std::fs::read_to_string(csl_config).unwrap();
        assert!(content.contains("\"LocalSkin\""));
        assert!(content.contains("LocalSkin/skins/{USERNAME}.png"));

        let remove_result = sync_instance_skins(&skins_dir, &game_dir, "GhostPlayer");
        assert!(remove_result.is_ok());

        std::fs::remove_file(skins_dir.join("TestPlayer.png")).unwrap();
        let clean_result = sync_instance_skins(&skins_dir, &game_dir, "TestPlayer");
        assert!(clean_result.is_ok());
        assert!(!game_dir
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join("TestPlayer.png")
            .exists());
        assert!(!game_dir.join("skins").join("TestPlayer.png").exists());
    }

    #[test]
    fn test_sync_instance_skins_preserves_existing_custom_skin_loader_json() {
        let temp = tempfile::tempdir().unwrap();
        let skins_dir = temp.path().join("skins");
        let game_dir = temp.path().join("game");
        let csl_dir = game_dir.join("CustomSkinLoader");
        std::fs::create_dir_all(&skins_dir).unwrap();
        std::fs::create_dir_all(&csl_dir).unwrap();

        std::fs::write(skins_dir.join("Modder.png"), b"pngdata").unwrap();

        let initial_json = serde_json::json!({
            "version": "14.12",
            "loadlist": [
                {
                    "name": "CustomServer",
                    "type": "Custom"
                }
            ]
        });
        std::fs::write(
            csl_dir.join("CustomSkinLoader.json"),
            serde_json::to_string(&initial_json).unwrap(),
        )
        .unwrap();

        let res = sync_instance_skins(&skins_dir, &game_dir, "Modder");
        assert!(res.is_ok());

        let updated_text =
            std::fs::read_to_string(csl_dir.join("CustomSkinLoader.json")).unwrap();
        let updated: serde_json::Value = serde_json::from_str(&updated_text).unwrap();
        let loadlist = updated.get("loadlist").unwrap().as_array().unwrap();
        assert_eq!(loadlist.len(), 2);
        assert_eq!(loadlist[0]["name"], "LocalSkin");
        assert_eq!(loadlist[1]["name"], "CustomServer");

        let res_repeat = sync_instance_skins(&skins_dir, &game_dir, "Modder");
        assert!(res_repeat.is_ok());
        let updated_repeat: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(csl_dir.join("CustomSkinLoader.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            updated_repeat.get("loadlist").unwrap().as_array().unwrap().len(),
            2
        );
    }
}
