use crate::view3d::Camera;
use egui::{pos2, Align2, Color32, FontId, Pos2, Rect, Stroke, Ui};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisFace { PosX, NegX, PosY, NegY, PosZ, NegZ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitKind {
    None,
    Face(AxisFace),
    Edge([AxisFace; 2]),
    Corner([AxisFace; 3]),
}

pub struct ViewCube {
    pub size: f32,
    pub margin: f32,
    hover: HitKind,
}

impl Default for ViewCube {
    fn default() -> Self {
        Self { size: 105.0, margin: 12.0, hover: HitKind::None }
    }
}

impl ViewCube {
    pub fn ui(&mut self, ui: &mut Ui, rect_full: Rect, cam: &mut Camera) {
        // правый-нижний угол
        let outer = Rect::from_min_size(
            pos2(rect_full.max.x - self.size - self.margin,
                 rect_full.max.y - self.size - self.margin),
            egui::vec2(self.size, self.size),
        );

        let edge_th   = (outer.width() * 0.14).clamp(4.0, 7.0);
        let corner_px = (outer.width() * 0.18).clamp(6.0, 10.0);
        let pad = 2.0 + 0.5 * edge_th + 0.5 * corner_px + 1.0;

        let content = outer.shrink(pad);
        let resp    = ui.allocate_rect(outer, egui::Sense::click());
        let painter = ui.painter_at(outer);

        self.draw_cube(&painter, content, cam, edge_th, corner_px);

        // hover / click
        let pointer = resp.hover_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));
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
                    self.apply_hit(cam, rect_full, hit);
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
        let (r,u,f) = cam.axes();

        // проекция [-1,1]^3 в осях (r/u)
        let s = rect.width() * 0.33;
        let c = rect.center();
        let to2 = |(x,y,z):(f32,f32,f32)| -> Pos2 {
            let sx = x*r.0 + y*r.1 + z*r.2;
            let sy = x*u.0 + y*u.1 + z*u.2;
            pos2(c.x + s*sx, c.y - s*sy)
        };

        // вершины
        let v = [
            to2((-1.0,-1.0,-1.0)), to2(( 1.0,-1.0,-1.0)), to2(( 1.0, 1.0,-1.0)), to2((-1.0, 1.0,-1.0)),
            to2((-1.0,-1.0, 1.0)), to2(( 1.0,-1.0, 1.0)), to2(( 1.0, 1.0, 1.0)), to2((-1.0, 1.0, 1.0)),
        ];

        // грани
        struct Face { idx:[usize;4], n:(f32,f32,f32), face:AxisFace }
        let faces = [
            Face{idx:[0,1,2,3], n:( 0.0, 0.0,-1.0), face:AxisFace::NegZ}, // XY-
            Face{idx:[4,5,6,7], n:( 0.0, 0.0, 1.0), face:AxisFace::PosZ}, // XY+
            Face{idx:[0,1,5,4], n:( 1.0, 0.0, 0.0), face:AxisFace::PosX}, // YZ+
            Face{idx:[3,2,6,7], n:(-1.0, 0.0, 0.0), face:AxisFace::NegX}, // YZ-
            Face{idx:[0,4,7,3], n:( 0.0, 1.0, 0.0), face:AxisFace::PosY}, // XZ+
            Face{idx:[1,2,6,5], n:( 0.0,-1.0, 0.0), face:AxisFace::NegY}, // XZ-
        ];

        // классификация видимости граней
        let mut back:  Vec<&Face> = Vec::new();
        let mut front: Vec<&Face> = Vec::new();
        // для рёбер используем более мягкий тест: >= 0.0
        let mut face_dot: [f32; 6] = [0.0; 6];

        for (i, fdef) in faces.iter().enumerate() {
            let d = dot(fdef.n, f);
            face_dot[i] = d;
            if d > 0.0 { front.push(fdef); } else { back.push(fdef); }
        }
        let mut sort_depth = |arr: &mut Vec<&Face>| {
            arr.sort_by(|a,b| dot(a.n, f).partial_cmp(&dot(b.n, f)).unwrap());
        };
        sort_depth(&mut back);
        sort_depth(&mut front);

        let outline = Stroke { width: 1.0, color: Color32::from_rgb(20, 20, 20) };

        // 1) дальние грани
        for fd in &back {
            let poly = [v[fd.idx[0]], v[fd.idx[1]], v[fd.idx[2]], v[fd.idx[3]]];
            painter.add(egui::Shape::convex_polygon(poly.to_vec(), tone_down(face_color(fd.face), 0.08), outline));
        }

        // 2) фронтальные грани
        for fd in &front {
            let poly = [v[fd.idx[0]], v[fd.idx[1]], v[fd.idx[2]], v[fd.idx[3]]];
            painter.add(egui::Shape::convex_polygon(poly.to_vec(), tone_up(tone_down(face_color(fd.face), 0.08), 0.10), outline));
            let c2 = centroid(&poly);
            painter.text(c2, Align2::CENTER_CENTER, plane_label(fd.face),
                         FontId::monospace(11.0), Color32::from_rgb(235,238,241));
        }

        // 3) рёбра — показываем если ХОТЯ БЫ ОДНА грань видима (dot >= 0.0)
        let edge_under = edge_th + 2.0;

        // индексы граней соответствуют порядку в массиве `faces`
        let face_idx = |f: AxisFace| -> usize {
            match f {
                AxisFace::NegZ => 0, AxisFace::PosZ => 1,
                AxisFace::PosX => 2, AxisFace::NegX => 3,
                AxisFace::PosY => 4, AxisFace::NegY => 5,
            }
        };
        let edges = [
            (0,1,[AxisFace::PosX,AxisFace::NegZ]), (1,2,[AxisFace::PosZ,AxisFace::PosX]),
            (2,3,[AxisFace::NegX,AxisFace::NegZ]), (3,0,[AxisFace::NegZ,AxisFace::NegX]),
            (4,5,[AxisFace::PosX,AxisFace::PosZ]), (5,6,[AxisFace::NegZ,AxisFace::PosX]),
            (6,7,[AxisFace::NegX,AxisFace::PosZ]), (7,4,[AxisFace::PosZ,AxisFace::NegX]),
            (0,4,[AxisFace::PosY,AxisFace::NegZ]), (1,5,[AxisFace::PosY,AxisFace::PosX]),
            (2,6,[AxisFace::PosY,AxisFace::PosZ]), (3,7,[AxisFace::PosY,AxisFace::NegX]),
        ];
        for (a,b,ff) in edges {
            let d0 = face_dot[face_idx(ff[0])];
            let d1 = face_dot[face_idx(ff[1])];
            if !(d0 >= 0.0 || d1 >= 0.0) { continue; } // ← мягкий критерий, чтобы силуэт не терять

            // подложка (отделение от грани)
            painter.add(egui::Shape::line_segment([v[a], v[b]],
                                                  Stroke{width: edge_under, color: Color32::from_rgb(16,16,16)}));

            let mut col = edge_axis_color(ff);
            if let HitKind::Edge(hff) = self.hover {
                if (hff[0]==ff[0] && hff[1]==ff[1]) || (hff[0]==ff[1] && hff[1]==ff[0]) {
                    col = tone_up(col, 0.16);
                }
            }
            painter.add(egui::Shape::line_segment([v[a], v[b]],
                                                  Stroke{width: edge_th, color: col}));
        }

        // 4) углы — видимы, если вершина принадлежит хотя бы одной фронтальной грани
        let is_front = |af: AxisFace| -> bool {
            match af {
                AxisFace::PosZ => face_dot[1] > 0.0,
                AxisFace::NegZ => face_dot[0] > 0.0,
                AxisFace::PosY => face_dot[4] > 0.0,
                AxisFace::NegY => face_dot[5] > 0.0,
                AxisFace::PosX => face_dot[2] > 0.0,
                AxisFace::NegX => face_dot[3] > 0.0,
            }
        };
        for (i, p) in v.iter().enumerate() {
            let triplet = [
                if [1,2,6,5].contains(&i) { AxisFace::PosX } else { AxisFace::NegX },
                if [3,2,6,7].contains(&i) { AxisFace::PosY } else { AxisFace::NegY },
                if [4,5,6,7].contains(&i) { AxisFace::PosZ } else { AxisFace::NegZ },
            ];
            if !(is_front(triplet[0]) || is_front(triplet[1]) || is_front(triplet[2])) { continue; }

            // подложка
            let r_under = Rect::from_center_size(*p, egui::vec2(corner_px+2.0, corner_px+2.0));
            painter.rect_filled(r_under, 2.0, Color32::from_rgb(16,16,16));

            // цвет — по «положительной» оси
            let face_for_color = triplet.into_iter()
                .find(|f| matches!(f, AxisFace::PosX|AxisFace::PosY|AxisFace::PosZ))
                .unwrap_or(AxisFace::PosX);
            let mut col = face_color(face_for_color);
            if let HitKind::Corner(arr) = self.hover {
                if arr.contains(&face_for_color) { col = tone_up(col, 0.16); }
            }
            let r = Rect::from_center_size(*p, egui::vec2(corner_px, corner_px));
            painter.rect_filled(r, 2.0, col);
        }
    }

    fn pick(&self, rect: Rect, p: Pos2, cam: &Camera) -> HitKind {
        let (r,u,f) = cam.axes();
        let s = rect.width() * 0.33;
        let c = rect.center();
        let to2 = |(x,y,z):(f32,f32,f32)| -> Pos2 {
            let sx = x*r.0 + y*r.1 + z*r.2;
            let sy = x*u.0 + y*u.1 + z*u.2;
            pos2(c.x + s*sx, c.y - s*sy)
        };
        let v = [
            to2((-1.0,-1.0,-1.0)), to2(( 1.0,-1.0,-1.0)), to2(( 1.0, 1.0,-1.0)), to2((-1.0, 1.0,-1.0)),
            to2((-1.0,-1.0, 1.0)), to2(( 1.0,-1.0, 1.0)), to2(( 1.0, 1.0, 1.0)), to2((-1.0, 1.0, 1.0)),
        ];

        // 1) вершина → 2) ребро → 3) фронтальная грань
        let vr = 7.5;
        for (i, pt) in v.iter().enumerate() {
            if (p - *pt).length_sq() <= vr*vr {
                let dir = [
                    if [1,2,6,5].contains(&i) { AxisFace::PosX } else { AxisFace::NegX },
                    if [3,2,6,7].contains(&i) { AxisFace::PosY } else { AxisFace::NegY },
                    if [4,5,6,7].contains(&i) { AxisFace::PosZ } else { AxisFace::NegZ },
                ];
                return HitKind::Corner([dir[0],dir[1],dir[2]]);
            }
        }

        let er = 6.0;
        let edges = [
            (0,1,[AxisFace::PosX,AxisFace::NegZ]), (1,2,[AxisFace::PosZ,AxisFace::PosX]),
            (2,3,[AxisFace::NegX,AxisFace::NegZ]), (3,0,[AxisFace::NegZ,AxisFace::NegX]),
            (4,5,[AxisFace::PosX,AxisFace::PosZ]), (5,6,[AxisFace::NegZ,AxisFace::PosX]),
            (6,7,[AxisFace::NegX,AxisFace::PosZ]), (7,4,[AxisFace::PosZ,AxisFace::NegX]),
            (0,4,[AxisFace::PosY,AxisFace::NegZ]), (1,5,[AxisFace::PosY,AxisFace::PosX]),
            (2,6,[AxisFace::PosY,AxisFace::PosZ]), (3,7,[AxisFace::PosY,AxisFace::NegX]),
        ];
        for (a,b,faces2) in edges {
            if dist_to_segment(p, v[a], v[b]) <= er {
                return HitKind::Edge(faces2);
            }
        }

        let faces = [
            ([0,1,2,3], AxisFace::NegZ, ( 0.0, 0.0,-1.0)),
            ([4,5,6,7], AxisFace::PosZ, ( 0.0, 0.0, 1.0)),
            ([0,1,5,4], AxisFace::PosX, ( 1.0, 0.0, 0.0)),
            ([3,2,6,7], AxisFace::NegX, (-1.0, 0.0, 0.0)),
            ([0,4,7,3], AxisFace::PosY, ( 0.0, 1.0, 0.0)),
            ([1,2,6,5], AxisFace::NegY, ( 0.0,-1.0, 0.0)),
        ];
        for (idx, face, nrm) in faces {
            if dot(nrm, f) <= 0.0 { continue; }
            let poly = [v[idx[0]], v[idx[1]], v[idx[2]], v[idx[3]]];
            if point_in_quad(p, &poly) {
                return HitKind::Face(face);
            }
        }

        HitKind::None
    }

    fn apply_hit(&self, cam: &mut Camera, rect_full: Rect, hit: HitKind) {
        let s0 = cam.world_to_screen(rect_full, cam.pivot);

        match hit {
            HitKind::Face(face) => {
                let (yaw, pitch) = match face {
                    AxisFace::PosX => (0.0, 0.0),
                    AxisFace::NegX => (PI, 0.0),
                    AxisFace::PosY => ( PI*0.5, 0.0),
                    AxisFace::NegY => (-PI*0.5, 0.0),
                    AxisFace::PosZ => (cam.yaw,  PI*0.5),
                    AxisFace::NegZ => (cam.yaw, -PI*0.5),
                };
                cam.reorient_around_pivot(yaw, pitch);
            }
            HitKind::Edge([a,b]) => {
                let dir = face_dir(a) + face_dir(b);
                let (yaw, pitch) = yaw_pitch_from_dir(dir);
                cam.reorient_around_pivot(yaw, pitch);
            }
            HitKind::Corner([a,b,c]) => {
                let dir = face_dir(a) + face_dir(b) + face_dir(c);
                let (yaw, pitch) = yaw_pitch_from_dir(dir);
                cam.reorient_around_pivot(yaw, pitch);
            }
            HitKind::None => {}
        }

        // screen-lock pivot
        if let Some(s0) = s0 {
            if let Some(s1) = cam.world_to_screen(rect_full, cam.pivot) {
                let dd = s1 - s0;
                if dd.length_sq() > 0.0 {
                    let (dx,dy,dz) = cam.screen_delta_to_world_pan(rect_full, dd.x, dd.y);
                    cam.center.x -= dx; cam.center.y -= dy; cam.center.z -= dz;
                }
            }
        }
    }
}

