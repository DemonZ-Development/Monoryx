use egui::{
    epaint::{Mesh, Vertex},
    Color32, Painter, Pos2, Rect, Response, Shape, TextureHandle, Ui,
};

const GRID: usize = 4;
const CAMERA_DISTANCE: f32 = 45.0;
const PERSPECTIVE: f32 = 0.2;
const CENTER_Z: f32 = 2.0;

pub struct SkinViewer {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub slim: bool,
    pub overlay: bool,
    pub show_cape: bool,
    pub elytra: bool,
}

impl Default for SkinViewer {
    fn default() -> Self {
        Self {
            yaw: -0.45,
            pitch: 0.15,
            zoom: 1.0,
            slim: false,
            overlay: true,
            show_cape: true,
            elytra: false,
        }
    }
}

struct Face {
    corners: [[f32; 3]; 4],
    uv: [f32; 4],
    cape: bool,
    rotated: [[f32; 3]; 4],
    depth: f32,
}

impl Face {
    fn new(corners: [[f32; 3]; 4], uv: [f32; 4], cape: bool) -> Self {
        Self {
            corners,
            uv,
            cape,
            rotated: corners,
            depth: 0.0,
        }
    }
}

impl SkinViewer {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn render(
        &mut self,
        ui: &Ui,
        rect: Rect,
        response: &Response,
        skin: &TextureHandle,
        cape: Option<&TextureHandle>,
    ) {
        if response.dragged() {
            let delta = response.drag_delta();

            self.yaw += delta.x * 0.01;
            self.pitch = (self.pitch + delta.y * 0.01).clamp(-1.4, 1.4);
        }

        if response.hovered() {
            let scroll = ui
                .input_mut(|input| std::mem::take(&mut input.smooth_scroll_delta))
                .y;

            self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.35, 3.0);
        }

        let arm = if self.slim { 3.0 } else { 4.0 };

        let parts = [
            (
                [-4.0, -16.0, -2.0],
                [4.0, -8.0, 6.0],
                [0.0, 0.0],
                [32.0, 0.0],
                0.5,
            ),
            (
                [-4.0, -8.0, 0.0],
                [4.0, 4.0, 4.0],
                [16.0, 16.0],
                [16.0, 32.0],
                0.25,
            ),
            (
                [-4.0 - arm, -8.0, 0.0],
                [-4.0, 4.0, 4.0],
                [40.0, 16.0],
                [40.0, 32.0],
                0.25,
            ),
            (
                [4.0, -8.0, 0.0],
                [4.0 + arm, 4.0, 4.0],
                [32.0, 48.0],
                [48.0, 48.0],
                0.25,
            ),
            (
                [-4.0, 4.0, 0.0],
                [0.0, 16.0, 4.0],
                [0.0, 16.0],
                [0.0, 32.0],
                0.25,
            ),
            (
                [0.0, 4.0, 0.0],
                [4.0, 16.0, 4.0],
                [16.0, 48.0],
                [0.0, 48.0],
                0.25,
            ),
        ];

        let mut faces = Vec::with_capacity(96);

        for (min, max, base_uv, overlay_uv, inflate) in parts {
            Self::part(&mut faces, min, max, base_uv, 0.0);

            if self.overlay {
                Self::part(&mut faces, min, max, overlay_uv, inflate);
            }
        }

        if let Some(texture) = cape.filter(|_| self.show_cape) {
            let [width, height] = texture.size();

            if !self.elytra && width >= 22 && height >= 17 {
                faces.push(Face::new(
                    [
                        [5.0, -8.0, -1.0],
                        [-5.0, -8.0, -1.0],
                        [-5.0, 8.0, -4.0],
                        [5.0, 8.0, -4.0],
                    ],
                    [1.0, 1.0, 11.0, 17.0],
                    true,
                ));
                faces.push(Face::new(
                    [
                        [-5.0, -8.0, -1.0],
                        [5.0, -8.0, -1.0],
                        [5.0, 8.0, -4.0],
                        [-5.0, 8.0, -4.0],
                    ],
                    [12.0, 1.0, 22.0, 17.0],
                    true,
                ));
            }

            if self.elytra && width >= 46 && height >= 22 {
                for side in [1.0_f32, -1.0] {
                    let inner_top = [side, -8.0, -1.5];
                    let outer_top = [side * 11.0, -8.0, -1.5];
                    let outer_bottom = [side * 13.0, 12.0, -5.0];
                    let inner_bottom = [side * 3.0, 12.0, -5.0];

                    let mut outer = [outer_top, inner_top, inner_bottom, outer_bottom];
                    let mut inner = [inner_top, outer_top, outer_bottom, inner_bottom];
                    let mut outer_u = [36.0, 46.0];
                    let mut inner_u = [24.0, 34.0];

                    if side < 0.0 {
                        std::mem::swap(&mut outer, &mut inner);
                        outer_u.reverse();
                        inner_u.reverse();
                    }

                    faces.push(Face::new(outer, [outer_u[0], 2.0, outer_u[1], 22.0], true));
                    faces.push(Face::new(inner, [inner_u[0], 2.0, inner_u[1], 22.0], true));
                }
            }
        }

