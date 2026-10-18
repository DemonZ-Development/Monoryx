use base64::Engine;
use egui::ColorImage;
use serde::Deserialize;

pub mod viewer;

const TEXTURE_HOST: &str = "textures.minecraft.net";
const MAX_TEXTURE_BYTES: usize = 2 * 1024 * 1024;
const MAX_TEXTURE_SIZE: u32 = 1024;

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
        image::imageops::replace(&mut upgraded, &image, 0, 0);

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

        image = upgraded;
    }

    Ok(ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

pub(crate) async fn upload_skin(
    http: &reqwest::Client,
    token: &str,
    png: Vec<u8>,
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
