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
            "home-halloween",
            Page::Home,
            1024,
            576,
            ThemeKind::Halloween,
            false,
        ),
        (
            "discord",
            Page::Settings,
            1280,
            900,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "home-medium",
            Page::Home,
            1024,
            576,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "home-long-name",
            Page::Home,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "home-empty",
            Page::Home,
            1024,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "home-no-screenshot",
            Page::Home,
            1280,
            720,
            ThemeKind::Halloween,
            false,
        ),
        (
            "discover-small",
            Page::Discover,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "discover-wide",
            Page::Discover,
            1536,
            816,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "discover-project-small",
            Page::Discover,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "downloads-active",
            Page::Downloads,
            850,
            650,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "library-small",
            Page::Library,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "home-updates-small",
            Page::Home,
            850,
            650,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "library-updates-small",
            Page::Library,
            850,
            650,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "library-wide",
            Page::Library,
            1536,
            816,
            ThemeKind::Monochrome,
            false,
        ),
        (
            "instances-small",
            Page::Instances,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "new-instance-small",
            Page::Instances,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "new-loader-small",
            Page::Instances,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "onboarding-intro",
            Page::Onboarding,
            1024,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "onboarding-account",
            Page::Onboarding,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "onboarding-defaults",
            Page::Onboarding,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "settings-small",
            Page::Settings,
            850,
            560,
            ThemeKind::Halloween,
            false,
        ),
        (
            "settings-appearance",
            Page::Settings,
            1024,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "settings-minecraft",
            Page::Settings,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "settings-runtime",
            Page::Settings,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "accounts-small",
            Page::Accounts,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "downloads-small",
            Page::Downloads,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "screenshots-small",
            Page::Screenshots,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
        (
            "nexeu-small",
            Page::Nexeu,
            850,
            650,
            ThemeKind::Halloween,
            false,
        ),
    ] {
        let is_halloween = label == "home-halloween";
        let ctx = egui::Context::default();
        let temp = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(temp.path().into());
        let manager = crate::instance::InstanceManager::new(paths.clone());
        let mut first = if is_halloween {
            let mut inst = manager
                .create(
                    "THE BEST LAUNCHERRR".into(),
                    "1.20.1".into(),
                    crate::instance::LoaderKind::Fabric,
                    "0.15.11".into(),
                )
                .unwrap();
            inst.memory_max_mb = 4000;
            inst.last_played_at = Some("Fri at 10:18 PM".into());
            manager.save(&inst).unwrap();
            inst
        } else {
            let inst = manager
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
            inst
        };

        if label == "home-long-name" {
            first.name = "Survival with friends - shaders and adventures".into();
            manager.save(&first).unwrap();
        }
        if label.contains("updates-small") {
            first.loader_version = "0.16.9".into();
            manager.save(&first).unwrap();
        }

        if label == "discover-wide" || page == Page::Library || label == "home-updates-small" {
            first.resolved_version_id = "fabric-1.21.1".into();
            let jar = crate::instance::readiness::client_jar_for(&paths, &first).unwrap();
            std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
            std::fs::write(jar, b"review fixture").unwrap();
            manager.save(&first).unwrap();
        }
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.page = page;
        state.config.profile = if is_halloween {
            Some(crate::account::offline::OfflineProfile::new("Cyrusbye720").unwrap())
        } else {
            Some(crate::account::offline::OfflineProfile::new("PreviewPlayer").unwrap())
        };
        state.config.theme = theme;
        crate::ui::theme::apply_selected_theme(&ctx, theme);
        state.selected_instance = Some(first.id.clone());
        state
            .mod_counts
            .insert(first.id.clone(), if is_halloween { 4 } else { 12 });
        if is_halloween {
            let shot_files = [
                (
                    "shot_main.png",
                    "C:/Users/satya/projects/Monoryx/target/ref_crops/shot_main.png",
                ),
                (
                    "shot_1.png",
                    "C:/Users/satya/projects/Monoryx/target/ref_crops/shot_instance.png",
                ),
                (
                    "shot_2.png",
                    "C:/Users/satya/projects/Monoryx/target/ref_crops/shot_2.png",
                ),
                (
                    "shot_3.png",
                    "C:/Users/satya/projects/Monoryx/target/ref_crops/shot_3.png",
                ),
                (
                    "shot_4.png",
                    "C:/Users/satya/projects/Monoryx/target/ref_crops/shot_4.png",
                ),
            ];
            for (idx, (fname, crop_path_str)) in shot_files.iter().enumerate() {
                let p = temp.path().join(fname);
                let texture = if std::path::Path::new(crop_path_str).exists() {
                    let img = image::open(crop_path_str).unwrap().to_rgba8();
                    let (w, h) = img.dimensions();
                    ctx.load_texture(
                        format!("halloween-shot-{idx}"),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            &img.into_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    )
                } else {
                    let pixels = vec![egui::Color32::from_rgb(180, 80, 20); 120 * 80];
                    ctx.load_texture(
                        format!("halloween-shot-{idx}"),
                        egui::ColorImage::new([120, 80], pixels),
                        egui::TextureOptions::LINEAR,
                    )
                };
                state.screenshot_thumbnails.insert(p.clone(), texture);
                state
                    .screenshots
                    .push(crate::app::screenshots::ScreenshotEntry {
                        path: p,
                        instance_id: first.id.clone(),
                        instance_name: first.name.clone(),
                        modified: std::time::SystemTime::UNIX_EPOCH
                            + std::time::Duration::from_secs(1_790_261_100),
                        bytes: 2048,
                    });
            }
            for overflow_idx in 5..15 {
                let p = temp.path().join(format!("overflow_{overflow_idx}.png"));
                state
                    .screenshots
                    .push(crate::app::screenshots::ScreenshotEntry {
                        path: p,
                        instance_id: first.id.clone(),
                        instance_name: first.name.clone(),
                        modified: std::time::SystemTime::UNIX_EPOCH
                            + std::time::Duration::from_secs(1_790_261_100),
                        bytes: 1024,
                    });
            }
        } else {
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
        }
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
                instance_id: first.id.clone(),
                instance_name: first.name.clone(),
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

        if label == "discord" {
            crate::ui::pages::settings::select_discord_for_preview(&ctx);
        } else if page == Page::Settings {
            crate::ui::pages::settings::select_review_tab(&ctx, label);
        }
        if label == "home-empty" {
            state.instance_list.clear();
            state.selected_instance = None;
        }
        if label == "home-no-screenshot" {
            state.screenshots.clear();
        }
        if page == Page::Onboarding {
            state.config.profile = None;
            state.onboarding_user = "PreviewPlayer".into();
            state.onboarding_step = match label {
                "onboarding-account" => 1,
                "onboarding-defaults" => 2,
                _ => 0,
            };
            if label == "onboarding-defaults" {
                state.onboarding_mem_auto = false;
            }
        }
        if label.starts_with("new-") {
            state.show_new_instance = true;
            state.new_draft.name = "Survival with friends".into();
            state.new_draft.version = "1.21.1".into();
            state.new_draft.versions = vec!["1.21.1".into(), "1.20.1".into(), "1.19.4".into()];
            state.new_draft.step = if label == "new-loader-small" { 1 } else { 0 };
        }

        if label == "new-loader-small" {
            state.new_draft.loader = crate::instance::LoaderKind::Fabric;
            state.new_draft.loader_version = "0.16.14".into();
            state.new_draft.loader_versions = vec!["0.16.14".into(), "0.16.13".into()];
            state.new_draft.loader_fetch_key = "1.21.1::fabric".into();
        }
        if page == Page::Downloads {
            state
                .downloads_history
                .push(crate::app::state::TrackedDownload {
                    id: "mod-install".into(),
                    label: "Installing Iris Shaders".into(),
                    downloaded: 100000,
                    total: Some(1250000),
                    speed_bps: 0.0,
                    state: "failed".into(),
                    message:
                        "Could not reach the download server. Check your connection and try again."
                            .into(),
                });
        }
        if page == Page::Downloads {
            state.retry_actions.insert(
                "mod-install".into(),
                crate::app::state::RetryAction::Game(first.id.clone()),
            );
            if label == "downloads-active" {
                let id = format!("game:{}", first.id);
                state.start_operation(
                    &id,
                    "Preparing Minecraft for Survival with friends".into(),
                    Some(first.id.clone()),
                    crate::app::state::RetryAction::Game(first.id.clone()),
                );
                state.handle_event(
                    crate::app::events::AppEvent::InstallProgress(
                        id,
                        "Downloading game files".into(),
                        14,
                        42,
                    ),
                    &ctx,
                );
                state.downloads.insert(
                    "client".into(),
                    crate::app::state::TrackedDownload {
                        id: "client".into(),
                        label: "Minecraft client".into(),
                        downloaded: 12_000_000,
                        total: Some(32_000_000),
                        speed_bps: 2_000_000.0,
                        state: "downloading".into(),
                        message: String::new(),
                    },
                );
            }
        }
        if page == Page::Discover {
            state.search.loader = "fabric".into();
            state.search.game_version = "1.21.1".into();
            state.search_total = 4;
            state.search_results = ["Sodium", "Fabric API", "Iris Shaders", "A mod with a long title for the narrow layout"]
                .into_iter().enumerate().map(|(index, name)| serde_json::from_value(serde_json::json!({
                    "slug": format!("review-{index}"), "title": name,
                    "description": "A Minecraft mod that improves performance and adds useful features to your world.",
                    "author": "Example author", "downloads": 12345678, "project_type": "mod",
                    "categories": ["fabric", "optimization"], "versions": ["1.21.1"]
                })).unwrap()).collect();
        }
        if label == "discover-project-small" {
            state.detail_slug = Some("review-0".into());
            state.detail_project = Some(serde_json::from_value(serde_json::json!({"slug": "review-0", "title": "Sodium", "description": "Improve Minecraft rendering performance.", "project_type": "mod", "body": "A performance mod for Minecraft."})).unwrap());
            state.detail_versions = ["1.21.1", "1.20.1"].into_iter().map(|game| serde_json::from_value(serde_json::json!({"id": game, "project_id": "review-0", "name": game, "version_number": game, "files": [], "dependencies": [], "game_versions": [game], "loaders": ["fabric"]})).unwrap()).collect();
        }
        if page == Page::Library {
            state.library_entries = [
                "Sodium",
                "Fabric API",
                "Iris Shaders",
                "Example long mod name with additional details",
            ]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                serde_json::from_value(serde_json::json!({
                    "file_name": format!("example-{index}-1.21.1.jar"), "kind": "mod",
                    "project_slug": format!("review-{index}"), "project_title": name,
                    "version_number": "0.16.14+1.21.1", "size": 1250000,
                    "enabled": index != 2, "installed_at": "2026-10-06T10:00:00Z",
                    "loader": "fabric", "game_version": "1.21.1"
                }))
                .unwrap()
            })
            .collect();
        }

        if label.contains("updates-small") {
            state.updates_instance = Some(first.id.clone());
            state.updates_checked = true;
            state.updates_summary =
                "1 compatible package update is available for Minecraft 1.21.1.".into();
            state.updates = vec![crate::modrinth::updates::UpdateInfo {
                file_name: "example-1-1.21.1.jar".into(),
                kind: crate::content::ContentKind::Mod,
                project_id: "fabric-api".into(),
                title: "Fabric API".into(),
                current_version: "0.100".into(),
                new_version: "0.101".into(),
                new_version_id: "review-new-version".into(),
            }];
            state.loader_update_instance = Some(first.id.clone());
            state.loader_update_candidate = Some((first.id.clone(), "0.16.14".into()));
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
                let rect = text.visual_bounding_rect();
                if rect.intersects(shape.clip_rect) {
                    assert!(
                        rect.right() <= width as f32 + 1.0,
                        "{label}: {:?} extends outside the window: {rect:?}",
                        text.galley.job.text
                    );
                }
            }
        }
        if page == Page::Onboarding {
            let button = if label == "onboarding-defaults" {
                "Open MONORYX"
            } else {
                "Continue"
            };
            let (rect, clip) = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == button => {
                        Some((text.visual_bounding_rect(), shape.clip_rect))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{label}: missing {button}"));
            assert!(
                clip.expand(1.0).contains_rect(rect),
                "{label}: {button} is clipped"
            );
            assert!(
                rect.bottom() < height as f32 && rect.top() >= 0.0,
                "{label}: {button} is outside the window"
            );
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
    let mut canvas = image::RgbaImage::from_pixel(width, height, image::Rgba([13, 14, 16, 255]));
    for clipped in ctx.tessellate(output.shapes, 1.0) {
        let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let texture = &textures[&mesh.texture_id];
        for indices in mesh.indices.as_chunks::<3>().0 {
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
