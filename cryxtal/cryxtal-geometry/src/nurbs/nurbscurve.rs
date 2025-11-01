use super::*;
use crate::errors::Error;
use crate::Result;
use cryxtal_base::bounding_box::{Bounded, BoundingBox};
use cryxtal_base::cgmath64::{EuclideanSpace, Homogeneous, InnerSpace, MetricSpace};
use cryxtal_base::cgmath_extend_traits::rat_der;
use cryxtal_base::hash::HashGen;
use cryxtal_base::tolerance::Tolerance;
use cryxtal_geotrait::algo;
use cryxtal_geotrait::{
    BoundedCurve, Concat, ConcatError, Cut, Invertible, ParameterDivision1D, ParameterRange,
    ParameterTransform, ParametricCurve, SPHint1D, SearchNearestParameter, SearchParameter,
    Transformed, D1,
};

impl<V> NurbsCurve<V> {
    #[inline(always)]
    pub const fn new(curve: BSplineCurve<V>) -> Self {
        NurbsCurve(curve)
    }

    #[inline(always)]
    pub const fn non_rationalized(&self) -> &BSplineCurve<V> {
        &self.0
    }

    #[inline(always)]
    pub fn into_non_rationalized(self) -> BSplineCurve<V> {
        self.0
    }

    #[inline(always)]
    pub const fn knot_vec(&self) -> &KnotVec {
        &self.0.knot_vec
    }

    #[inline(always)]
    pub fn knot(&self, idx: usize) -> f64 {
        self.0.knot_vec[idx]
    }

    #[inline(always)]
    pub const fn control_points(&self) -> &Vec<V> {
        &self.0.control_points
    }

    #[inline(always)]
    pub fn control_point(&self, idx: usize) -> &V {
        &self.0.control_points[idx]
    }

    #[inline(always)]
    pub fn control_point_mut(&mut self, idx: usize) -> &mut V {
        &mut self.0.control_points[idx]
    }

    #[inline(always)]
    pub fn control_points_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.control_points.iter_mut()
    }

    #[inline(always)]
    pub fn transform_control_points<F: FnMut(&mut V)>(&mut self, f: F) {
        self.0.transform_control_points(f)
    }

    #[inline(always)]
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    #[inline(always)]
    pub fn is_clamped(&self) -> bool {
        self.0.knot_vec.is_clamped(self.0.degree())
    }

    #[inline(always)]
    pub fn knot_normalize(&mut self) -> &mut Self {
        self.0.knot_vec.try_normalize().unwrap();
        self
    }

    #[inline(always)]
    pub fn knot_translate(&mut self, x: f64) -> &mut Self {
        self.0.knot_vec.translate(x);
        self
    }
}

