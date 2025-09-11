use egui::{Pos2, Rect, Response, Sense, Stroke, Ui};
use cad_core::model3d::{ElementGeom, Project3D, Pt3};

/// Простой 3D viewer: орбита-вокруг-центра, панорамирование, скролл-зум.
/// Рисуем каркасом через egui::Painter.
#[derive(Debug, Clone)]
pub struct View3D {
    pub center: Pt3,  // точка, вокруг которой крутим камеру
    pub yaw: f32,     // горизонтальный угол (рад)
    pub pitch: f32,   // вертикальный угол (рад)
    pub dist: f32,    // расстояние до центра
    pub fov_y: f32,   // вертикальный FOV (рад)
    pub near: f32,
    pub far: f32,
    pub rot_sens: f32,
    pub pan_sens: f32,
    pub zoom_sens: f32,
}

impl Default for View3D {
    fn default() -> Self {
        Self {
            center: Pt3::new(0.0, 0.0, 0.0),
            yaw: 0.8,
            pitch: 0.4,
            dist: 6000.0,
            fov_y: 45.0_f32.to_radians(),
            near: 10.0,
            far: 1_000_000.0,
            rot_sens: 0.01,
            pan_sens: 1.0,
            zoom_sens: 1.1,
        }
    }
}

impl View3D {
    pub fn ui(&mut self, ui: &mut Ui, project: &Project3D) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.handle_inputs(ui, &resp, rect);

        let stroke = Stroke { width: 1.0, color: ui.visuals().strong_text_color() };
        let mut lines: Vec<[Pos2; 2]> = Vec::new();

        for model in &project.models {
            for el in &model.elements {
                match &el.geom {
                    ElementGeom::Extrusion { profile, height } => {
                        if profile.len() >= 2 {
                            // применяем локальную матрицу только как перенос (MVP)
                            let t = [el.xform[0][3], el.xform[1][3], el.xform[2][3]];
                            // нижний и верхний
                            let mut bot: Vec<Pt3> = Vec::with_capacity(profile.len());
                            let mut top: Vec<Pt3> = Vec::with_capacity(profile.len());
                            for p in profile {
                                bot.push(Pt3::new(p.x + t[0], p.y + t[1], 0.0 + t[2]));
                                top.push(Pt3::new(p.x + t[0], p.y + t[1], *height + t[2]));
                            }
                            // нижний контур
                            for w in bot.windows(2) {
                                if let (Some(a), Some(b)) = (self.project(rect, w[0]), self.project(rect, w[1])) {
                                    lines.push([a, b]);
                                }
                            }
                            // верхний контур
                            for w in top.windows(2) {
                                if let (Some(a), Some(b)) = (self.project(rect, w[0]), self.project(rect, w[1])) {
                                    lines.push([a, b]);
                                }
                            }
                            // вертикали
                            let n = bot.len().min(top.len());
                            for i in 0..n {
                                if let (Some(a), Some(b)) = (self.project(rect, bot[i]), self.project(rect, top[i])) {
                                    lines.push([a, b]);
                                }
                            }
                        }
                    }
                    ElementGeom::SweepCylinder { path, .. } => {
                        // осевую отрисуем линией
                        if path.len() >= 2 {
                            for w in path.windows(2) {
                                if let (Some(a), Some(b)) = (self.project(rect, w[0]), self.project(rect, w[1])) {
                                    lines.push([a, b]);
                                }
                            }
                        }
                    }
                    // В этом viewer-е Brep пока игнорируем одинаково для любых сборок.
                    ElementGeom::Brep(_) => {}
                }
            }
        }

