// cad-core/src/ifc.rs
use anyhow::Result;
use ifc_core as ffi;

use crate::model3d::{Element3D, ElementGeom, Model3D, Project3D};
use crate::Pt2;

/// Импорт IFC → конвертим в наш Project3D.
pub fn import_ifc(path: &str) -> Result<Project3D> {
    let raw = ffi::import_raw(path)?; // владелец C-памяти

    let mut model = Model3D::default();
    let mut id_counter: u64 = 1;

    // --- Экструзии ---
    for ex in raw.extrusions() {
        // профиль
        let pts = unsafe {
            std::slice::from_raw_parts(ex.profile.pts, ex.profile.len.max(0) as usize)
        };
        let profile: Vec<Pt2> = pts.iter().map(|p| Pt2::new(p.x as f32, p.y as f32)).collect();

        // xform (без аннотации типа — пусть выведется из поля Element3D::xform)
        let mut xf = [[0.0; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                xf[r][c] = ex.xform[r * 4 + c] as f32;
            }
        }

        model.elements.push(Element3D {
            id: {
                let id = id_counter;
                id_counter += 1;
                id
            },
            name: "IFC Extrusion".into(),
            xform: xf,
            geom: ElementGeom::Extrusion {
                profile,
                height: ex.height as f32,
            },
            material: 0,
            rebars: vec![],
            meta: Default::default(),
        });
    }

    // --- Меши (если начинаешь отдавать из C++ — раскомментируй и заполни свой тип) ---
    // for m in raw.meshes() {
    //     let positions = unsafe {
    //         std::slice::from_raw_parts(m.positions, m.positions_len.max(0) as usize)
    //     }.to_vec();
    //     let indices = unsafe {
    //         std::slice::from_raw_parts(m.indices, m.indices_len.max(0) as usize)
    //     }.to_vec();
    //
    //     let mut xf = [[0.0; 4]; 4];
    //     for r in 0..4 {
    //         for c in 0..4 {
    //             xf[r][c] = m.xform[r * 4 + c] as f32;
    //         }
    //     }
    //
    //     model.elements.push(Element3D {
    //         id: { let id = id_counter; id_counter += 1; id },
    //         name: "IFC Mesh".into(),
    //         xform: xf,
    //         geom: ElementGeom::TriMesh { positions, indices }, // подгони под свой enum
    //         material: 0,
    //         rebars: vec![],
    //         meta: Default::default(),
    //     });
    // }

    Ok(Project3D { models: vec![model] })
}