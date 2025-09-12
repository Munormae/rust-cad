use cad_core::model3d::Pt3;
use egui::{Pos2, Rect};

#[derive(Debug, Clone, Copy)]
pub enum Projection {
    /// Ортографическая проекция: half_h — половина высоты видимой области (в мировых).
    Ortho { half_h: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Точка в мире, которая попадает в центр экрана.
    pub center: Pt3,
    /// Точка вращения (визир); вокруг неё орбитим.
    pub pivot: Pt3,
    /// Углы ориентации камеры (радианы).
    pub yaw: f32,
    pub pitch: f32,
    /// Z-диапазон отсечения вдоль оси взгляда (в координатах камеры).
    pub near: f32,
    pub far: f32,
    /// Тип проекции.
    pub proj: Projection,
}

impl Camera {
    /// Базовый пресет: ортографика.
    pub fn default_ortho() -> Self {
        Self {
            center: Pt3::new(0.0, 0.0, 0.0),
            pivot: Pt3::new(0.0, 0.0, 0.0),
            yaw: 0.8,
            pitch: 0.4,
            // для орто удобно разрешить сквозные значения по z:
            near: -1_000_000.0,
            far: 1_000_000.0,
            proj: Projection::Ortho { half_h: 5000.0 },
        }
    }

    /// Базисы камеры (right, up, fwd).
    ///
    /// ВАЖНО: делаем right независимым от pitch, чтобы не было сингулярности на полюсах.
    /// right = вращение мирового (0,1,0) вокруг Z на yaw.
    /// fwd   = (cos(yaw)*cos(pitch), sin(yaw)*cos(pitch), sin(pitch)).
    /// up    = right × fwd.
    pub fn axes(&self) -> ((f32, f32, f32), (f32, f32, f32), (f32, f32, f32)) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();

        // УСТОЙЧИВЫЙ базис:
        // right — зависит только от yaw (не вырождается и не "флипает" при |pitch|≈90°)
        let r = (-sy, cy, 0.0);

        // forward — как обычно из yaw/pitch
        let f = (cy * cp, sy * cp, sp);

        // up — праворукий: f × r (именно в таком порядке!)
        let u = (
            f.1 * r.2 - f.2 * r.1,
            f.2 * r.0 - f.0 * r.2,
            f.0 * r.1 - f.1 * r.0,
        );

        (r, u, f)
    }

    /// Мир → экран (ортографика).
    pub fn world_to_screen(&self, rect: Rect, p: Pt3) -> Option<Pos2> {
        let (r, u, f) = self.axes();

        // в кам-координаты (относительно center, т.к. орто):
        let vx = p.x - self.center.x;
        let vy = p.y - self.center.y;
        let vz = p.z - self.center.z;

        let x = vx * r.0 + vy * r.1 + vz * r.2;
        let y = vx * u.0 + vy * u.1 + vz * u.2;
        let z = vx * f.0 + vy * f.1 + vz * f.2;

        if z < self.near || z > self.far {
            return None;
        }

        let aspect = (rect.width() / rect.height()).max(1e-4);
        let half_h = match self.proj {
            Projection::Ortho { half_h } => half_h,
        };

        // орто-проекция в NDC
        let ndc_x = x / (half_h * aspect);
        let ndc_y = y / half_h;

        // NDC → пиксели
        let sx = rect.center().x + 0.5 * rect.width() * ndc_x;
        let sy = rect.center().y - 0.5 * rect.height() * ndc_y;
        Some(Pos2::new(sx, sy))
    }

    /// Сдвиг курсора в пикселях → сдвиг центра вида в мировых (для панорамирования).
    pub fn screen_delta_to_world_pan(&self, rect: Rect, dpx: f32, dpy: f32) -> (f32, f32, f32) {
        let (r, u, _f) = self.axes();
        let aspect = (rect.width() / rect.height()).max(1e-4);
        let half_h = match self.proj {
            Projection::Ortho { half_h } => half_h,
        };

        // перевод пикселей экрана в мировые по осям экрана (x — вправо, y — вверх)
        let dx_world = (dpx / rect.width()) * (2.0 * half_h * aspect);
        let dy_world = -(dpy / rect.height()) * (2.0 * half_h);

        // и затем в мировые координаты
        (
            r.0 * dx_world + u.0 * dy_world,
            r.1 * dx_world + u.1 * dy_world,
            r.2 * dx_world + u.2 * dy_world,
        )
    }

    /// Зум для орто: масштабируем half_h.
    pub fn zoom_ortho(&mut self, factor: f32) {
        if let Projection::Ortho { half_h } = &mut self.proj {
            *half_h = (*half_h * factor).clamp(0.001, 1.0e12);
        }
    }

