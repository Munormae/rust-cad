use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals:   Vec<[f32; 3]>,
    pub indices:   Vec<u32>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool { self.positions.is_empty() || self.indices.is_empty() }
}