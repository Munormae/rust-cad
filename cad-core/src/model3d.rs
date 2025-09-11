//! 3D-модель проекта: ЖБ-элементы и арматура.
//! Геометрия хранится в лёгких представлениях (экструзии/свипы), а точный B-Rep
//! подключаем опционально через фичу `truck-brep`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::mesh::Mesh;

// Truck geometry: кривые/узлы — всегда доступны
use truck_geometry::prelude::*; // BSplineCurve, KnotVec, Point3, ..

// ---- B-Rep: опционально через фичу `truck-brep` ----
#[cfg(feature = "truck-brep")]
use truck_modeling::Solid;

#[cfg(not(feature = "truck-brep"))]
#[derive(Debug, Clone)]
pub struct SolidStub; // лёгкая заглушка, когда truck_modeling не подключён

/// Удобная 3D-точка в мм (без cgmath)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pt3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Pt3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

pub type Id = u64;
pub type MaterialId = u32;

/// Весь проект
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project3D {
    pub models: Vec<Model3D>,
}

/// Одна 3D-модель
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model3D {
    pub name: String,
    pub elements: Vec<Element3D>,
    pub materials: Vec<Material>,
}

/// Геометрическое представление элемента.
///
/// - `Brep` компилируется только с фичей `truck-brep`. Без неё — хранится заглушка,
///   а сериализация всегда пропускает это поле (`#[serde(skip)]`), чтобы не плодить
///   бинарные данные в JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementGeom {
    /// Экструзия 2D-профиля по +Z на `height` (мм)
    Extrusion {
        profile: Vec<crate::Pt2>,
        height: f32,
    },

    /// Свип цилиндрическим профилем радиуса `radius` вдоль 3D-пути
    SweepCylinder { path: Vec<Pt3>, radius: f32 },

    /// Треугольная сетка (позиции и индексы треугольников, в мм)
    Mesh {
        positions: Vec<Pt3>,
        indices: Vec<u32>,
    },

    /// Точное тело (B-Rep). В JSON **не** пишем.
    #[serde(skip)]
    #[cfg(feature = "truck-brep")]
    Brep(Solid),

    /// Заглушка для сборки без `truck-brep`
    #[serde(skip)]
    #[cfg(not(feature = "truck-brep"))]
    Brep(SolidStub),
}

/// Материал
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub kind: MaterialKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialKind {
    Concrete { grade: String },
    Steel { fy_mpa: f32 },
}

/// Арматура
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rebar {
    pub id: Id,
    pub diameter_mm: f32,
    pub path: RebarPath,
    pub count: u32,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RebarPath {
    /// Ломаная
    Polyline(Vec<Pt3>),

    /// B-spline: степень + узлы + КТ (веса опционально)
    Nurbs {
        degree: usize,
        knots: Vec<f64>,
        ctrl_pts: Vec<Pt3>,
        weights: Option<Vec<f64>>,
    },
}

/// Элемент ЖБ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element3D {
    pub id: Id,
    pub name: String,         // марка/тип
    pub xform: [[f32; 4]; 4], // row-major 4x4
    pub geom: ElementGeom,    // тело
    pub material: MaterialId,
    pub rebars: Vec<Rebar>, // арматура
    pub meta: Meta,
}

/// Произвольные свойства
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Meta {
    pub props: BTreeMap<String, String>,
}

// ===================== Конвертеры в Truck =====================

impl RebarPath {
    /// Поднять путь арматуры в B-spline Truck (веса пока игнорируем).
    pub fn to_bspline(&self) -> BSplineCurve<Point3> {
        match self {
            RebarPath::Polyline(pts) => {
                let ctrl: Vec<Point3> = pts
                    .iter()
                    .map(|p| Point3::new(p.x as f64, p.y as f64, p.z as f64))
                    .collect();
                let n = ctrl.len();
                // degree=2, но не выше n-1 и не ниже 1
                let degree = 2usize.min(n.saturating_sub(1)).max(1);
                // open-uniform узловой вектор
                let mut knots = vec![0.0; degree + 1];
                if n > degree + 1 {
                    let inner = n - degree - 1;
                    for i in 1..=inner {
                        knots.push(i as f64 / (inner as f64 + 1.0));
                    }
                }
                knots.extend(std::iter::repeat(1.0).take(degree + 1));
                BSplineCurve::new(KnotVec::from(knots), ctrl)
            }
            RebarPath::Nurbs {
                knots, ctrl_pts, ..
            } => {
                // Truck хранит степень внутри кривой, здесь достаточно узлов и КТ.
                let ctrl: Vec<Point3> = ctrl_pts
                    .iter()
                    .map(|p| Point3::new(p.x as f64, p.y as f64, p.z as f64))
                    .collect();
                BSplineCurve::new(KnotVec::from(knots.clone()), ctrl)
            }
        }
    }
}

impl ElementGeom {
    /// Ссылка на B-Rep (есть только при включённой фиче)
    #[cfg(feature = "truck-brep")]
    pub fn as_brep(&self) -> Option<&Solid> {
        match self {
            ElementGeom::Brep(s) => Some(s),
            _ => None,
        }
    }

    /// Мут-ссылка на B-Rep
    #[cfg(feature = "truck-brep")]
    pub fn as_brep_mut(&mut self) -> Option<&mut Solid> {
        match self {
            ElementGeom::Brep(s) => Some(s),
            _ => None,
        }
    }

    /// Без фичи — всегда None
    #[cfg(not(feature = "truck-brep"))]
    pub fn as_brep(&self) -> Option<()> {
        None
    }
    #[cfg(not(feature = "truck-brep"))]
    pub fn as_brep_mut(&mut self) -> Option<()> {
        None
    }
}

// ===================== Триангуляция для рендера/экспорта =====================

