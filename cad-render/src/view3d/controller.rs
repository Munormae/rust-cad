use crate::view3d::{Camera, Snapper};
use egui::{Key, PointerButton, Rect, Response};

pub struct Controller {
    pub pan_sens: f32,
    pub rot_sens: f32,
    pub zoom_sens: f32,         // множитель (>1)
    rotate_with_mouse: bool,    // режим Ctrl+R
    placing_pivot: bool,        // режим «V → ждём ЛКМ»
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            pan_sens: 1.0,
            rot_sens: 0.01,
            zoom_sens: 1.1,
            rotate_with_mouse: false,
            placing_pivot: false,
        }
    }
}

impl Controller {
    pub fn handle(&mut self, cam: &mut Camera, snap: &mut Snapper, resp: &Response, rect: Rect) {
        // --- хоткеи ---
        resp.ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(Key::R) {
                self.rotate_with_mouse = true; // «Rotate with mouse» до конца текущего drag
            }
            if i.key_pressed(Key::V) {
                self.placing_pivot = true;     // следующий ЛКМ поставит pivot
            }
            if i.key_pressed(Key::Escape) {
                self.placing_pivot = false;    // отмена постановки pivot
            }
        });

        // --- постановка pivot по ЛКМ после V (со снапом) ---
        if self.placing_pivot && resp.clicked_by(PointerButton::Primary) {
            if let Some(hit) = snap.hover {
                cam.pivot = hit.world;
            } else if let Some(pos) = resp.interact_pointer_pos() {
                cam.pivot = cam.screen_to_world_on_z0(rect, pos);
            }
            self.placing_pivot = false;
            return; // этот клик не идёт в пан/вращение
        }

        // --- колесо: зум орто ---
        let scroll = resp.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.0 {
            let f = if scroll > 0.0 { 1.0 / self.zoom_sens } else { self.zoom_sens };
            cam.zoom_ortho(f);
        }

        // --- жесты мыши ---
        if resp.dragged() && resp.drag_delta() != egui::vec2(0.0, 0.0) {
            let lmb  = resp.dragged_by(PointerButton::Primary);
            let mmb  = resp.dragged_by(PointerButton::Middle);
            let d    = resp.drag_delta();
            let ctrl = resp.ctx.input(|i| i.modifiers.ctrl);

            // PAN: ЛКМ (если не в режиме вращения) ИЛИ MMB без Ctrl
            if (lmb && !self.rotate_with_mouse) || (mmb && !ctrl) {
                let (dx,dy,dz) = cam.screen_delta_to_world_pan(rect, d.x * self.pan_sens, d.y * self.pan_sens);
                cam.center.x -= dx; cam.center.y -= dy; cam.center.z -= dz;
                // pivot не двигаем — фиксирован в мире
            }

            // ROTATE: Ctrl+MMB ИЛИ Ctrl+R режим + ЛКМ
            if (mmb && ctrl) || (lmb && self.rotate_with_mouse) {
                // зафиксируем экранную позицию визира
                let s0 = cam.world_to_screen(rect, cam.pivot);

                // поворот вокруг pivot
                cam.rotate_around_pivot(d.x * self.rot_sens, -d.y * self.rot_sens);

                // screen-lock: компенсируем «уплывание» проекции pivot
                if let Some(s0) = s0 {
                    if let Some(s1) = cam.world_to_screen(rect, cam.pivot) {
                        let dd = s1 - s0;
                        if dd.length_sq() > 0.0001 {
                            let (dx,dy,dz) = cam.screen_delta_to_world_pan(rect, dd.x, dd.y);
                            cam.center.x -= dx; cam.center.y -= dy; cam.center.z -= dz;
                        }
                    }
                }
            }

            // ПКМ полностью игнорируем — ничего не делаем
        }

        // окончание «Rotate with mouse»
        if resp.drag_stopped() {
            self.rotate_with_mouse = false;
        }
    }
}