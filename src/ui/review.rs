use crate::{
    app::{AppState, Page},
    config::ThemeKind,
};
use std::collections::HashMap;

#[test]
fn review_home_worlds_crash_and_discord_layouts() {
    let output_dir = std::env::var_os("MONORYX_UI_REVIEW").map(std::path::PathBuf::from);
    for (label, page, width, height, theme, crash) in [
        (
            "home-wide",
            Page::Home,
            1536,
            816,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "home-small",
            Page::Home,
            850,
            650,
            ThemeKind::SoftBrown,
            false,
        ),
        (
            "worlds-wide",
            Page::Worlds,
            1536,
            816,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "worlds-small",
            Page::Worlds,
            850,
            650,
            ThemeKind::SoftPink,
            false,
        ),
        ("crash", Page::Home, 1280, 800, ThemeKind::Gloss, true),
        (
            "discord",
            Page::Settings,
            1280,
            900,
            ThemeKind::Monochrome,
            false,
        ),
    ] {
        let ctx = egui::Context::default();
        let temp = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(temp.path().into());
        let manager = crate::instance::InstanceManager::new(paths.clone());
        let first = manager
            .create(
                "Survival with friends".into(),
                "1.21.1".into(),
                crate::instance::LoaderKind::Fabric,
                "0.16.14".into(),
            )
            .unwrap();
        manager
            .create(
                "Creative builds".into(),
                "1.21.1".into(),
                crate::instance::LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.page = page;
        state.config.profile =
            Some(crate::account::offline::OfflineProfile::new("PreviewPlayer").unwrap());
        state.config.theme = theme;
        state.selected_instance = Some(first.id.clone());
        state.mod_counts.insert(first.id.clone(), 12);
        let shot_path = temp.path().join("fixture.png");
        let pixels = (0..480 * 270)
            .map(|index| {
                let y = index / 480;
                if y < 130 {
                    egui::Color32::from_rgb(96, 139, 170)
                } else {
                    egui::Color32::from_rgb(51, 90, 63)
                }
            })
            .collect();
        state.screenshot_thumbnails.insert(
            shot_path.clone(),
            ctx.load_texture(
                "review-landscape",
                egui::ColorImage::new([480, 270], pixels),
                egui::TextureOptions::LINEAR,
            ),
        );
        state
            .screenshots
            .push(crate::app::screenshots::ScreenshotEntry {
                path: shot_path,
                instance_id: first.id.clone(),
                instance_name: first.name.clone(),
                modified: std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(1_789_000_000),
                bytes: 1024,
            });
        state.worlds.instance_id = Some(first.id.clone());
        state.worlds.selected_world = Some("Cozy home".into());
        let now = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_789_000_000);
        state.worlds.snapshot.worlds = ["Cozy home", "Creative test"]
            .into_iter()
            .map(|name| crate::instance::worlds::WorldInfo {
                name: name.into(),
                display_name: name.into(),
                modified: now,
                last_played: Some(now),
                version: Some("1.21.1".into()),
                mode: Some("Survival".into()),
                bytes: Some(182_000_000),
                icon: None,
            })
            .collect();
        state.worlds.snapshot.backups = vec![crate::instance::worlds::WorldBackup {
            file_name: "fixture.zip".into(),
            world_name: "Cozy home".into(),
            modified: now,
            bytes: 60_000_000,
        }];
        if crash {
            let log = "Mod 'CraftyAI' (craftyai) 1.4.0 requires version 0.160.6 or later of mod 'Fabric API' (fabric-api), which is missing!";
            state.crash_report = Some(crate::minecraft::crash::CrashInfo {
                instance_id: first.id,
                instance_name: first.name,
                exit_code: 1,
                summary: "Incompatible mods found".into(),
                advice: crate::minecraft::diagnostics::explain(log, 1),
                details: log.into(),
                source_label: "Example session log".into(),
                report_path: None,
                crash_reports_dir: temp.path().join("reports"),
                logs_dir: temp.path().join("logs"),
            });
        }
        if page == Page::Settings {
            crate::ui::pages::settings::select_discord_for_preview(&ctx);
        }
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("startup-window-size-applied"), 5_u8));
        let mut frame = eframe::Frame::_new_kittest();
        let mut textures = HashMap::new();
        let mut final_output = None;
        for pass in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width as f32, height as f32),
                    )),
                    time: Some(pass as f64),
                    ..Default::default()
                },
                |ctx| {
                    crate::ui::shell::app_update(&mut state, ctx, &mut frame);
                },
            );
            for (id, delta) in &output.textures_delta.set {
                let egui::ImageData::Color(image) = &delta.image;
                if let Some([x, y]) = delta.pos {
                    let target: &mut egui::ColorImage = textures.get_mut(id).unwrap();
                    for row in 0..image.size[1] {
                        for col in 0..image.size[0] {
                            target.pixels[(y + row) * target.size[0] + x + col] =
                                image.pixels[row * image.size[0] + col];
                        }
                    }
                } else {
                    textures.insert(*id, (**image).clone());
                }
            }
            final_output = Some(output);
        }
        let output = final_output.unwrap();
        assert!(!output.shapes.is_empty());
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let rect = text.visual_bounding_rect().intersect(shape.clip_rect);
                assert!(
                    rect.right() <= width as f32 + 1.0,
                    "{label}: text extends outside the window"
                );
            }
        }
        if let Some(dir) = &output_dir {
            std::fs::create_dir_all(dir).unwrap();
            raster(&ctx, output, &textures, width, height)
                .save(dir.join(format!("{label}.png")))
                .unwrap();
        }
    }
}