impl<V: Homogeneous<Scalar = f64>> NurbsCurve<V> {
    #[inline(always)]
    pub fn try_from_bspline_and_weights(
        curve: BSplineCurve<V::Point>,
        weights: Vec<f64>,
    ) -> Result<Self> {
        let BSplineCurve {
            knot_vec,
            control_points,
        } = curve;
        if control_points.len() != weights.len() {
            return Err(Error::DifferentLength);
        }
        let control_points = control_points
            .into_iter()
            .zip(weights)
            .map(|(pt, w)| V::from_point_weight(pt, w))
            .collect();
        Ok(Self(BSplineCurve::new_unchecked(knot_vec, control_points)))
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> NurbsCurve<V> {
    #[inline(always)]
    pub fn get_closure(&self) -> impl Fn(f64) -> V::Point + '_ {
        move |t| self.subs(t)
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> NurbsCurve<V>
where
    V::Point: Tolerance,
{
    pub fn is_const(&self) -> bool {
        let pt = self.0.control_points[0].to_point();
        self.0
            .control_points
            .iter()
            .all(move |vec| vec.to_point().near(&pt))
    }

    #[inline(always)]
    pub fn near_as_curve(&self, other: &Self) -> bool {
        self.0
            .sub_near_as_curve(&other.0, 2, move |x, y| x.to_point().near(&y.to_point()))
    }

    #[inline(always)]
    pub fn near2_as_curve(&self, other: &Self) -> bool {
        self.0
            .sub_near_as_curve(&other.0, 2, move |x, y| x.to_point().near2(&y.to_point()))
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> NurbsCurve<V> {
    pub fn add_knot(&mut self, x: f64) -> &mut Self {
        self.0.add_knot(x);
        self
    }

    pub fn remove_knot(&mut self, idx: usize) -> &mut Self {
        let _ = self.try_remove_knot(idx);
        self
    }

    pub fn try_remove_knot(&mut self, idx: usize) -> Result<&mut Self> {
        self.0.try_remove_knot(idx)?;
        Ok(self)
    }

    pub fn elevate_degree(&mut self) -> &mut Self {
        self.0.elevate_degree();
        self
    }

    #[inline(always)]
    pub fn clamp(&mut self) -> &mut Self {
        self.0.clamp();
        self
    }

    pub fn optimize(&mut self) -> &mut Self {
        self.0.optimize();
        self
    }

    pub fn syncro_degree(&mut self, other: &mut Self) {
        let (degree0, degree1) = (self.degree(), other.degree());
        for _ in degree0..degree1 {
            self.elevate_degree();
        }
        for _ in degree1..degree0 {
            other.elevate_degree();
        }
    }

    pub fn syncro_knots(&mut self, other: &mut Self) {
        self.0.syncro_knots(&mut other.0)
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> ParameterTransform
    for NurbsCurve<V>
{
    #[inline(always)]
    fn parameter_transform(&mut self, scalar: f64, r#move: f64) -> &mut Self {
        self.0.parameter_transform(scalar, r#move);
        self
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> Cut for NurbsCurve<V> {
    #[inline(always)]
    fn cut(&mut self, t: f64) -> Self {
        NurbsCurve(self.0.cut(t))
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> Concat<NurbsCurve<V>>
    for NurbsCurve<V>
where
    <V as Homogeneous>::Point: Debug,
{
    type Output = NurbsCurve<V>;
    fn try_concat(
        &self,
        other: &Self,
    ) -> std::result::Result<Self, ConcatError<<V as Homogeneous>::Point>> {
        let curve0 = self.clone();
        let mut curve1 = other.clone();
        let w0 = curve0.0.control_points.last().unwrap().weight();
        let w1 = curve1.0.control_points[0].weight();
        curve1.transform_control_points(|pt| *pt *= w0 / w1);
        match curve0.0.try_concat(&curve1.0) {
            Ok(curve) => Ok(NurbsCurve::new(curve)),
            Err(err) => Err(err.point_map(|v| v.to_point())),
        }
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> NurbsCurve<V>
where
    V::Point: Tolerance,
{
    pub fn make_locally_injective(&mut self) -> &mut Self {
        let mut iter = self.0.bezier_decomposition().into_iter();
        while let Some(bezier) = iter.next().map(NurbsCurve::new) {
            if !bezier.is_const() {
                *self = bezier;
                break;
            }
        }
        let mut x = 0.0;
        for mut bezier in iter.map(NurbsCurve::new) {
            if bezier.is_const() {
                x += bezier.0.knot_vec.range_length();
            } else {
                let s0 = self.0.control_points.last().unwrap().weight();
                let s1 = bezier.0.control_points[0].weight();
                bezier
                    .0
                    .control_points
                    .iter_mut()
                    .for_each(move |vec| *vec *= s0 / s1);
                self.concat(bezier.knot_translate(-x));
            }
        }
        self
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> ParameterDivision1D
    for NurbsCurve<V>
where
    V::Point: MetricSpace<Metric = f64> + HashGen<f64>,
{
    type Point = V::Point;
    #[inline(always)]
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<V::Point>) {
        algo::curve::parameter_division(self, range, tol)
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> SearchNearestParameter<D1>
    for NurbsCurve<V>
where
    V::Point: MetricSpace<Metric = f64>,
    <V::Point as EuclideanSpace>::Diff: InnerSpace + Tolerance,
{
    type Point = V::Point;

    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        point: V::Point,
        hint: H,
        trial: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(hint) => hint,
            SPHint1D::Range(x, y) => {
                algo::curve::presearch(self, point, (x, y), PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_nearest_parameter(self, point, hint, trial)
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> SearchParameter<D1>
    for NurbsCurve<V>
where
    V::Point: MetricSpace<Metric = f64>,
    <V::Point as EuclideanSpace>::Diff: InnerSpace + Tolerance,
{
    type Point = V::Point;
    #[inline(always)]
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        point: V::Point,
        hint: H,
        trial: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(hint) => hint,
            SPHint1D::Range(x, y) => {
                algo::curve::presearch(self, point, (x, y), PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_parameter(self, point, hint, trial)
    }
}

impl<V: Homogeneous<Scalar = f64>> NurbsCurve<V>
where
    V::Point: Bounded<Scalar = f64>,
{
    #[inline(always)]
    pub fn roughly_bounding_box(&self) -> BoundingBox<V::Point> {
        self.0.control_points.iter().map(|p| p.to_point()).collect()
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> ParametricCurve for NurbsCurve<V> {
    type Point = V::Point;
    type Vector = <V::Point as EuclideanSpace>::Diff;
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        self.0.ders(n, t).rat_ders()[n]
    }
    fn ders(&self, n: usize, t: f64) -> crate::prelude::CurveDers<Self::Vector> {
        self.0.ders(n, t).rat_ders()
    }
    #[inline(always)]
    fn subs(&self, t: f64) -> Self::Point {
        self.0.subs(t).to_point()
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Self::Vector {
        rat_der(&[self.0.subs(t), self.0.der(t)])
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Self::Vector {
        rat_der(&[self.0.subs(t), self.0.der(t), self.0.der2(t)])
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (
            Bound::Included(self.0.knot_vec[0]),
            Bound::Included(self.0.knot_vec[self.0.knot_vec.len() - 1]),
        )
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> BoundedCurve for NurbsCurve<V> {}

impl<V: Clone> Invertible for NurbsCurve<V> {
    #[inline(always)]
    fn invert(&mut self) {
        self.0.invert();
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        let mut curve = self.0.clone();
        curve.invert();
        NurbsCurve(curve)
    }
}

impl<M, V: Copy> Transformed<M> for NurbsCurve<V>
where
    M: Copy + std::ops::Mul<V, Output = V>,
{
    #[inline(always)]
    fn transform_by(&mut self, trans: M) {
        self.0
            .control_points
            .iter_mut()
            .for_each(move |v| *v = trans * *v)
    }
}

impl<V: Homogeneous<Scalar = f64>> From<BSplineCurve<V::Point>> for NurbsCurve<V> {
    #[inline(always)]
    fn from(bspcurve: BSplineCurve<V::Point>) -> NurbsCurve<V> {
        NurbsCurve::new(BSplineCurve::new_unchecked(
            bspcurve.knot_vec,
            bspcurve
                .control_points
                .into_iter()
                .map(V::from_point)
                .collect(),
        ))
    }
}