impl Element3D {
    /// Грубая генерация меша из геометрии элемента.
    /// `tube_sides` — число граней трубы для `SweepCylinder`.
    pub fn triangulate(&self, tube_sides: u32) -> Mesh {
        match &self.geom {
            ElementGeom::Extrusion { profile, height } => {
                triangulate_extrusion(profile, *height, self.xform)
            }
            ElementGeom::SweepCylinder { path, radius } => {
                triangulate_tube(path, *radius, tube_sides, self.xform)
            }
            ElementGeom::Mesh { positions, indices } => {
                triangulate_from_mesh(positions, indices, self.xform)
            }
            #[cfg(feature = "truck-brep")]
            ElementGeom::Brep(_) => Mesh::default(), // TODO: триангуляция B-Rep
            #[cfg(not(feature = "truck-brep"))]
            ElementGeom::Brep(_) => Mesh::default(),
        }
    }
}

fn apply_xform(p: [f32; 3], m: [[f32; 4]; 4]) -> [f32; 3] {
    let x = p[0] * m[0][0] + p[1] * m[0][1] + p[2] * m[0][2] + m[0][3];
    let y = p[0] * m[1][0] + p[1] * m[1][1] + p[2] * m[1][2] + m[1][3];
    let z = p[0] * m[2][0] + p[1] * m[2][1] + p[2] * m[2][2] + m[2][3];
    [x, y, z]
}

fn triangulate_extrusion(poly: &Vec<crate::Pt2>, h: f32, xf: [[f32; 4]; 4]) -> Mesh {
    // MVP: предполагаем, что poly — замкнутый и без самопересечений.
    let mut m = Mesh::default();
    if poly.len() < 2 {
        return m;
    }

    // вершины: нижний и верхний контуры
    let base_idx = 0u32;
    for k in 0..2 {
        let z = if k == 0 { 0.0 } else { h };
        for p in poly {
            let v = apply_xform([p.x, p.y, z], xf);
            m.positions.push(v);
        }
    }

    // боковые индексы
    let n = poly.len() as u32;
    for i in 0..(n - 1) {
        let a0 = base_idx + i;
        let b0 = base_idx + i + 1;
        let a1 = base_idx + n + i;
        let b1 = base_idx + n + i + 1;
        m.indices.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
    }

    // крышки (фан к нулевому)
    // низ
    for i in 1..(n - 1) {
        m.indices
            .extend_from_slice(&[base_idx, base_idx + i, base_idx + i + 1]);
    }
    // верх
    let top = base_idx + n;
    for i in 1..(n - 1) {
        m.indices.extend_from_slice(&[top, top + i + 1, top + i]); // обратная ориентация
    }

    // примитивные нормали
    m.normals = vec![[0.0, 1.0, 0.0]; m.positions.len()];
    m
}

fn triangulate_tube(path: &Vec<Pt3>, r: f32, sides: u32, xf: [[f32; 4]; 4]) -> Mesh {
    // MVP: набор колец перпендикулярно оси звена, без скруглений в коленах.
    let mut m = Mesh::default();
    if path.len() < 2 || sides < 3 {
        return m;
    }

    let sides = sides as usize;
    let two_pi = std::f32::consts::TAU;

    // окружность в локальной (side, up) базе
    let mut ring = Vec::with_capacity(sides);
    for i in 0..sides {
        let ang = two_pi * (i as f32) / (sides as f32);
        ring.push([ang.cos() * r, ang.sin() * r]); // (x,y) в перпендикулярной плоскости
    }

    // строим вдоль звеньев
    let mut base = 0u32;
    for seg in 0..(path.len() - 1) {
        let p0 = path[seg];
        let p1 = path[seg + 1];

        let dir = {
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let dz = p1.z - p0.z;
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
            [dx / len, dy / len, dz / len]
        };

        // две перпендикулярные оси к dir
        let up_ref = if dir[2].abs() < 0.9 {
            [0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let side = cross(up_ref, dir);
        let side_n = norm(side);
        let up2 = cross(dir, side_n);

        // два кольца вершин (начало и конец сегмента)
        for k in 0..2 {
            let base_pt = if k == 0 {
                [p0.x, p0.y, p0.z]
            } else {
                [p1.x, p1.y, p1.z]
            };
            for xy in &ring {
                let v = [
                    base_pt[0] + side_n[0] * xy[0] + up2[0] * xy[1],
                    base_pt[1] + side_n[1] * xy[0] + up2[1] * xy[1],
                    base_pt[2] + side_n[2] * xy[0] + up2[2] * xy[1],
                ];
                m.positions.push(apply_xform(v, xf));
            }
        }

        // индексы боковой поверхности
        let n = sides as u32;
        let i0 = base;
        let i1 = base + n;
        for i in 0..n {
            let i_next = (i + 1) % n;
            m.indices.extend_from_slice(&[
                i0 + i,
                i0 + i_next,
                i1 + i_next,
                i0 + i,
                i1 + i_next,
                i1 + i,
            ]);
        }
        base += n * 2;
    }

    m.normals = vec![[0.0, 1.0, 0.0]; m.positions.len()];
    m
}

/// Применить трансформацию к уже готовому мешу (вариант ElementGeom::Mesh).
fn triangulate_from_mesh(positions: &Vec<Pt3>, indices: &Vec<u32>, xf: [[f32; 4]; 4]) -> Mesh {
    let mut m = Mesh::default();
    m.positions.reserve(positions.len());
    for p in positions {
        m.positions.push(apply_xform([p.x, p.y, p.z], xf));
    }
    m.indices.extend_from_slice(indices);

    // простые нормали-заглушки (при необходимости посчитаем позднее)
    m.normals = vec![[0.0, 1.0, 0.0]; m.positions.len()];
    m
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}