        let scale = rect.height() / 42.0 * self.zoom;

        for face in &mut faces {
            face.rotated = face.corners.map(|corner| self.rotate(corner));
            face.depth = face.rotated.iter().map(|corner| corner[2]).sum::<f32>() / 4.0;
        }

        faces.retain(|face| {
            let points = face
                .rotated
                .map(|corner| Self::project(corner, rect, scale));

            let area: f32 = (0..4)
                .map(|index| {
                    let current = points[index];
                    let next = points[(index + 1) % 4];

                    current.x * next.y - next.x * current.y
                })
                .sum();

            area > 0.0
        });

        faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));

        let painter = ui.painter_at(rect);

        for face in &faces {
            let texture = if face.cape {
                cape.unwrap_or(skin)
            } else {
                skin
            };

            Self::draw_face(&painter, rect, scale, texture, face);
        }
    }

    fn part(faces: &mut Vec<Face>, min: [f32; 3], max: [f32; 3], uv: [f32; 2], inflate: f32) {
        let [width, height, depth] = [0, 1, 2].map(|axis| (max[axis] - min[axis]).round());
        let [x0, y0, z0] = min.map(|value| value - inflate);
        let [x1, y1, z1] = max.map(|value| value + inflate);

        let [u, v] = uv;
        let u1 = u + depth;
        let u2 = u1 + width;
        let u3 = u2 + depth;
        let u4 = u3 + width;
        let v1 = v + depth;
        let v2 = v1 + height;

        faces.extend([
            Face::new(
                [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                [u1, v1, u2, v2],
                false,
            ),
            Face::new(
                [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
                [u3, v1, u4, v2],
                false,
            ),
            Face::new(
                [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                [u, v1, u1, v2],
                false,
            ),
            Face::new(
                [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
                [u2, v1, u3, v2],
                false,
            ),
            Face::new(
                [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                [u1, v, u2, v1],
                false,
            ),
            Face::new(
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                [u2, v, u2 + width, v1],
                false,
            ),
        ]);
    }

    fn rotate(&self, point: [f32; 3]) -> [f32; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();

        let local_z = point[2] - CENTER_Z;

        let x = point[0] * cy + local_z * sy;
        let z = -point[0] * sy + local_z * cy;

        [x, point[1] * cp + z * sp, -point[1] * sp + z * cp]
    }

    fn project(point: [f32; 3], rect: Rect, scale: f32) -> Pos2 {
        let factor = scale * CAMERA_DISTANCE / (CAMERA_DISTANCE - point[2] * PERSPECTIVE);

        Pos2::new(
            rect.center().x + point[0] * factor,
            rect.center().y + point[1] * factor,
        )
    }

    fn draw_face(painter: &Painter, rect: Rect, scale: f32, texture: &TextureHandle, face: &Face) {
        let [width, height] = texture.size();
        let unit = (width / 64).max(1) as f32;
        let steps = GRID as f32;

        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let mix = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
            std::array::from_fn(|axis| lerp(a[axis], b[axis], t))
        };

        let mut mesh = Mesh::with_texture(texture.id());

        for row in 0..=GRID {
            let t = row as f32 / steps;

            for column in 0..=GRID {
                let s = column as f32 / steps;

                let top = mix(face.rotated[0], face.rotated[1], s);
                let bottom = mix(face.rotated[3], face.rotated[2], s);
                let point = mix(top, bottom, t);

                let u = lerp(face.uv[0], face.uv[2], s) * unit / width as f32;
                let v = lerp(face.uv[1], face.uv[3], t) * unit / height as f32;

                mesh.vertices.push(Vertex {
                    pos: Self::project(point, rect, scale),
                    uv: Pos2::new(u, v),
                    color: Color32::WHITE,
                });
            }
        }

        let stride = (GRID + 1) as u32;

        for row in 0..GRID as u32 {
            for column in 0..GRID as u32 {
                let a = row * stride + column;
                let b = a + 1;
                let d = a + stride;
                let c = d + 1;

                mesh.add_triangle(a, b, c);
                mesh.add_triangle(a, c, d);
            }
        }

        painter.add(Shape::mesh(mesh));
    }
}
