use crate::view3d::Camera;
use egui::{pos2, Align2, Color32, FontId, Pos2, Rect, Stroke, Ui};

/* ---------- Types ---------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitKind {
    None,
    Face(AxisFace),
    Edge([AxisFace; 2]),
    Corner([AxisFace; 3]),
}

/* ---------- Theme ---------- */
const COL_BG: Color32 = Color32::from_rgb(28, 28, 30);
const COL_FACE: Color32 = Color32::from_rgb(45, 45, 48); // тёмно-серые грани
const COL_FACE_BACK: Color32 = Color32::from_rgb(38, 38, 40); // дальние чуть темнее
const COL_OUTLINE: Color32 = Color32::from_rgb(18, 18, 18); // обводка граней
const COL_EDGE: Color32 = Color32::from_rgb(64, 64, 70); // рёбра
const COL_EDGE_BG: Color32 = Color32::from_rgb(16, 16, 16); // подложка под ребро
const COL_CORNER: Color32 = Color32::from_rgb(72, 72, 78); // углы
const COL_TEXT: Color32 = Color32::from_rgb(235, 238, 241);

const COL_HOVER: Color32 = Color32::from_rgb(90, 160, 230); // голубой hover
const COL_HOVER_2: Color32 = Color32::from_rgb(120, 185, 245); // актив

/* ---------- Плавная анимация поворота ---------- */
#[derive(Clone, Copy)]
struct TurnAnim {
    start_yaw: f32,
    start_pitch: f32,
    end_yaw: f32,
    end_pitch: f32,
    lock_px: Option<Pos2>, // экранная позиция pivot, которую держим
    t: f32,                // 0..1
    dur: f32,              // сек
}

/* ---------- ViewCube ---------- */

pub struct ViewCube {
    pub size: f32,
    pub margin: f32,
    hover: HitKind,
    anim: Option<TurnAnim>,
}

impl Default for ViewCube {
    fn default() -> Self {
        Self {
            size: 100.0,
            margin: 12.0,
            hover: HitKind::None,
            anim: None,
        }
    }
}

