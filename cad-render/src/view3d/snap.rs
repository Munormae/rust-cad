use crate::view3d::Camera;
use cad_core::model3d::{ElementGeom, Project3D, Pt3};
use egui::{Color32, Pos2, Rect, Stroke};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub enum SnapKind {
    Origin,
    Vertex,
    Midpoint,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapHit {
    pub world: Pt3,
    pub screen: Pos2,
    pub dist_px: f32,
    pub kind: SnapKind,
}

#[derive(Debug, Clone)]
pub struct Snapper {
    pub hover: Option<SnapHit>,
    pub radius_px: f32, // радиус поиска в пикселях
}

impl Default for Snapper {
    fn default() -> Self {
        Self { hover: None, radius_px: 10.0 }
    }
}

impl Snapper {
    pub fn new() -> Self { Default::default() }

    /// Обновить текущую наведение-точку по положению курсора.
    pub fn update_hover(
        &mut self,
        cam: &Camera,
        project: &Project3D,
        rect: Rect,
        cursor: Option<Pos2>,
    ) {
        let Some(cursor) = cursor else { self.hover = None; return; };

        let mut best: Option<SnapHit> = None;
        let mut consider = |world: Pt3, kind: SnapKind| {
            if let Some(screen) = cam.world_to_screen(rect, world) {
                let d = (screen - cursor).length();
                if d <= self.radius_px && (best.is_none() || d < best.unwrap().dist_px) {
                    best = Some(SnapHit { world, screen, dist_px: d, kind });
                }
            }
        };

        // начало координат
        consider(Pt3::new(0.0, 0.0, 0.0), SnapKind::Origin);

        for model in &project.models {
            for el in &model.elements {
                match &el.geom {
                    ElementGeom::Extrusion { profile, height } => {
                        if profile.is_empty() { continue; }
                        // только трансляция из xform
                        let t = [el.xform[0][3], el.xform[1][3], el.xform[2][3]];

                        // вершины: низ + верх
                        let mut bot = Vec::with_capacity(profile.len());
                        let mut top = Vec::with_capacity(profile.len());
                        for p in profile {
                            bot.push(Pt3::new(p.x + t[0], p.y + t[1], 0.0 + t[2]));
                            top.push(Pt3::new(p.x + t[0], p.y + t[1], *height + t[2]));
                        }
                        for v in &bot { consider(*v, SnapKind::Vertex); }
                        for v in &top { consider(*v, SnapKind::Vertex); }

                        // середины рёбер: нижний/верхний контуры
                        for w in bot.windows(2) {
                            let mid = Pt3::new((w[0].x + w[1].x) * 0.5, (w[0].y + w[1].y) * 0.5, (w[0].z + w[1].z) * 0.5);
                            consider(mid, SnapKind::Midpoint);
                        }
                        for w in top.windows(2) {
                            let mid = Pt3::new((w[0].x + w[1].x) * 0.5, (w[0].y + w[1].y) * 0.5, (w[0].z + w[1].z) * 0.5);
                            consider(mid, SnapKind::Midpoint);
                        }
                        // середины вертикалей
                        let n = bot.len().min(top.len());
                        for i in 0..n {
                            let mid = Pt3::new(
                                0.5 * (bot[i].x + top[i].x),
                                0.5 * (bot[i].y + top[i].y),
                                0.5 * (bot[i].z + top[i].z),
                            );
                            consider(mid, SnapKind::Midpoint);
                        }
                    }
                    ElementGeom::SweepCylinder { path, .. } => {
                        if path.is_empty() { continue; }
                        // вершины пути
                        for p in path {
                            consider(*p, SnapKind::Vertex);
                        }
                        // середины сегментов
                        for w in path.windows(2) {
                            let mid = Pt3::new((w[0].x + w[1].x) * 0.5, (w[0].y + w[1].y) * 0.5, (w[0].z + w[1].z) * 0.5);
                            consider(mid, SnapKind::Midpoint);
                        }
                    }
                    ElementGeom::Mesh { positions, indices } => {
                        // вершины
                        for v in positions { consider(*v, SnapKind::Vertex); }
                        // середины рёбер без дублей
                        let mut edges = HashSet::<(u32,u32)>::new();
                        for tri in indices.chunks(3) {
                            if tri.len() < 3 { continue; }
                            let e = [(tri[0],tri[1]), (tri[1],tri[2]), (tri[2],tri[0])];
                            for (a,b) in e {
                                let key = if a < b {(a,b)} else {(b,a)};
                                if edges.insert(key) {
                                    let pa = positions[a as usize];
                                    let pb = positions[b as usize];
                                    let mid = Pt3::new((pa.x + pb.x)*0.5, (pa.y + pb.y)*0.5, (pa.z + pb.z)*0.5);
                                    consider(mid, SnapKind::Midpoint);
                                }
                            }
                        }
                    }
                    ElementGeom::Brep(_) => {}
                }
            }
        }

        self.hover = best;
    }

    /// Нарисовать маркер текущей наведённой привязки.
    pub fn draw(&self, cam: &Camera, rect: Rect, painter: &egui::Painter) {
        if let Some(hit) = &self.hover {
            let (col, r) = match hit.kind {
                SnapKind::Origin   => (Color32::from_rgb(255, 160, 0), 8.0),
                SnapKind::Vertex   => (Color32::from_rgb(255, 0, 200), 7.0),
                SnapKind::Midpoint => (Color32::from_rgb(0, 200, 255), 7.0),
            };
            let stroke = Stroke { width: 1.5, color: col };
            painter.circle_stroke(hit.screen, r, stroke);
            painter.line_segment([hit.screen + egui::vec2(-r, 0.0), hit.screen + egui::vec2(r, 0.0)], stroke);
            painter.line_segment([hit.screen + egui::vec2(0.0, -r), hit.screen + egui::vec2(0.0, r)], stroke);
        }

        // дополнительно можно подписать pivot
        let _ = (cam, rect);
    }
}
