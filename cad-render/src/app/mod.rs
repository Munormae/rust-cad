// cad-render/src/app/mod.rs
use anyhow::Result;
use cad_core::*;
use egui::{Context, Key, PointerButton, Sense, Ui};

mod view3d;
use view3d::View3D;

mod draw;
mod osnap;
mod input;
mod selection;
mod history;
mod picking;
mod camera;

pub use osnap::{apply_osnap_or_grid, compute_osnap, Osnap, SnapKind};
pub use input::is_pan_drag;
pub use selection::{Selection, SelectionRect};
pub use history::History;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Line,
    Arc,
    Nurbs,
    Pan,
}

pub struct AppState {
    pub doc: Document,
    pub tool: Tool,

    pub(crate) tmp_pts: Vec<Pt2>,

    pub(crate) selection: Selection,
    pub(crate) drag_prev_world: Option<Pt2>,
    pub(crate) select_rect: Option<SelectionRect>,

    pub(crate) ortho_enabled: bool,
    pub osnap: Osnap,

    pub(crate) history: History,

    // ---- 3D ----
    pub show_3d: bool,
    pub viewer3d: View3D,
    pub project3d: cad_core::model3d::Project3D,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            doc: Document::new(),
            tool: Tool::Select,
            tmp_pts: Vec::new(),
            selection: Selection::default(),
            drag_prev_world: None,
            select_rect: None,
            ortho_enabled: false,
            osnap: Osnap::default(),
            history: History::default(),
            show_3d: false,
            viewer3d: View3D::default(),
            project3d: cad_core::model3d::Project3D::default(),
        }
    }
}

impl AppState {
    pub fn ui(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| self.toolbar(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_3d {
                if self.project3d.models.is_empty() {
                    self.project3d = demo_project3d();
                }
                self.viewer3d.ui(ui, &self.project3d);
            } else {
                self.canvas(ui);
            }
        });
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("rust-cad");
            ui.separator();

            for (label, t) in [
                ("Select", Tool::Select),
                ("Line",   Tool::Line),
                ("Arc",    Tool::Arc),
                ("NURBS",  Tool::Nurbs),
                ("Pan",    Tool::Pan),
            ] {
                if ui.selectable_label(self.tool == t, label).clicked() {
                    self.tool = t;
                    self.tmp_pts.clear();
                    self.select_rect = None;
                }
            }
            ui.separator();

            // --- 2D Zoom controls (показываем только в 2D) ---
            if !self.show_3d {
                if ui.button("Zoom-In").clicked()  { self.zoom_by(1.25, None); }
                if ui.button("Zoom-Out").clicked() { self.zoom_by(1.0 / 1.25, None); }
                if ui.button("Fit (Extents)").clicked() {
                    let rect = ui.available_rect_before_wrap();
                    self.zoom_to_fit_all(rect);
                }
                ui.separator();
            }

            // OSNAP / Ortho
            let os = ui.selectable_label(self.osnap.enabled, "OSNAP (F3)");
            if os.clicked() { self.osnap.enabled = !self.osnap.enabled; }
            let ortho_btn = ui.selectable_label(self.ortho_enabled, "Ortho (F8)");
            if ortho_btn.clicked() { self.ortho_enabled = !self.ortho_enabled; }

            ui.separator();

            // 2D / 3D toggle
            if ui.selectable_label(self.show_3d, "3D View").clicked() {
                self.show_3d = !self.show_3d;
                if self.show_3d {
                    let rect = ui.available_rect_before_wrap();
                    self.viewer3d.fit_project(&self.project3d, rect);
                }
            }

            ui.separator();