    /// Повернуть вид вокруг pivot, сохраняя расстояние center→pivot.
    ///
    /// Сначала раскладываем текущий вектор по **старым** осям, затем меняем углы,
    /// затем собираем в **новых** осях. Pitch не «упирается»: оборачиваем по ±π.
    pub fn rotate_around_pivot(&mut self, dyaw: f32, dpitch: f32) {
        // Вектор от pivot к центру.
        let v = Pt3::new(
            self.center.x - self.pivot.x,
            self.center.y - self.pivot.y,
            self.center.z - self.pivot.z,
        );

        // Оси ДО изменения углов
        let (r0, u0, f0) = self.axes();

        // Разложим старый вектор по старым осям
        let nx = v.x * r0.0 + v.y * r0.1 + v.z * r0.2;
        let ny = v.x * u0.0 + v.y * u0.1 + v.z * u0.2;
        let nz = v.x * f0.0 + v.y * f0.1 + v.z * f0.2;

        // Обновляем углы (yaw свободно; pitch оборачиваем по ±π)
        self.yaw += dyaw;
        self.pitch = wrap_pi(self.pitch + dpitch);

        // Оси ПОСЛЕ изменения углов
        let (r1, u1, f1) = self.axes();

        // Собираем новый вектор в мировых
        let v2 = Pt3::new(
            r1.0 * nx + u1.0 * ny + f1.0 * nz,
            r1.1 * nx + u1.1 * ny + f1.1 * nz,
            r1.2 * nx + u1.2 * ny + f1.2 * nz,
        );

        // Новый center = pivot + повернутый вектор
        self.center = Pt3::new(
            self.pivot.x + v2.x,
            self.pivot.y + v2.y,
            self.pivot.z + v2.z,
        );
    }

    /// Подогнать орто-параметры под bbox, центрируя вид.
    pub fn fit_bbox_ortho(&mut self, rect: Rect, min: Pt3, max: Pt3) {
        self.center = Pt3::new(
            0.5 * (min.x + max.x),
            0.5 * (min.y + max.y),
            0.5 * (min.z + max.z),
        );

        let (r, u, _) = self.axes();
        let dx = (max.x - min.x).abs();
        let dy = (max.y - min.y).abs();
        let dz = (max.z - min.z).abs();

        // проекция габарита на оси экрана
        let sx = (r.0 * dx + r.1 * dy + r.2 * dz).abs();
        let sy = (u.0 * dx + u.1 * dy + u.2 * dz).abs();

        let aspect = (rect.width() / rect.height()).max(1e-4);
        let half_h_x = 0.5 * sx / aspect;
        let half_h_y = 0.5 * sy;
        let need = half_h_x.max(half_h_y).max(1.0);

        if let Projection::Ortho { half_h } = &mut self.proj {
            *half_h = need * 1.1; // небольшой запас
        }
    }

    /// Проекция точки под курсором на плоскость Z=0 (для установки pivot).
    pub fn screen_to_world_on_z0(&self, rect: Rect, cursor: Pos2) -> Pt3 {
        // Эквивалент «пан-смещения» от центра экрана в мировой позиции:
        let (dx, dy, dz) = self.screen_delta_to_world_pan(
            rect,
            cursor.x - rect.center().x,
            cursor.y - rect.center().y,
        );
        // Берём X/Y из экранного сдвига; Z фиксируем нулём.
        let _ = dz;
        Pt3::new(self.center.x + dx, self.center.y + dy, 0.0)
    }
    pub fn reorient_around_pivot(&mut self, new_yaw: f32, new_pitch: f32) {
        // разложим текущий вектор center-pivot по старым осям и соберём в новых
        let v = Pt3::new(
            self.center.x - self.pivot.x,
            self.center.y - self.pivot.y,
            self.center.z - self.pivot.z,
        );
        let (r0, u0, f0) = self.axes();
        let nx = v.x * r0.0 + v.y * r0.1 + v.z * r0.2;
        let ny = v.x * u0.0 + v.y * u0.1 + v.z * u0.2;
        let nz = v.x * f0.0 + v.y * f0.1 + v.z * f0.2;

        self.yaw = new_yaw;
        // тот же wrap_pi, что и в rotate_around_pivot
        fn wrap_pi(a: f32) -> f32 {
            use std::f32::consts::PI;
            let mut x = a % (2.0 * PI);
            if x > PI {
                x -= 2.0 * PI;
            }
            if x <= -PI {
                x += 2.0 * PI;
            }
            x
        }
        self.pitch = wrap_pi(new_pitch);

        let (r1, u1, f1) = self.axes();
        let v2 = Pt3::new(
            r1.0 * nx + u1.0 * ny + f1.0 * nz,
            r1.1 * nx + u1.1 * ny + f1.1 * nz,
            r1.2 * nx + u1.2 * ny + f1.2 * nz,
        );
        self.center = Pt3::new(
            self.pivot.x + v2.x,
            self.pivot.y + v2.y,
            self.pivot.z + v2.z,
        );
    }
}

// ===== утилиты =====

fn cross(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

fn length(v: (f32, f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt()
}

fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let l = length(v).max(1e-6);
    (v.0 / l, v.1 / l, v.2 / l)
}

/// Оборачивание угла в диапазон (-π, π].
fn wrap_pi(a: f32) -> f32 {
    use std::f32::consts::PI;
    let mut x = a % (2.0 * PI);
    if x > PI {
        x -= 2.0 * PI;
    }
    if x <= -PI {
        x += 2.0 * PI;
    }
    x
}