impl ViewCube {
    pub fn ui(&mut self, ui: &mut Ui, rect_full: Rect, cam: &mut Camera) {
        // правый-нижний угол
        let outer = Rect::from_min_size(
            pos2(
                rect_full.max.x - self.size - self.margin,
                rect_full.max.y - self.size - self.margin,
            ),
            egui::vec2(self.size, self.size),
        );

        // отступы, чтобы не клипалось
        let edge_th = (outer.width() * 0.14).clamp(4.0, 7.0);
        let corner_px = (outer.width() * 0.18).clamp(6.0, 12.0);
        let pad = 2.0 + 0.5 * edge_th + 0.5 * corner_px + 1.0;

        let content = outer.shrink(pad);
        let resp = ui.allocate_rect(outer, egui::Sense::click());
        let painter = ui.painter_at(outer);

        // фон под куб
        painter.rect_filled(outer, 4.0, COL_BG);

        // шаг анимации (если идёт)
        if let Some(anim) = &mut self.anim {
            let dt = ui.input(|i| i.stable_dt).max(1.0 / 240.0);
            anim.t = (anim.t + dt / anim.dur).min(1.0);

            let yaw = lerp_angle(anim.start_yaw, anim.end_yaw, anim.t);
            let pitch = lerp_linear(anim.start_pitch, anim.end_pitch, anim.t);

            if let Some(s0) = anim.lock_px {
                cam.reorient_around_pivot(yaw, pitch);
                if let Some(s1) = cam.world_to_screen(rect_full, cam.pivot) {
                    let dd = s1 - s0;
                    if dd.length_sq() > 0.0 {
                        let (dx, dy, dz) = cam.screen_delta_to_world_pan(rect_full, dd.x, dd.y);
                        cam.center.x -= dx;
                        cam.center.y -= dy;
                        cam.center.z -= dz;
                    }
                }
                ui.ctx().request_repaint();
            } else {
                cam.reorient_around_pivot(yaw, pitch);
            }

            if anim.t >= 1.0 {
                self.anim = None;
            }
        }

        self.draw_cube(&painter, content, cam, edge_th, corner_px);

        // hover / click
        let pointer = resp
            .hover_pos()
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));
        self.hover = HitKind::None;
        if let Some(p) = pointer {
            if outer.contains(p) {
                self.hover = self.pick(content, p, cam);
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if outer.contains(p) {
                    let hit = self.pick(content, p, cam);
                    self.start_turn_animation(cam, rect_full, hit); // плавное вращение
                }
            }
        }
    }

    fn draw_cube(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        cam: &Camera,
        edge_th: f32,
        corner_px: f32,
    ) {
        let (r, u, f) = cam.axes();

        // проекция [-1,1]^3 в экранных осях (r/u)
        let s = rect.width() * 0.33;
        let c = rect.center();
        let to2 = |(x, y, z): (f32, f32, f32)| -> Pos2 {
            let sx = x * r.0 + y * r.1 + z * r.2;
            let sy = x * u.0 + y * u.1 + z * u.2;
            pos2(c.x + s * sx, c.y - s * sy)
        };

        // вершины
        let v = [
            to2((-1.0, -1.0, -1.0)),
            to2((1.0, -1.0, -1.0)),
            to2((1.0, 1.0, -1.0)),
            to2((-1.0, 1.0, -1.0)),
            to2((-1.0, -1.0, 1.0)),
            to2((1.0, -1.0, 1.0)),
            to2((1.0, 1.0, 1.0)),
            to2((-1.0, 1.0, 1.0)),
        ];

        // грани (исправлены индексы!)
        struct Face {
            idx: [usize; 4],
            n: (f32, f32, f32),
            face: AxisFace,
        }
        let faces = [
            Face {
                idx: [0, 1, 2, 3],
                n: (0.0, 0.0, -1.0),
                face: AxisFace::NegZ,
            }, // XY-
            Face {
                idx: [4, 5, 6, 7],
                n: (0.0, 0.0, 1.0),
                face: AxisFace::PosZ,
            }, // XY+
            Face {
                idx: [1, 2, 6, 5],
                n: (1.0, 0.0, 0.0),
                face: AxisFace::PosX,
            }, // YZ+
            Face {
                idx: [0, 3, 7, 4],
                n: (-1.0, 0.0, 0.0),
                face: AxisFace::NegX,
            }, // YZ-
            Face {
                idx: [3, 2, 6, 7],
                n: (0.0, 1.0, 0.0),
                face: AxisFace::PosY,
            }, // XZ+
            Face {
                idx: [0, 1, 5, 4],
                n: (0.0, -1.0, 0.0),
                face: AxisFace::NegY,
            }, // XZ-
        ];

        // видимость граней: фронтальные d >= 0.0
        let mut back: Vec<&Face> = Vec::new();
        let mut front: Vec<&Face> = Vec::new();
        let mut face_dot: [f32; 6] = [0.0; 6];

        for (i, fdef) in faces.iter().enumerate() {
            let d = dot(fdef.n, f);
            face_dot[i] = d;
            if d >= 0.0 {
                front.push(fdef);
            } else {
                back.push(fdef);
            }
        }
        let mut sort_depth = |arr: &mut Vec<&Face>| {
            arr.sort_by(|a, b| dot(a.n, f).partial_cmp(&dot(b.n, f)).unwrap());
        };
        sort_depth(&mut back);
        sort_depth(&mut front);

        // 1) дальние грани
        for fd in &back {
            let poly = [v[fd.idx[0]], v[fd.idx[1]], v[fd.idx[2]], v[fd.idx[3]]];
            painter.add(egui::Shape::convex_polygon(
                poly.to_vec(),
                COL_FACE_BACK,
                Stroke {
                    width: 1.0,
                    color: COL_OUTLINE,
                },
            ));
        }

        // 2) фронтальные грани; hover — голубым
        for fd in &front {
            let poly = [v[fd.idx[0]], v[fd.idx[1]], v[fd.idx[2]], v[fd.idx[3]]];

            let mut fill = COL_FACE;
            if matches!(self.hover, HitKind::Face(hf) if hf == fd.face) {
                fill = COL_HOVER;
            }
            painter.add(egui::Shape::convex_polygon(
                poly.to_vec(),
                fill,
                Stroke {
                    width: 1.0,
                    color: COL_OUTLINE,
                },
            ));

            let c2 = centroid(&poly);
            painter.text(
                c2,
                Align2::CENTER_CENTER,
                plane_label(fd.face),
                FontId::monospace(11.0),
                COL_TEXT,
            );
        }

        // 3) рёбра: рисуем, если ХОТЯ БЫ одна соседняя грань фронтальная
        let edge_under = edge_th + 2.0;
        let face_idx = |f: AxisFace| -> usize {
            match f {
                AxisFace::NegZ => 0,
                AxisFace::PosZ => 1,
                AxisFace::PosX => 2,
                AxisFace::NegX => 3,
                AxisFace::PosY => 4,
                AxisFace::NegY => 5,
            }
        };

        // рёбра (исправлены пары граней!)
        let edges = [
            // Z- «дно»
            (0, 1, [AxisFace::NegZ, AxisFace::NegY]),
            (1, 2, [AxisFace::NegZ, AxisFace::PosX]),
            (2, 3, [AxisFace::NegZ, AxisFace::PosY]),
            (3, 0, [AxisFace::NegZ, AxisFace::NegX]),
            // Z+ «крышка»
            (4, 5, [AxisFace::PosZ, AxisFace::NegY]),
            (5, 6, [AxisFace::PosZ, AxisFace::PosX]),
            (6, 7, [AxisFace::PosZ, AxisFace::PosY]),
            (7, 4, [AxisFace::PosZ, AxisFace::NegX]),
            // вертикали
            (0, 4, [AxisFace::NegX, AxisFace::NegY]),
            (1, 5, [AxisFace::PosX, AxisFace::NegY]),
            (2, 6, [AxisFace::PosX, AxisFace::PosY]),
            (3, 7, [AxisFace::NegX, AxisFace::PosY]),
        ];

        for (a, b, ff) in edges {
            let d0 = face_dot[face_idx(ff[0])];
            let d1 = face_dot[face_idx(ff[1])];
            let visible = d0 >= 0.0 || d1 >= 0.0;
            if !visible {
                continue;
            }

            painter.add(egui::Shape::line_segment(
                [v[a], v[b]],
                Stroke {
                    width: edge_under,
                    color: COL_EDGE_BG,
                },
            ));

            let mut col = COL_EDGE;
            if let HitKind::Edge(hff) = self.hover {
                if same_edge(hff, ff) {
                    col = COL_HOVER_2;
                }
            }
            painter.add(egui::Shape::line_segment(
                [v[a], v[b]],
                Stroke {
                    width: edge_th,
                    color: col,
                },
            ));
        }

        // 4) углы — рисуем только видимые (хотя бы одна фронтальная грань)
        for (i, p) in v.iter().enumerate() {
            let triplet = corner_triplet(i);
            let visible = triplet.into_iter().any(|af| face_dot[face_idx(af)] >= 0.0);
            if !visible {
                continue;
            }

            let mut col = COL_CORNER;
            if let HitKind::Corner(hc) = self.hover {
                if same_corner(hc, triplet) {
                    col = COL_HOVER_2;
                }
            }
            // подложка-обводка
            let r_under = Rect::from_center_size(*p, egui::vec2(corner_px + 2.0, corner_px + 2.0));
            painter.rect_filled(r_under, 2.0, COL_EDGE_BG);

            let r = Rect::from_center_size(*p, egui::vec2(corner_px, corner_px));
            painter.rect_filled(r, 2.0, col);
        }
    }

    fn pick(&self, rect: Rect, p: Pos2, cam: &Camera) -> HitKind {
        let (r, u, fwd) = cam.axes();
        let s = rect.width() * 0.33;
        let c = rect.center();
        let to2 = |(x, y, z): (f32, f32, f32)| -> Pos2 {
            let sx = x * r.0 + y * r.1 + z * r.2;
            let sy = x * u.0 + y * u.1 + z * u.2;
            pos2(c.x + s * sx, c.y - s * sy)
        };
        let v = [
            to2((-1.0, -1.0, -1.0)),
            to2((1.0, -1.0, -1.0)),
            to2((1.0, 1.0, -1.0)),
            to2((-1.0, 1.0, -1.0)),
            to2((-1.0, -1.0, 1.0)),
            to2((1.0, -1.0, 1.0)),
            to2((1.0, 1.0, 1.0)),
            to2((-1.0, 1.0, 1.0)),
        ];

        // 1) угол (приоритет)
        let vr = 8.0;
        for (i, pt) in v.iter().enumerate() {
            if (p - *pt).length_sq() <= vr * vr {
                return HitKind::Corner(corner_triplet(i));
            }
        }

        // 2) ребро
        let er = 7.0;
        let edges = [
            (0, 1, [AxisFace::NegZ, AxisFace::NegY]),
            (1, 2, [AxisFace::NegZ, AxisFace::PosX]),
            (2, 3, [AxisFace::NegZ, AxisFace::PosY]),
            (3, 0, [AxisFace::NegZ, AxisFace::NegX]),
            (4, 5, [AxisFace::PosZ, AxisFace::NegY]),
            (5, 6, [AxisFace::PosZ, AxisFace::PosX]),
            (6, 7, [AxisFace::PosZ, AxisFace::PosY]),
            (7, 4, [AxisFace::PosZ, AxisFace::NegX]),
            (0, 4, [AxisFace::NegX, AxisFace::NegY]),
            (1, 5, [AxisFace::PosX, AxisFace::NegY]),
            (2, 6, [AxisFace::PosX, AxisFace::PosY]),
            (3, 7, [AxisFace::NegX, AxisFace::PosY]),
        ];
        for (a, b, faces2) in edges {
            if dist_to_segment(p, v[a], v[b]) <= er {
                return HitKind::Edge(faces2);
            }
        }

        // 3) фронтальные грани
        let faces = [
            ([0, 1, 2, 3], AxisFace::NegZ, (0.0, 0.0, -1.0)),
            ([4, 5, 6, 7], AxisFace::PosZ, (0.0, 0.0, 1.0)),
            ([1, 2, 6, 5], AxisFace::PosX, (1.0, 0.0, 0.0)),
            ([0, 3, 7, 4], AxisFace::NegX, (-1.0, 0.0, 0.0)),
            ([3, 2, 6, 7], AxisFace::PosY, (0.0, 1.0, 0.0)),
            ([0, 1, 5, 4], AxisFace::NegY, (0.0, -1.0, 0.0)),
        ];
        for (idx, face, nrm) in faces {
            if dot(nrm, fwd) < 0.0 {
                continue;
            } // только фронтальные
            let poly = [v[idx[0]], v[idx[1]], v[idx[2]], v[idx[3]]];
            if point_in_quad(p, &poly) {
                return HitKind::Face(face);
            }
        }

        HitKind::None
    }

    fn apply_hit(&mut self, cam: &mut Camera, rect_full: Rect, hit: HitKind) {
        self.start_turn_animation(cam, rect_full, hit);
    }

    fn start_turn_animation(&mut self, cam: &mut Camera, rect_full: Rect, hit: HitKind) {
        use std::f32::consts::PI;

        let (yaw_target, pitch_target) = match hit {
            HitKind::Face(face) => match face {
                AxisFace::PosX => (0.0, 0.0),
                AxisFace::NegX => (PI, 0.0),
                AxisFace::PosY => (PI * 0.5, 0.0),
                AxisFace::NegY => (-PI * 0.5, 0.0),
                AxisFace::PosZ => (cam.yaw, PI * 0.5),
                AxisFace::NegZ => (cam.yaw, -PI * 0.5),
            },
            HitKind::Edge([a, b]) => {
                let dir = face_dir(a) + face_dir(b);
                yaw_pitch_from_dir(dir)
            }
            HitKind::Corner([a, b, c]) => {
                let dir = face_dir(a) + face_dir(b) + face_dir(c);
                yaw_pitch_from_dir(dir)
            }
            HitKind::None => (cam.yaw, cam.pitch),
        };

        self.anim = Some(TurnAnim {
            start_yaw: cam.yaw,
            start_pitch: cam.pitch,
            end_yaw: yaw_target,
            end_pitch: pitch_target,
            lock_px: cam.world_to_screen(rect_full, cam.pivot),
            t: 0.0,
            dur: 0.30,
        });
    }
}

