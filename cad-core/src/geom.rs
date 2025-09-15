use serde::{Deserialize, Serialize};
use truck_geometry::prelude::*; // Point2, BSplineCurve, NurbsCurve, KnotVec, ParametricCurve

// --------------------------- базовые типы (serde/UI) ---------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pt2 {
    pub x: f32,
    pub y: f32,
}
impl Pt2 {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
}

// Линейка сущностей 2D
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityKind {
    LineSeg { a: Pt2, b: Pt2 },

    /// Углы в **радианах**
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

    /// Рациональная/нерциональная кривая: если `weights == None`, это обычный B-сплайн.
    /// degree = порядок - 1 (как обычно), knots — открытый/униформ и т.п. на твоей совести :)
    NurbsCurve2D {
        degree: usize,
        knots: Vec<f64>,
        ctrl_pts: Vec<Pt2>,
        weights: Option<Vec<f64>>,
    },

    /// Текстовая аннотация (для полноты совместимости)
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
    fn default() -> Self { Self { stroke_px: 1.5 } }
}

// --------------------------- конвертеры в truck ---------------------------

impl From<Pt2> for Point2<f64> {
    fn from(p: Pt2) -> Self { Point2::new(p.x as f64, p.y as f64) }
}
impl From<Point2<f64>> for Pt2 {
    fn from(p: Point2<f64>) -> Self { Pt2::new(p.x as f32, p.y as f32) }
}

/// Представление кривой truck'ом
pub enum TruckCurve2 {
    BSpline(BSplineCurve<Point2<f64>>),
    Nurbs(NurbsCurve<Point2<f64>>),
}

impl EntityKind {
    /// Сконструировать truck-кривую (NURBS/BSpline) из нашей сущности.
    /// Для Polyline/Line вернёт degree=1 B-splines,
    /// для Arc — точный квадратичный рациональный NURBS (с разбиением по ≤ 90°).
    pub fn to_truck(&self) -> Option<TruckCurve2> {
        match self {
            EntityKind::LineSeg { a, b } => {
                let ctrl = vec![(*a).into(), (*b).into()];
                let n = ctrl.len();
                // degree=1, open uniform: [0,0,1,1]
                let knots = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
                Some(TruckCurve2::BSpline(BSplineCurve::new(knots, ctrl)))
            }

            EntityKind::Polyline { pts, closed } => {
                if pts.len() < 2 {
                    return None;
                }
                let mut ctrl: Vec<Point2<f64>> = pts.iter().copied().map(Into::into).collect();
                if *closed {
                    ctrl.push(ctrl[0]);
                }
                let n = ctrl.len();
                // degree=1, open uniform с равномерными внутренними узлами
                let mut kv = vec![0.0, 0.0];
                if n > 2 {
                    for i in 1..(n - 1) {
                        kv.push(i as f64 / (n - 1) as f64);
                    }
                }
                kv.extend_from_slice(&[1.0, 1.0]);
                Some(TruckCurve2::BSpline(BSplineCurve::new(KnotVec::from(kv), ctrl)))
            }

            EntityKind::Arc { center, radius, start_angle, end_angle } => {
                let c = (center.x as f64, center.y as f64);
                let r = *radius as f64;
                let a0 = *start_angle as f64;
                let a1 = *end_angle as f64;
                let nurbs = nurbs_arc_deg2(c, r, a0, a1);
                Some(TruckCurve2::Nurbs(nurbs))
            }

            EntityKind::NurbsCurve2D { degree, knots, ctrl_pts, weights } => {
                let ctrl: Vec<Point2<f64>> = ctrl_pts.iter().copied().map(Into::into).collect();
                let kv = KnotVec::from(knots.clone());
                if let Some(w) = weights {
                    let nurbs = NurbsCurve::new(BSplineCurve::new(kv, ctrl), w.clone());
                    Some(TruckCurve2::Nurbs(nurbs))
                } else {
                    Some(TruckCurve2::BSpline(BSplineCurve::new(kv, ctrl)))
                }
            }

            EntityKind::Text { .. } => None,
        }
    }

    /// Сэмплинг кривой для быстрой отрисовки (N экранных шагов).
    pub fn sample(&self, steps: usize) -> Vec<Pt2> {
        match self.to_truck() {
            Some(TruckCurve2::BSpline(c)) => sample_curve_bspline(&c, steps),
            Some(TruckCurve2::Nurbs(c)) => sample_curve_nurbs(&c, steps),
            None => match self {
                // текст не рисуем как кривую
                EntityKind::Text { .. } => vec![],
                // если не удалось собрать BSpline, хотя бы вернём исходные точки полилинии
                EntityKind::Polyline { pts, .. } => pts.clone(),
                // на всякий: прямая как 2 точки
                EntityKind::LineSeg { a, b } => vec![*a, *b],
                // fallback для дуги: дискретизация угла
                EntityKind::Arc { center, radius, start_angle, end_angle } => {
                    let c = (center.x as f64, center.y as f64);
                    let r = *radius as f64;
                    let s = *start_angle as f64;
                    let e = *end_angle as f64;
                    sample_arc(c, r, s, e, steps)
                }
                EntityKind::NurbsCurve2D { .. } => vec![], // не должно случаться
            },
        }
    }
}

