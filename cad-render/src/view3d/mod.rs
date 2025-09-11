mod camera;      pub use camera::Camera;
mod controller;  pub use controller::Controller;
mod renderer;    pub use renderer::draw_scene;
mod gizmos;
mod snap;        pub use snap::{Snapper, SnapKind};
mod viewcube;    use viewcube::ViewCube;

use cad_core::model3d::{Project3D, Pt3};
use egui::{Rect, Sense, Ui};

pub struct View3D {
    pub cam: Camera,
    pub ctrl: Controller,
    pub snap: Snapper,
    viewcube: ViewCube,
}

impl Default for View3D {
    fn default() -> Self {
        Self {
            cam: Camera::default_ortho(),
            ctrl: Controller::default(),
            snap: Snapper::new(),
            viewcube: ViewCube::default(),
        }
    }
}

impl View3D {
    pub fn ui(&mut self, ui: &mut Ui, project: &Project3D) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // обновляем наведённый снап под курсором
        let cursor = resp
            .interact_pointer_pos()
            .or_else(|| resp.ctx.input(|i| i.pointer.hover_pos()));
        self.snap.update_hover(&self.cam, project, rect, cursor);

        // управление (пан/зум/вращение/постановка pivot)
        self.ctrl.handle(&mut self.cam, &mut self.snap, &resp, rect);

        // рисуем сцену
        draw_scene(ui, rect, &self.cam, project);

        // оверлеи: маркер снапа
        let painter = ui.painter_at(rect);
        self.snap.draw(&self.cam, rect, &painter);

        // интерактивный ViewCube поверх всего
        self.viewcube.ui(ui, rect, &mut self.cam);
    }

    /// Подогнать вид под проект и выставить pivot в центр bbox
    pub fn fit_project(&mut self, project: &Project3D, rect: Rect) {
        if let Some((min, max)) = renderer::bbox_project(project) {
            self.cam.fit_bbox_ortho(rect, min, max);
            self.cam.pivot = Pt3::new(
                0.5 * (min.x + max.x),
                0.5 * (min.y + max.y),
                0.5 * (min.z + max.z),
            );
        }
    }

    pub fn set_pivot(&mut self, p: Pt3) {
        self.cam.pivot = p;
    }
}
