// cad-core/src/sheet.rs
//! 2D-листы: вьюпорты (проекции 3D), аннотации и экспорт.

use serde::{Deserialize, Serialize};
use crate::Pt2;
use super::model3d::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub id: Id,
    pub name: String,
    pub size_mm: (f32, f32),       // A1/A2/A3... (мм)
    pub viewports: Vec<Viewport>,
    pub annots: Vec<Annot>,        // общие аннотации (штамп/таблицы)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub id: Id,
    pub element_ref: Id,           // ссылка на Element3D (пока так; потом можно на Model3D)
    pub kind: ViewKind,            // План/Фасад/Разрез/Изо
    pub rect_mm: RectMM,           // прямоугольник окна на листе
    pub scale: f32,                // 1 : scale
    pub clip: Option<Section>,     // секущая плоскость/объём (для разрезов)
    pub annots: Vec<Annot>,        // локальные аннотации к виду
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RectMM { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewKind {
    Plan,                          // вид сверху (Z+)
    ElevX,                         // фасад по X
    ElevY,                         // фасад по Y
    Section,                       // разрез (см. clip)
    Iso,                           // изометрия
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub origin_mm: Pt2,            // точка на листе для позиционирования/оси
    // позже: мировая плоскость/направление, локальная матрица вида и т.п.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Annot {
    Text { pos_mm: Pt2, content: String, h_mm: f32 },
    DimLinear { a_mm: Pt2, b_mm: Pt2, off_mm: f32 },
    DimRadius { c_mm: Pt2, p_mm: Pt2 },
    Leader { pts_mm: Vec<Pt2>, text: String, h_mm: f32 },
    Table { origin_mm: Pt2, rows: Vec<Vec<String>> },
    Block { name: String, at_mm: Pt2 }, // штамп
}