/* ===== utils ===== */

fn dot(a:(f32,f32,f32), b:(f32,f32,f32)) -> f32 { a.0*b.0 + a.1*b.1 + a.2*b.2 }
fn centroid(p:&[Pos2;4]) -> Pos2 {
    pos2((p[0].x+p[1].x+p[2].x+p[3].x)/4.0, (p[0].y+p[1].y+p[2].y+p[3].y)/4.0)
}
fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    (a + t*ab - p).length()
}
fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    #[inline] fn cross2(ax:f32, ay:f32, bx:f32, by:f32)->f32 { ax*by - ay*bx }
    let ab = b - a; let bc = c - b; let ca = a - c;
    let ap = p - a; let bp = p - b; let cp = p - c;
    let s1 = cross2(ab.x, ab.y, ap.x, ap.y);
    let s2 = cross2(bc.x, bc.y, bp.x, bp.y);
    let s3 = cross2(ca.x, ca.y, cp.x, cp.y);
    (s1>=0.0 && s2>=0.0 && s3>=0.0) || (s1<=0.0 && s2<=0.0 && s3<=0.0)
}
fn point_in_quad(p: Pos2, q:&[Pos2;4]) -> bool {
    point_in_triangle(p, q[0], q[1], q[2]) || point_in_triangle(p, q[0], q[2], q[3])
}