/* ===== utils ===== */

fn dot(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}
fn centroid(p: &[Pos2; 4]) -> Pos2 {
    pos2(
        (p[0].x + p[1].x + p[2].x + p[3].x) / 4.0,
        (p[0].y + p[1].y + p[2].y + p[3].y) / 4.0,
    )
}
fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    (a + t * ab - p).length()
}
fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    #[inline]
    fn cross2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
        ax * by - ay * bx
    }
    let ab = b - a;
    let bc = c - b;
    let ca = a - c;
    let ap = p - a;
    let bp = p - b;
    let cp = p - c;
    let s1 = cross2(ab.x, ab.y, ap.x, ap.y);
    let s2 = cross2(bc.x, bc.y, bp.x, bp.y);
    let s3 = cross2(ca.x, ca.y, cp.x, cp.y);
    (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
}
fn point_in_quad(p: Pos2, q: &[Pos2; 4]) -> bool {
    point_in_triangle(p, q[0], q[1], q[2]) || point_in_triangle(p, q[0], q[2], q[3])
}

#[derive(Clone, Copy)]
struct Vec3(f32, f32, f32);
impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

fn face_dir(f: AxisFace) -> Vec3 {
    match f {
        AxisFace::PosX => Vec3(1.0, 0.0, 0.0),
        AxisFace::NegX => Vec3(-1.0, 0.0, 0.0),
        AxisFace::PosY => Vec3(0.0, 1.0, 0.0),
        AxisFace::NegY => Vec3(0.0, -1.0, 0.0),
        AxisFace::PosZ => Vec3(0.0, 0.0, 1.0),
        AxisFace::NegZ => Vec3(0.0, 0.0, -1.0),
    }
}
fn yaw_pitch_from_dir(v: Vec3) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt().max(1e-6);
    let dx = v.0 / len;
    let dy = v.1 / len;
    let dz = v.2 / len;
    (dy.atan2(dx), dz.clamp(-1.0, 1.0).asin())
}