// --------------------------- утилиты: дуга как Nurbs2 ---------------------------

/// Точный квадратичный рациональный NURBS для дуги [a0..a1] центра c и радиуса r.
/// Разбивает дугу на сегменты ≤ 90° для устойчивых весов.
/// Формулы: P0, P2 — конец/начало; P1 = (P0+P2)/(2w), где w = cos(Δ/2).
fn nurbs_arc_deg2(c: (f64, f64), r: f64, a0: f64, a1: f64) -> NurbsCurve<Point2<f64>> {
    let mut angs = split_angles(a0, a1);
    if angs.len() < 2 {
        angs = vec![a0, a1];
    }
    let mut ctrl: Vec<Point2<f64>> = Vec::with_capacity(2 * angs.len() + 1);
    let mut wts: Vec<f64> = Vec::with_capacity(2 * angs.len() + 1);

    // первый сегмент
    let (p0, p1, p2, w) = arc_segment_ctrl(c, r, angs[0], angs[1]);
    ctrl.push(p0);
    ctrl.push(p1);
    ctrl.push(p2);
    wts.push(1.0);
    wts.push(w);
    wts.push(1.0);

    // остальные сегменты «пришиваются»
    for k in 1..(angs.len() - 1) {
        let (.., q1, q2, wq) = arc_segment_ctrl(c, r, angs[k], angs[k + 1]);
        ctrl.push(q1);
        ctrl.push(q2);
        wts.push(wq);
        wts.push(1.0);
    }

    // knot-vector для degree=2: [0,0,0, s1, s2, ..., 1,1,1] с равномерными внутренними узлами по сегментам
    let segs = angs.len() - 1;
    let mut kv = vec![0.0, 0.0, 0.0];
    for i in 1..segs {
        kv.push(i as f64 / segs as f64);
    }
    kv.extend_from_slice(&[1.0, 1.0, 1.0]);

    NurbsCurve::new(BSplineCurve::new(KnotVec::from(kv), ctrl), wts)
}

// один дуговой сегмент (≤ 90°): отдаёт P0,P1,P2 и вес средней точки
fn arc_segment_ctrl(c: (f64, f64), r: f64, a0: f64, a1: f64) -> (Point2<f64>, Point2<f64>, Point2<f64>, f64) {
    let (cx, cy) = c;
    let (s0, c0) = a0.sin_cos();
    let (s1, c1) = a1.sin_cos();
    let p0 = Point2::new(cx + r * c0, cy + r * s0);
    let p2 = Point2::new(cx + r * c1, cy + r * s1);

    let dm = 0.5 * (a1 - a0);
    let w = dm.cos(); // вес средней точки
    // средняя точка через формулу (P0 + P2) / (2w)
    let p1 = Point2::new((p0.x + p2.x) / (2.0 * w), (p0.y + p2.y) / (2.0 * w));
    (p0, p1, p2, w)
}

// разбиение на ≤ 90° сегменты
fn split_angles(a0: f64, a1: f64) -> Vec<f64> {
    let mut start = a0;
    let mut v = vec![start];
    let step = std::f64::consts::FRAC_PI_2; // 90°
    let dir = if a1 >= a0 { 1.0 } else { -1.0 };
    while (a1 - start) * dir > step {
        start += dir * step;
        v.push(start);
    }
    v.push(a1);
    v
}

// --------------------------- утилиты: сэмплинг ---------------------------

fn sample_curve_bspline(c: &BSplineCurve<Point2<f64>>, steps: usize) -> Vec<Pt2> {
    let steps = steps.max(2);
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            Pt2::from(c.subs(t))
        })
        .collect()
}

fn sample_curve_nurbs(c: &NurbsCurve<Point2<f64>>, steps: usize) -> Vec<Pt2> {
    let steps = steps.max(2);
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            Pt2::from(c.subs(t))
        })
        .collect()
}

fn sample_arc(c: (f64, f64), r: f64, a0: f64, a1: f64, steps: usize) -> Vec<Pt2> {
    let steps = steps.max(2);
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            let a = a0 + (a1 - a0) * t;
            let (s, co) = a.sin_cos();
            Pt2::new((c.0 + r * co) as f32, (c.1 + r * s) as f32)
        })
        .collect()
}

// --------------------------- демо ---------------------------

/// Демонстрационный B-сплайн Безье (degree=3), чисто для проверки пайплайна
pub fn demo_nurbs() -> BSplineCurve<Point2<f64>> {
    let ctrl = vec![
        Point2::new(0.0, 0.0),
        Point2::new(50.0, 100.0),
        Point2::new(100.0, 0.0),
        Point2::new(150.0, 50.0),
    ];
    let knots = KnotVec::bezier_knot(3);
    BSplineCurve::new(knots, ctrl)
}
