use anyhow::{Context, Result};
use ifc_rs::IFC;
use std::path::Path;

use crate::model3d::{Model3D, Project3D};

/// Load an IFC file into the lightweight project container used by the viewer.
pub fn import_ifc(path: &str) -> Result<Project3D> {
    let ifc = IFC::from_file(path)
        .with_context(|| format!("failed to parse IFC file at `{}`", path))?;

    let mut model = Model3D::default();
    model.name = infer_name(&ifc, path);

    // TODO: convert the parsed IFC geometry into Element3D entries.
    Ok(Project3D { models: vec![model] })
}

fn infer_name(ifc: &IFC, path: &str) -> String {
    let header_name = ifc.header.name.name.0.trim();
    if !header_name.is_empty() {
        return header_name.to_owned();
    }

    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("IFC Model")
        .to_owned()
}