            // === IFC ===
            if ui.button("Import IFC").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("IFC", &["ifc"]).pick_file() {
                    match cad_core::ifc_io::import_ifc(path.to_string_lossy().as_ref()) {
                        Ok(project) => {
                            self.project3d = project;
                            self.show_3d = true;
                            let rect = ui.available_rect_before_wrap();
                            self.viewer3d.fit_project(&self.project3d, rect);
                        }
                        Err(e) => eprintln!("IFC import error: {e}"),
                    }
                }
            }

            ui.separator();

            // === DXF ===
            if ui.button("Import DXF").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("DXF", &["dxf"]).pick_file() {
                    match cad_core::dxf_io::import_dxf(path.to_string_lossy().as_ref()) {
                        Ok(doc) => {
                            self.history.record(&self.doc);
                            self.doc = doc;
                            self.selection.clear();
                            self.select_rect = None;
                            self.tmp_pts.clear();
                            self.zoom_to_fit_all(ui.available_rect_before_wrap());
                        }
                        Err(e) => eprintln!("DXF import error: {e}"),
                    }
                }
            }
            if ui.button("Export DXF").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("DXF", &["dxf"]).save_file() {
                    if let Err(e) = cad_core::dxf_io::export_dxf(&self.doc, path.to_string_lossy().as_ref()) {
                        eprintln!("DXF export error: {e}");
                    }
                }
            }
            // === /DXF ===

            ui.separator();
            if ui.button("Save JSON").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file() {
                    if let Err(e) = std::fs::write(&path, self.doc.to_json()) {
                        eprintln!("save json error: {e}");
                    }
                }
            }
            if ui.button("Export SVG").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("SVG", &["svg"]).save_file() {
                    let size = ui.available_size();
                    let svg = self.doc.export_svg(size.x.max(1.0), size.y.max(1.0));
                    if let Err(e) = std::fs::write(&path, svg) {
                        eprintln!("save svg error: {e}");
                    }
                }
            }
        });
    }

    #[inline]
    fn apply_ortho(&self, anchor: Pt2, mut p: Pt2) -> Pt2 {
        if !self.ortho_enabled { return p; }
        let dx = (p.x - anchor.x).abs();
        let dy = (p.y - anchor.y).abs();
        if dx > dy { p.y = anchor.y; } else { p.x = anchor.x; }
        p
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        // зум из camera.rs
        self.handle_zoom(&response, rect);
        self.handle_shortcuts(ui, rect);

        // PAN
        if is_pan_drag(ui, &response, self.tool) {
            let d = response.drag_delta();
            let z = self.doc.camera.zoom.max(0.01);
            self.doc.camera.pan.x -= d.x / z;
            self.doc.camera.pan.y += d.y / z; // инверсия Y
            self.drag_prev_world = None;
            self.select_rect = None;
        }

        // osnap marker
        self.osnap.last = None;
        if let Some(mp) = response.hover_pos() {
            let world = self.from_screen(mp, rect);
            if self.osnap.enabled {
                if let Some((_id, p, kind)) =
                    compute_osnap(&self.doc, &self.doc.camera, rect, world, self.osnap.pixel_radius)
                {
                    self.osnap.last = Some((p, kind));
                }
            }
        }

        // draw
        self.draw_grid(ui, rect);
        self.draw_entities(ui, rect);
        self.selection.draw_overlay(ui, rect, self);

        // click
        if response.clicked_by(PointerButton::Primary) {
            if let Some(mp) = response.interact_pointer_pos() {
                let p_world = self.from_screen(mp, rect);
                let mut p = apply_osnap_or_grid(&self.osnap, &self.doc, &self.doc.camera, rect, p_world);
                match self.tool {
                    Tool::Line  => if let Some(a) = self.tmp_pts.first().copied() { p = self.apply_ortho(a, p); }
                    Tool::Nurbs => if let Some(a) = self.tmp_pts.last().copied()  { p = self.apply_ortho(a, p); }
                    _ => {}
                }
                self.on_primary_click(p, rect, &response).ok();
            }
        }

        // drag LMB — move or box
        if self.tool == Tool::Select && response.dragged_by(PointerButton::Primary) {
            if let Some(mp) = response.interact_pointer_pos() {
                let world = self.from_screen(mp, rect);

                if self.select_rect.is_some() {
                    if let Some(sr) = &mut self.select_rect { sr.update_current(world); }
                } else if let Some(prev) = self.drag_prev_world {
                    if !self.selection.is_empty() {
                        let dx = world.x - prev.x;
                        let dy = world.y - prev.y;
                        if dx != 0.0 || dy != 0.0 {
                            self.history.ensure_drag_backup(&self.doc);
                            for id in self.selection.ids.iter().copied().collect::<Vec<_>>() {
                                if let Some(ent) = self.doc.entities.iter_mut().find(|e| e.id == id) {
                                    translate_entity(ent, dx, dy);
                                }
                            }
                        }
                        self.drag_prev_world = Some(world);
                    }
                } else {
                    let start_world = self.from_screen(response.interact_pointer_pos().unwrap(), rect);
                    let picked = self.pick_entity(start_world, rect, self.osnap.pixel_radius * 1.2);
                    if picked.is_none() {
                        self.select_rect = Some(SelectionRect::start(start_world));
                    } else {
                        let id = picked.unwrap();
                        if !self.selection.ids.contains(&id) {
                            self.selection.clear();
                            self.selection.add(id);
                        }
                        self.drag_prev_world = Some(world);
                    }
                }
            }
        } else if response.drag_stopped_by(PointerButton::Primary) {
            if let Some(sr) = self.select_rect.take() { self.apply_selection_rect(sr, rect); }
            if self.history.has_drag_backup() { self.history.commit_drag(&mut self.doc); }
            self.drag_prev_world = None;
        }

        // Esc
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.tmp_pts.clear();
            self.selection.clear();
            self.select_rect = None;
            self.drag_prev_world = None;
        }

        // previews
        if let Some(mp) = response.hover_pos() {
            let p_world = self.from_screen(mp, rect);
            let mut p = apply_osnap_or_grid(&self.osnap, &self.doc, &self.doc.camera, rect, p_world);
            match self.tool {
                Tool::Line  => { if let Some(a) = self.tmp_pts.first().copied() { p = self.apply_ortho(a, p); } }
                Tool::Nurbs => { if let Some(a) = self.tmp_pts.last().copied()  { p = self.apply_ortho(a, p); } }
                _ => {}
            }
            self.draw_previews(ui, rect, p);
        }
    }

    fn handle_shortcuts(&mut self, ui: &Ui, rect: egui::Rect) {
        ui.input(|i| {
            if i.key_pressed(Key::F3) { self.osnap.enabled = !self.osnap.enabled; }
            if i.key_pressed(Key::F8) { self.ortho_enabled = !self.ortho_enabled; }

            // Zoom shortcuts
            if i.key_pressed(Key::Home) {
                if self.show_3d {
                    self.viewer3d.fit_project(&self.project3d, rect);
                } else {
                    self.zoom_to_fit_all(rect);
                }
            }
            if i.modifiers.command || i.modifiers.ctrl {
                if i.key_pressed(Key::Num0) { self.reset_zoom(rect); }
            }
            if !self.show_3d {
                if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) { self.zoom_by(1.25, None); }
                if i.key_pressed(Key::Minus) { self.zoom_by(1.0 / 1.25, None); }
            }

            // Delete
            if i.key_pressed(Key::Delete) && !self.selection.is_empty() {
                self.history.record(&self.doc);
                self.doc.entities.retain(|e| !self.selection.ids.contains(&e.id));
                self.selection.clear();
            }
            // Duplicate
            if i.modifiers.ctrl && i.key_pressed(Key::D) && !self.selection.is_empty() {
                self.history.record(&self.doc);
                let ids: Vec<u64> = self.selection.ids.iter().copied().collect();
                for id in ids {
                    if let Some(ent) = self.doc.entities.iter().find(|e| e.id == id).cloned() {
                        let mut copy = ent.clone();
                        copy.id = 0;
                        translate_entity(&mut copy, 10.0, 10.0);
                        let new_id = self.doc.add_entity(copy);
                        self.selection.add(new_id);
                    }
                }
            }
            // Undo / Redo
            if i.modifiers.ctrl && i.key_pressed(Key::Z) {
                if let Some(prev) = self.history.undo() {
                    self.doc = prev; self.selection.clear(); self.select_rect = None;
                }
            }
            if i.modifiers.ctrl && i.key_pressed(Key::Y) {
                if let Some(next) = self.history.redo() {
                    self.doc = next; self.selection.clear(); self.select_rect = None;
                }
            }
        });
    }

    // === Клик ЛКМ по канвасу ===
    fn on_primary_click(&mut self, p: Pt2, rect: egui::Rect, response: &egui::Response) -> Result<()> {
        match self.tool {
            Tool::Select => {
                let pick = self.pick_entity(p, rect, self.osnap.pixel_radius * 1.2);
                if let Some(id) = pick {
                    let shift = response.ctx.input(|i| i.modifiers.shift);
                    if !shift { self.selection.clear(); }
                    self.selection.toggle(id);
                    if self.selection.ids.contains(&id) {
                        self.drag_prev_world = response.interact_pointer_pos().map(|mp| self.from_screen(mp, rect));
                    }
                } else {
                    self.selection.clear();
                    self.drag_prev_world = None;
                }
            }
            Tool::Line => {
                if self.tmp_pts.is_empty() { self.history.record(&self.doc); self.tmp_pts.push(p); }
                else { let a = self.tmp_pts[0]; make_line(&mut self.doc, a, p, "0"); self.tmp_pts.clear(); }
            }
            Tool::Arc => {
                if self.tmp_pts.is_empty() { self.history.record(&self.doc); self.tmp_pts.push(p); }
                else {
                    let center = self.tmp_pts[0];
                    let dx = p.x - center.x; let dy = p.y - center.y;
                    let r = (dx * dx + dy * dy).sqrt().max(1.0);
                    make_arc(&mut self.doc, center, r, 0.0, std::f32::consts::FRAC_PI_2, "0");
                    self.tmp_pts.clear();
                }
            }
            Tool::Nurbs => {
                if let Some(last) = self.tmp_pts.last() {
                    let same = (last.x - p.x).abs() < f32::EPSILON && (last.y - p.y).abs() < f32::EPSILON;
                    if same && self.tmp_pts.len() >= 3 {
                        nurbs_from_polyline(&mut self.doc, &self.tmp_pts, "0")?;
                        self.tmp_pts.clear();
                        return Ok(());
                    }
                }
                if self.tmp_pts.is_empty() { self.history.record(&self.doc); }
                self.tmp_pts.push(p);
            }
            Tool::Pan => {}
        }
        Ok(())
    }
}

// ---- маленький демо-проект для 3D режима ----
fn demo_project3d() -> cad_core::model3d::Project3D {
    use cad_core::model3d::{Element3D, ElementGeom, Model3D, Project3D};

    let mut m = Model3D::default();
    let profile = vec![
        Pt2::new(0.0, 0.0),
        Pt2::new(3000.0, 0.0),
        Pt2::new(3000.0, 800.0),
        Pt2::new(0.0, 800.0),
        Pt2::new(0.0, 0.0),
    ];
    m.elements.push(Element3D {
        id: 1,
        name: "Beam".into(),
        xform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        geom: ElementGeom::Extrusion { profile, height: 6000.0 },
        material: 0,
        rebars: vec![],
        meta: Default::default(),
    });

    Project3D { models: vec![m] }
}
