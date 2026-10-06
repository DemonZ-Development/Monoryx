use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};

const VERTEX: &str = "#version 330 core
out vec2 uv;
void main() {
    vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    uv = p;
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}";

const FRAGMENT: &str = "#version 330 core
in vec2 uv;
out vec4 color;
uniform sampler2D image;
uniform vec2 axis;
void main() {
    if (axis == vec2(0.0)) {
        color = texture(image, uv);
    } else {
        color = texture(image, uv) * 0.227027;
        color += texture(image, uv + axis * 1.384615) * 0.316216;
        color += texture(image, uv - axis * 1.384615) * 0.316216;
        color += texture(image, uv + axis * 3.230769) * 0.070270;
        color += texture(image, uv - axis * 3.230769) * 0.070270;
    }
}";

struct Resources {
    gl: Arc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    textures: Vec<glow::Texture>,
    framebuffers: Vec<glow::Framebuffer>,
    size: [i32; 2],
    screen: [u32; 2],
}

impl Resources {
    unsafe fn create(gl: Arc<glow::Context>, screen: [u32; 2]) -> Result<Self, String> {
        unsafe {
            let program = gl.create_program()?;
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, FRAGMENT),
            ] {
                let shader = match gl.create_shader(kind) {
                    Ok(shader) => shader,
                    Err(error) => {
                        gl.delete_program(program);
                        return Err(error);
                    }
                };
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let error = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    gl.delete_program(program);
                    return Err(error);
                }
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let error = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(error);
            }
            let vao = match gl.create_vertex_array() {
                Ok(vao) => vao,
                Err(error) => {
                    gl.delete_program(program);
                    return Err(error);
                }
            };
            let scale = (512.0 / screen[0].max(screen[1]).max(1) as f32).min(1.0);
            let size = [
                (screen[0] as f32 * scale).round().max(1.0) as i32,
                (screen[1] as f32 * scale).round().max(1.0) as i32,
            ];
            let mut resources = Self {
                gl,
                program,
                vao,
                textures: Vec::with_capacity(2),
                framebuffers: Vec::with_capacity(2),
                size,
                screen,
            };
            for _ in 0..2 {
                let texture = resources.gl.create_texture()?;
                resources.textures.push(texture);
                resources.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                    resources.gl.tex_parameter_i32(
                        glow::TEXTURE_2D,
                        parameter,
                        glow::LINEAR as i32,
                    );
                }
                for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                    resources.gl.tex_parameter_i32(
                        glow::TEXTURE_2D,
                        parameter,
                        glow::CLAMP_TO_EDGE as i32,
                    );
                }
                resources.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    size[0],
                    size[1],
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                let framebuffer = resources.gl.create_framebuffer()?;
                resources.framebuffers.push(framebuffer);
                resources
                    .gl
                    .bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                resources.gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0,
                );
                if resources.gl.check_framebuffer_status(glow::FRAMEBUFFER)
                    != glow::FRAMEBUFFER_COMPLETE
                {
                    return Err("Blur framebuffer unavailable".into());
                }
            }
            Ok(resources)
        }
    }

    unsafe fn draw(&self, texture: usize, axis: [f32; 2]) {
        unsafe {
            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.active_texture(glow::TEXTURE0);
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.textures[texture]));
            self.gl.uniform_1_i32(
                self.gl.get_uniform_location(self.program, "image").as_ref(),
                0,
            );
            self.gl.uniform_2_f32(
                self.gl.get_uniform_location(self.program, "axis").as_ref(),
                axis[0],
                axis[1],
            );
            self.gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            for framebuffer in &self.framebuffers {
                self.gl.delete_framebuffer(*framebuffer);
            }
            for texture in &self.textures {
                self.gl.delete_texture(*texture);
            }
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_program(self.program);
        }
    }
}

#[derive(Default)]
struct Backdrop {
    resources: Option<Resources>,
    unavailable: bool,
    active: bool,
}

pub fn paint(ctx: &egui::Context, painter: &egui::Painter, active: bool) {
    let id = egui::Id::new("dialog-blur-resources");
    let shared = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Arc<Mutex<Backdrop>>>(id)
            .clone()
    });
    let was_active = shared.lock().is_ok_and(|mut backdrop| {
        let was_active = backdrop.active;
        backdrop.active = active;
        was_active
    });
    if !active && !was_active {
        return;
    }
    painter.add(egui::PaintCallback {
        rect: ctx.screen_rect(),
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, painter| {
            let Ok(mut backdrop) = shared.lock() else {
                return;
            };
            let gl = painter.gl();
            unsafe {
                if !active {
                    backdrop.resources = None;
                    return;
                }
                gl.disable(glow::SCISSOR_TEST);
                gl.disable(glow::BLEND);
                gl.disable(glow::DEPTH_TEST);
                gl.disable(glow::CULL_FACE);
                let refresh = backdrop
                    .resources
                    .as_ref()
                    .is_none_or(|resources| resources.screen != info.screen_size_px);
                if refresh && !backdrop.unavailable {
                    backdrop.resources = None;
                    match Resources::create(gl.clone(), info.screen_size_px) {
                        Ok(resources) => backdrop.resources = Some(resources),
                        Err(error) => {
                            backdrop.unavailable = true;
                            tracing::debug!("Dialog blur unavailable: {error}");
                        }
                    }
                    if let Some(resources) = &backdrop.resources {
                        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, painter.intermediate_fbo());
                        gl.bind_framebuffer(
                            glow::DRAW_FRAMEBUFFER,
                            Some(resources.framebuffers[0]),
                        );
                        gl.blit_framebuffer(
                            0,
                            0,
                            info.screen_size_px[0] as i32,
                            info.screen_size_px[1] as i32,
                            0,
                            0,
                            resources.size[0],
                            resources.size[1],
                            glow::COLOR_BUFFER_BIT,
                            glow::LINEAR,
                        );
                        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.framebuffers[1]));
                        gl.viewport(0, 0, resources.size[0], resources.size[1]);
                        resources.draw(0, [1.0 / resources.size[0] as f32, 0.0]);
                        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resources.framebuffers[0]));
                        resources.draw(1, [0.0, 1.0 / resources.size[1] as f32]);
                    }
                }
                gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
                gl.viewport(
                    0,
                    0,
                    info.screen_size_px[0] as i32,
                    info.screen_size_px[1] as i32,
                );
                if let Some(resources) = &backdrop.resources {
                    resources.draw(0, [0.0, 0.0]);
                }
            }
        })),
    });
}

pub fn destroy(ctx: &egui::Context) {
    if let Some(shared) = ctx.data_mut(|data| {
        data.remove_temp::<Arc<Mutex<Backdrop>>>(egui::Id::new("dialog-blur-resources"))
    }) {
        if let Ok(mut backdrop) = shared.lock() {
            backdrop.resources = None;
        }
    }
}
