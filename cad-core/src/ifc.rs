// cad-core/src/ifc.rs
use anyhow::{anyhow, Result};
use std::ffi::CString;

// Тянем С++ FFI из ifc_core, импорт переименовываем, чтобы не конфликтовал с нашей функцией.
use ifc_core::{
    extrusions_len,
    extrusions_ptr,
    import_ifc as cxx_import_ifc,
    ExtrusionRaw,
    FileRaw,
};

use crate::model3d::{Element3D, ElementGeom, Model3D, Project3D};
use crate::Pt2 as Pt2f; // твой 2D-тип с f32

/// Импорт IFC → в наш Project3D.
pub fn import_ifc(path: &str) -> Result<Project3D> {
    // C-строка для C++ функции
    let cpath = CString::new(path)?;
    // unique_ptr<FileRaw>
    let file_up = unsafe { cxx_import_ifc(cpath.as_ptr()) };

    // Берём &FileRaw, пока жив unique_ptr
    let raw: &FileRaw = file_up
        .as_ref()
        .ok_or_else(|| anyhow!("IFC import failed (null from C++)"))?;

    // Читаем экструдии
    let len = extrusions_len(raw) as usize;
    let ptr = extrusions_ptr(raw);
    let extrs: &[ExtrusionRaw] = unsafe { std::slice::from_raw_parts(ptr, len) };

    let mut model = Model3D::default();
    let mut id_counter: u64 = 1;

    for ex in extrs {
        // профиль
        let pts = unsafe {
            std::slice::from_raw_parts(ex.profile.pts, ex.profile.len as usize)
        };
        let profile: Vec<Pt2f> = pts.iter().map(|p| Pt2f::new(p.x as f32, p.y as f32)).collect();

        // матрица 4x4
        let mut xf = [[0.0f32; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                xf[r][c] = ex.xform[r * 4 + c] as f32;
            }
        }

        model.elements.push(Element3D {
            id: { let id = id_counter; id_counter += 1; id },
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

    Ok(Project3D { models: vec![model] })
}