#[derive(Clone, Copy)]
struct Vec3(f32,f32,f32);
impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self::Output { Vec3(self.0+rhs.0, self.1+rhs.1, self.2+rhs.2) }
}
fn face_dir(f: AxisFace) -> Vec3 {
    match f {
        AxisFace::PosX => Vec3( 1.0, 0.0, 0.0),
        AxisFace::NegX => Vec3(-1.0, 0.0, 0.0),
        AxisFace::PosY => Vec3( 0.0, 1.0, 0.0),
        AxisFace::NegY => Vec3( 0.0,-1.0, 0.0),
        AxisFace::PosZ => Vec3( 0.0, 0.0, 1.0),
        AxisFace::NegZ => Vec3( 0.0, 0.0,-1.0),
    }
}
fn yaw_pitch_from_dir(v: Vec3) -> (f32,f32) {
    let len = (v.0*v.0 + v.1*v.1 + v.2*v.2).sqrt().max(1e-6);
    let dx = v.0/len; let dy = v.1/len; let dz = v.2/len;
    (dy.atan2(dx), dz.clamp(-1.0, 1.0).asin())
}
fn plane_label(face: AxisFace) -> &'static str {
    match face {
        AxisFace::PosZ | AxisFace::NegZ => "XY",
        AxisFace::PosY | AxisFace::NegY => "XZ",
        AxisFace::PosX | AxisFace::NegX => "YZ",
    }
}
fn face_color(f: AxisFace) -> Color32 {
    match f {
        AxisFace::PosX => Color32::from_rgb(200, 90, 80),   // X — красный
        AxisFace::NegX => Color32::from_rgb(110, 45, 40),
        AxisFace::PosY => Color32::from_rgb(70, 190, 120),  // Y — зелёный
        AxisFace::NegY => Color32::from_rgb(40, 110, 70),
        AxisFace::PosZ => Color32::from_rgb(90, 150, 210),  // Z — синий
        AxisFace::NegZ => Color32::from_rgb(55, 90, 130),
    }
}
fn tone_up(c: Color32, k: f32) -> Color32 {
    let (r,g,b,a) = (c.r() as f32, c.g() as f32, c.b() as f32, c.a());
    let m = |v:f32| -> u8 { (v*(1.0+k)).clamp(0.0,255.0) as u8 };
    Color32::from_rgba_unmultiplied(m(r), m(g), m(b), a)
}
fn tone_down(c: Color32, k: f32) -> Color32 {
    let (r,g,b,a) = (c.r() as f32, c.g() as f32, c.b() as f32, c.a());
    let m = |v:f32| -> u8 { (v*(1.0-k)).clamp(0.0,255.0) as u8 };
    Color32::from_rgba_unmultiplied(m(r), m(g), m(b), a)
}

/// Цвет ребра — по оси пересечения двух граней (третья ось).
fn edge_axis_color(ff: [AxisFace;2]) -> Color32 {
    let has_x = matches!(ff[0], AxisFace::PosX|AxisFace::NegX) || matches!(ff[1], AxisFace::PosX|AxisFace::NegX);
    let has_y = matches!(ff[0], AxisFace::PosY|AxisFace::NegY) || matches!(ff[1], AxisFace::PosY|AxisFace::NegY);
    let has_z = matches!(ff[0], AxisFace::PosZ|AxisFace::NegZ) || matches!(ff[1], AxisFace::PosZ|AxisFace::NegZ);
    if !has_x { return face_color(AxisFace::PosX); } // || X
    if !has_y { return face_color(AxisFace::PosY); } // || Y
    if !has_z { return face_color(AxisFace::PosZ); } // || Z
    Color32::LIGHT_GRAY
}
