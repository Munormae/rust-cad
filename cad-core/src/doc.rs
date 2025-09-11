use crate::{Entity, EntityKind, Layer, Pt2, Style};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

// math + truck
use cgmath::Point2;
use truck_geometry::prelude::*; // BSplineCurve, KnotVec, трейты

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Grid {
    pub step: f32,
    pub show: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Camera2D {
    pub pan: Pt2,
    pub zoom: f32,
}
impl Default for Camera2D {
    fn default() -> Self {
        Self {
            pan: Pt2 { x: 0.0, y: 0.0 },
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub layers: Vec<Layer>,
    pub entities: Vec<Entity>,
    pub style: Style,
    pub grid: Grid,
    pub camera: Camera2D,
    #[serde(skip)]
    next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            layers: vec![Layer {
                name: "0".into(),
                visible: true,
                locked: false,
            }],
            entities: vec![],
            style: Style::default(),
            grid: Grid {
                step: 10.0,
                show: true,
            },
            camera: Camera2D::default(),
            next_id: 1,
        }
    }
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, mut e: Entity) -> u64 {
        e.id = self.next_id;
        self.next_id += 1;
        self.entities.push(e);
        self.next_id - 1
    }
    pub fn remove_entity(&mut self, id: u64) -> bool {
        if let Some(i) = self.entities.iter().position(|e| e.id == id) {
            self.entities.remove(i);
            true
        } else {
            false
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
    pub fn from_json(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn export_svg(&self, width: f32, height: f32) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>", w=width, h=height);
        let _ = writeln!(
            out,
            "<g stroke='black' fill='none' stroke-width='{}'>",
            self.style.stroke_px
        );

        for e in &self.entities {
            match &e.kind {
                EntityKind::LineSeg { a, b } => {
                    let _ = writeln!(
                        out,
                        "<line x1='{:.3}' y1='{:.3}' x2='{:.3}' y2='{:.3}' />",
                        a.x, a.y, b.x, b.y
                    );
                }
                EntityKind::Arc {
                    center,
                    radius,
                    start_angle,
                    end_angle,
                } => {
                    let (sa, ea) = (*start_angle as f64, *end_angle as f64);
                    let (sx, sy) = (
                        center.x as f64 + (*radius as f64) * sa.cos(),
                        center.y as f64 + (*radius as f64) * sa.sin(),
                    );
                    let (ex, ey) = (
                        center.x as f64 + (*radius as f64) * ea.cos(),
                        center.y as f64 + (*radius as f64) * ea.sin(),
                    );
                    let large = if (ea - sa).abs() > std::f64::consts::PI {
                        1
                    } else {
                        0
                    };
                    let sweep = if ea > sa { 1 } else { 0 };
                    let _ = writeln!(
                        out,
                        "<path d='M {sx} {sy} A {r} {r} 0 {large} {sweep} {ex} {ey}' />",
                        sx = sx,
                        sy = sy,
                        r = radius,
                        large = large,
                        sweep = sweep,
                        ex = ex,
                        ey = ey
                    );
                }
                EntityKind::Polyline { pts, closed } => {
                    if let Some(p0) = pts.first() {
                        let mut d = String::new();
                        let _ = write!(d, "M {} {} ", p0.x, p0.y);
                        for p in &pts[1..] {
                            let _ = write!(d, "L {} {} ", p.x, p.y);
                        }
                        if *closed {
                            let _ = write!(d, "Z");
                        }
                        let _ = writeln!(out, "<path d='{d}' />");
                    }
                }
                EntityKind::NurbsCurve2D { .. } => {
                    let poly = sample_nurbs2d_as_polyline(&e.kind, 64);
                    if !poly.is_empty() {
                        let mut d = String::new();
                        let p0 = &poly[0];
                        let _ = write!(d, "M {} {} ", p0.x, p0.y);
                        for p in &poly[1..] {
                            let _ = write!(d, "L {} {} ", p.x, p.y);
                        }
                        let _ = writeln!(out, "<path d='{d}' />");
                    }
                }
                EntityKind::Text {
                    pos,
                    content,
                    height,
                } => {
                    let _ = writeln!(
                        out,
                        "<text x='{:.3}' y='{:.3}' font-size='{:.3}' fill='black'>{}</text>",
                        pos.x,
                        pos.y,
                        height,
                        xml_escape(content)
                    );
                }
            }
        }

        let _ = writeln!(out, "</g></svg>");
        out
    }
}

/// Семплируем как B-spline (веса игнорируем, чтобы обойти расхождения API).
fn sample_nurbs2d_as_polyline(kind: &EntityKind, samples: usize) -> Vec<Pt2> {
    match kind {
        EntityKind::NurbsCurve2D {
            degree: _,
            knots,
            ctrl_pts,
            ..
        } => {
            let ctrl: Vec<Point2<f64>> = ctrl_pts
                .iter()
                .map(|p| Point2::new(p.x as f64, p.y as f64))
                .collect();

            let kv = KnotVec::from(knots.clone());
            let u0 = *knots.first().unwrap_or(&0.0);
            let u1 = *knots.last().unwrap_or(&1.0);

            let bs = BSplineCurve::<Point2<f64>>::new(kv, ctrl);

            let mut out = Vec::with_capacity(samples + 1);
            for i in 0..=samples {
                let t = u0 + (u1 - u0) * (i as f64) / (samples as f64);
                let p = bs.subs(t);
                out.push(Pt2 {
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            out
        }
        _ => vec![],
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