        let painter = ui.painter_at(rect);
        for seg in lines {
            painter.line_segment(seg, stroke);
        }
    }

    /// Подогнать камеру под проект (простая bbox-эвристика).
    pub fn fit_project(&mut self, project: &Project3D, rect: Rect) {
        if let Some((min, max)) = bbox_project(project) {
            self.center = Pt3::new(
                0.5 * (min.x + max.x),
                0.5 * (min.y + max.y),
                0.5 * (min.z + max.z),
            );
            let size = ((max.x - min.x).abs().max((max.y - min.y).abs())).max((max.z - min.z).abs());
            let aspect = (rect.width() / rect.height()).max(0.1);
            // примитивная оценка дистанции под FOV
            let half = 0.5 * size.max(1.0);
            let f = (self.fov_y * 0.5).tan();
            self.dist = (half / f).max(100.0) * (aspect.max(1.0));
        }
    }

    // ======== ввод ========

    fn handle_inputs(&mut self, ui: &Ui, resp: &Response, rect: Rect) {
        // колесо — зум
        let scroll = resp.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.0 {
            let factor = if scroll > 0.0 { 1.0 / self.zoom_sens } else { self.zoom_sens };
            self.dist = (self.dist * factor).clamp(50.0, 10_000_000.0);
        }

        // ЛКМ — орбита, ПКМ/СКМ — панорамирование
        if resp.dragged() && resp.drag_delta() != egui::vec2(0.0, 0.0) {
            let lmb = resp.dragged_by(egui::PointerButton::Primary);
            let rmb = resp.dragged_by(egui::PointerButton::Secondary)
                || resp.dragged_by(egui::PointerButton::Middle);
            let d = resp.drag_delta();

            if lmb {
                self.yaw   += d.x * self.rot_sens;
                self.pitch -= d.y * self.rot_sens;
                self.pitch = self.pitch.clamp(-1.5, 1.5);
            } else if rmb {
                // пан — в экранных, переводим приблизительно в мир относительно расстояния
                let k = self.dist * 0.001 * self.pan_sens;
                let (right, up, _fwd) = self.view_axes();
                self.center.x -= (right.0 * d.x + up.0 * d.y) * k;
                self.center.y -= (right.1 * d.x + up.1 * d.y) * k;
                self.center.z -= (right.2 * d.x + up.2 * d.y) * k;
            }
        }

        // Home — Fit (обрабатывается извне; тут ничего не делаем)
        let _ = (ui, rect);
    }

    // ======== проекция ========

    fn project(&self, rect: Rect, p: Pt3) -> Option<Pos2> {
        // камера в сферич координатах вокруг center
        let (cx, cy, cz) = (self.center.x, self.center.y, self.center.z);
        let cam_pos = {
            let (_sy, cyaw) = self.yaw.sin_cos();
            let (sp, cp) = self.pitch.sin_cos();
            // forward смотрит в центр
            let fx =  cyaw * cp;
            let fy =  _sy * cp;
            let fz =  sp;
            Pt3::new(
                cx - fx * self.dist,
                cy - fy * self.dist,
                cz - fz * self.dist,
            )
        };

        // базисы камеры
        let (right, up, fwd) = self.view_axes();

        // в кам-координаты
        let vx = p.x - cam_pos.x;
        let vy = p.y - cam_pos.y;
        let vz = p.z - cam_pos.z;
        let x = vx * right.0 + vy * right.1 + vz * right.2;
        let y = vx * up.0    + vy * up.1    + vz * up.2;
        let z = vx * fwd.0   + vy * fwd.1   + vz * fwd.2; // вдоль взгляда

        if z <= self.near || z >= self.far {
            return None;
        }

        // перспективная проекция
        let h = (self.fov_y * 0.5).tan();
        let aspect = (rect.width() / rect.height()).max(0.0001);
        let ndc_x = x / (z * h * aspect);
        let ndc_y = y / (z * h);

        // в экран (центр — геометрический центр rect)
        let sx = rect.center().x + 0.5 * rect.width() * ndc_x;
        let sy = rect.center().y - 0.5 * rect.height() * ndc_y;

        Some(Pos2::new(sx, sy))
    }

    fn view_axes(&self) -> ((f32, f32, f32), (f32, f32, f32), (f32, f32, f32)) {
        // оси камеры из yaw/pitch (ролл = 0)
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        // forward (на объект)
        let f = (cy * cp, sy * cp, sp);
        // right = normalize(f × world_up(0,0,1)) с учётом сингулярности на полюсах
        let world_up = (0.0_f32, 0.0, 1.0);
        let r = normalize(cross(f, world_up));
        // up = r × f
        let u = cross(r, f);
        (r, u, f)
    }
}

// ================= утилиты =================

fn cross(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0)
}
fn length(v: (f32, f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt()
}
fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let l = length(v).max(1e-6);
    (v.0 / l, v.1 / l, v.2 / l)
}

/// bbox по всем элементам (грубая оценка; Brep игнорим)
fn bbox_project(p: &Project3D) -> Option<(Pt3, Pt3)> {
    let mut min = Pt3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Pt3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;

    let mut acc = |q: Pt3| {
        if q.x.is_finite() && q.y.is_finite() && q.z.is_finite() {
            if q.x < min.x { min.x = q.x; }
            if q.y < min.y { min.y = q.y; }
            if q.z < min.z { min.z = q.z; }
            if q.x > max.x { max.x = q.x; }
            if q.y > max.y { max.y = q.y; }
            if q.z > max.z { max.z = q.z; }
            any = true;
        }
    };

    for m in &p.models {
        for e in &m.elements {
            match &e.geom {
                ElementGeom::Extrusion { profile, height } => {
                    let t = [e.xform[0][3], e.xform[1][3], e.xform[2][3]];
                    for p2 in profile {
                        acc(Pt3::new(p2.x + t[0], p2.y + t[1], 0.0 + t[2]));
                        acc(Pt3::new(p2.x + t[0], p2.y + t[1], *height + t[2]));
                    }
                }
                ElementGeom::SweepCylinder { path, .. } => {
                    for p3 in path {
                        acc(*p3);
                    }
                }
                ElementGeom::Brep(_) => {}
            }
        }
    }
    if any { Some((min, max)) } else { None }
}
