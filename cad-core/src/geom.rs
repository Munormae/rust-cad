use serde::{Deserialize, Serialize};

// math + truck
use cgmath::Point2;
use truck_geometry::prelude::*; // BSplineCurve, KnotVec, NurbsCurve, трейты

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pt2 {
    pub x: f32,
    pub y: f32,
}
impl Pt2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityKind {
    LineSeg {
        a: Pt2,
        b: Pt2,
    },
    Arc {
        center: Pt2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
    },
    Polyline {
        pts: Vec<Pt2>,
        closed: bool,
    },

    /// 2D NURBS/BSpline
    NurbsCurve2D {
        degree: usize,
        knots: Vec<f64>,
        ctrl_pts: Vec<Pt2>,
        weights: Option<Vec<f64>>,
    },

    /// Текстовая аннотация
    Text {
        pos: Pt2,
        content: String,
        height: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub id: u64,
    pub layer: String,
    pub kind: EntityKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Style {
    pub stroke_px: f32,
}
impl Default for Style {
    fn default() -> Self {
        Self { stroke_px: 1.5 }
    }
}

/// Демонстрационный BSpline (без весов)
pub fn demo_nurbs() -> BSplineCurve<Point2<f64>> {
    let ctrl: Vec<Point2<f64>> = vec![
        Point2::new(0.0, 0.0),
        Point2::new(50.0, 100.0),
        Point2::new(100.0, 0.0),
    ];
    let knots = KnotVec::bezier_knot(3);
    // В truck порядок аргументов такой: new(knot_vec, ctrl_pts)
    BSplineCurve::new(knots, ctrl)
}