fn raster(
    ctx: &egui::Context,
    output: egui::FullOutput,
    textures: &HashMap<egui::TextureId, egui::ColorImage>,
    width: u32,
    height: u32,
) -> image::RgbaImage {
    let mut canvas = image::RgbaImage::from_pixel(width, height, image::Rgba([11, 12, 14, 255]));
    for clipped in ctx.tessellate(output.shapes, 1.0) {
        let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let texture = &textures[&mesh.texture_id];
        for indices in mesh.indices.chunks_exact(3) {
            let v = [
                mesh.vertices[indices[0] as usize],
                mesh.vertices[indices[1] as usize],
                mesh.vertices[indices[2] as usize],
            ];
            let bounds = egui::Rect::from_points(&[v[0].pos, v[1].pos, v[2].pos])
                .intersect(clipped.clip_rect)
                .intersect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                ));
            let area = cross(v[1].pos - v[0].pos, v[2].pos - v[0].pos);
            if area.abs() < 0.00001 {
                continue;
            }
            for y in bounds.min.y.max(0.0) as u32..(bounds.max.y.ceil() as u32).min(height) {
                for x in bounds.min.x.max(0.0) as u32..(bounds.max.x.ceil() as u32).min(width) {
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let a = cross(v[1].pos - p, v[2].pos - p) / area;
                    let b = cross(v[2].pos - p, v[0].pos - p) / area;
                    let c = 1.0 - a - b;
                    if a < 0.0 || b < 0.0 || c < 0.0 {
                        continue;
                    }
                    let uv = v[0].uv.to_vec2() * a + v[1].uv.to_vec2() * b + v[2].uv.to_vec2() * c;
                    let tx = ((uv.x * texture.size[0] as f32) as usize).min(texture.size[0] - 1);
                    let ty = ((uv.y * texture.size[1] as f32) as usize).min(texture.size[1] - 1);
                    let texel = texture.pixels[ty * texture.size[0] + tx].to_array();
                    let mut color = [0.0; 4];
                    for channel in 0..4 {
                        color[channel] = (f32::from(v[0].color[channel]) * a
                            + f32::from(v[1].color[channel]) * b
                            + f32::from(v[2].color[channel]) * c)
                            * f32::from(texel[channel])
                            / 255.0;
                    }
                    let pixel = canvas.get_pixel_mut(x, y);
                    for channel in 0..3 {
                        pixel[channel] = (color[channel]
                            + f32::from(pixel[channel]) * (1.0 - color[3] / 255.0))
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    canvas
}
fn cross(a: egui::Vec2, b: egui::Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}