// линейная интерполяция
#[inline]
fn lerp_linear(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
// интерполяция угла по кратчайшему пути
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = (b - a) % std::f32::consts::TAU;
    if diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    } else if diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }
    a + diff * t
}

/* подпись плоскости на грани */
fn plane_label(face: AxisFace) -> &'static str {
    match face {
        AxisFace::PosZ | AxisFace::NegZ => "XY",
        AxisFace::PosY | AxisFace::NegY => "XZ",
        AxisFace::PosX | AxisFace::NegX => "YZ",
    }
}

/* --- helpers для точного сравнения угла/ребра --- */

#[inline]
fn corner_triplet(i: usize) -> [AxisFace; 3] {
    match i {
        0 => [AxisFace::NegX, AxisFace::NegY, AxisFace::NegZ],
        1 => [AxisFace::PosX, AxisFace::NegY, AxisFace::NegZ],
        2 => [AxisFace::PosX, AxisFace::PosY, AxisFace::NegZ],
        3 => [AxisFace::NegX, AxisFace::PosY, AxisFace::NegZ],
        4 => [AxisFace::NegX, AxisFace::NegY, AxisFace::PosZ],
        5 => [AxisFace::PosX, AxisFace::NegY, AxisFace::PosZ],
        6 => [AxisFace::PosX, AxisFace::PosY, AxisFace::PosZ],
        7 => [AxisFace::NegX, AxisFace::PosY, AxisFace::PosZ],
        _ => [AxisFace::NegX, AxisFace::NegY, AxisFace::NegZ],
    }
}

#[inline]
fn same_edge(a: [AxisFace; 2], b: [AxisFace; 2]) -> bool {
    (a[0] == b[0] && a[1] == b[1]) || (a[0] == b[1] && a[1] == b[0])
}

#[inline]
fn same_corner(a: [AxisFace; 3], b: [AxisFace; 3]) -> bool {
    use AxisFace::*;
    let m = |x: AxisFace| -> u32 {
        1u32 << (match x {
            PosX => 0,
            NegX => 1,
            PosY => 2,
            NegY => 3,
            PosZ => 4,
            NegZ => 5,
        })
    };
    let ma = m(a[0]) | m(a[1]) | m(a[2]);
    let mb = m(b[0]) | m(b[1]) | m(b[2]);
    ma == mb
}
