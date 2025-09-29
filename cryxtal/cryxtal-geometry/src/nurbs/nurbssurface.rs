use super::*;
use cryxtal_geotrait::algo::surface;
use cryxtal_geotrait::ParameterRange;
use crate::errors::Error;
use cryxtal_base::hash::HashGen;
use crate::prelude::{
    BoundedSurface, BoundingBox, D2, EuclideanSpace, Homogeneous, IncludeCurve, Invertible,
    MetricSpace, ParametricSurface, ParametricSurface3D, ParameterDivision2D, Point3, SearchNearestParameter,
    SearchParameter, SPHint2D, Tolerance, Transformed, Vector3, Vector4,
};
use cryxtal_geotrait::{ParametricCurve, BoundedCurve};
use cryxtal_base::cgmath_extend_traits::{multi_rat_der, rat_der};
use cryxtal_base::cgmath64::InnerSpace;
use crate::prelude::algo::surface::{SsnpVector, SspVector};

impl<V> NurbsSurface<V> {
    #[inline(always)]
    pub const fn new(bspsurface: BSplineSurface<V>) -> Self {
        NurbsSurface(bspsurface)
    }

    #[inline(always)]
    pub const fn non_rationalized(&self) -> &BSplineSurface<V> {
        &self.0
    }

    #[inline(always)]
    pub fn non_rationalized_mut(&mut self) -> &mut BSplineSurface<V> {
        &mut self.0
    }

    #[inline(always)]
    pub fn into_non_rationalized(self) -> BSplineSurface<V> {
        self.0
    }

    #[inline(always)]
    pub const fn knot_vecs(&self) -> &(KnotVec, KnotVec) {
        &self.0.knot_vecs
    }

    #[inline(always)]
    pub const fn uknot_vec(&self) -> &KnotVec {
        &self.0.knot_vecs.0
    }

    #[inline(always)]
    pub const fn vknot_vec(&self) -> &KnotVec {
        &self.0.knot_vecs.1
    }

    #[inline(always)]
    pub fn uknot(&self, idx: usize) -> f64 {
        self.0.knot_vecs.0[idx]
    }

    #[inline(always)]
    pub fn vknot(&self, idx: usize) -> f64 {
        self.0.knot_vecs.1[idx]
    }

    #[inline(always)]
    pub const fn control_points(&self) -> &Vec<Vec<V>> {
        &self.0.control_points
    }

    #[inline(always)]
    pub fn control_point(&self, idx0: usize, idx1: usize) -> &V {
        &self.0.control_points[idx0][idx1]
    }

    #[inline(always)]
    pub fn transform_control_points<F: FnMut(&mut V)>(&mut self, f: F) {
        self.0.transform_control_points(f)
    }

    #[inline(always)]
    pub fn ctrl_pts_row_iter(
        &self,
        column_idx: usize,
    ) -> impl ExactSizeIterator<Item = &V> + std::iter::FusedIterator<Item = &V> {
        self.0.ctrl_pts_row_iter(column_idx)
    }

    #[inline(always)]
    pub fn ctrl_pts_column_iter(&self, row_idx: usize) -> std::slice::Iter<'_, V> {
        self.0.control_points[row_idx].iter()
    }

    #[inline(always)]
    pub fn control_point_mut(&mut self, idx0: usize, idx1: usize) -> &mut V {
        &mut self.0.control_points[idx0][idx1]
    }

    #[inline(always)]
    pub fn control_points_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.control_points.iter_mut().flatten()
    }

    #[inline(always)]
    pub fn udegree(&self) -> usize {
        self.0.udegree()
    }

    #[inline(always)]
    pub fn vdegree(&self) -> usize {
        self.0.vdegree()
    }

    #[inline(always)]
    pub fn degrees(&self) -> (usize, usize) {
        (self.udegree(), self.vdegree())
    }

    #[inline(always)]
    pub fn is_clamped(&self) -> bool {
        self.0.is_clamped()
    }

    pub fn swap_axes(&mut self) -> &mut Self
    where
        V: Clone,
    {
        self.0.swap_axes();
        self
    }

    #[inline(always)]
    pub fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        self.0.parameter_range()
    }

    #[inline(always)]
    pub fn column_curve(&self, row_idx: usize) -> NurbsCurve<V>
    where
        V: Clone,
    {
        NurbsCurve(self.0.column_curve(row_idx))
    }

    #[inline(always)]
    pub fn row_curve(&self, column_idx: usize) -> NurbsCurve<V>
    where
        V: Clone,
    {
        NurbsCurve(self.0.row_curve(column_idx))
    }
}

impl<V: Homogeneous<Scalar = f64>> NurbsSurface<V> {
    #[inline(always)]
    pub fn try_from_bspline_and_weights(
        surface: BSplineSurface<V::Point>,
        weights: Vec<Vec<f64>>,
    ) -> Result<Self> {
        let BSplineSurface {
            knot_vecs,
            control_points,
        } = surface;
        if control_points.len() != weights.len() {
            return Err(Error::DifferentLength);
        }
        let control_points = control_points
            .into_iter()
            .zip(weights)
            .map(|(control_points, weights)| {
                if control_points.len() != weights.len() {
                    return Err(Error::DifferentLength);
                }
                Ok(control_points
                    .into_iter()
                    .zip(weights)
                    .map(|(pt, w)| V::from_point_weight(pt, w))
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(BSplineSurface::new_unchecked(
            knot_vecs,
            control_points,
        )))
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> NurbsSurface<V> {
    #[inline(always)]
    pub fn get_closure(&self) -> impl Fn(f64, f64) -> V::Point + '_ {
        move |u, v| self.subs(u, v)
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> NurbsSurface<V>
where
    V::Point: Tolerance,
{
    #[inline(always)]
    pub fn is_const(&self) -> bool {
        let pt = self.0.control_points[0][0].to_point();
        for vec in self.0.control_points.iter().flat_map(|pts| pts.iter()) {
            if !vec.to_point().near(&pt) {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    pub fn near_as_surface(&self, other: &Self) -> bool {
        self.0
            .sub_near_as_surface(&other.0, 2, move |x, y| x.to_point().near(&y.to_point()))
    }

    #[inline(always)]
    pub fn near2_as_surface(&self, other: &Self) -> bool {
        self.0
            .sub_near_as_surface(&other.0, 2, move |x, y| x.to_point().near2(&y.to_point()))
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V> + Tolerance> NurbsSurface<V> {
    #[inline(always)]
    pub fn add_uknot(&mut self, x: f64) -> &mut Self {
        self.0.add_uknot(x);
        self
    }

    #[inline(always)]
    pub fn add_vknot(&mut self, x: f64) -> &mut Self {
        self.0.add_vknot(x);
        self
    }

    #[inline(always)]
    pub fn try_remove_uknot(&mut self, idx: usize) -> Result<&mut Self> {
        match self.0.try_remove_uknot(idx) {
            Ok(_) => Ok(self),
            Err(error) => Err(error),
        }
    }

    #[inline(always)]
    pub fn remove_uknot(&mut self, idx: usize) -> &mut Self {
        self.0.remove_uknot(idx);
        self
    }

    #[inline(always)]
    pub fn try_remove_vknot(&mut self, idx: usize) -> Result<&mut Self> {
        match self.0.try_remove_vknot(idx) {
            Ok(_) => Ok(self),
            Err(error) => Err(error),
        }
    }

    #[inline(always)]
    pub fn remove_vknot(&mut self, idx: usize) -> &mut Self {
        self.0.remove_vknot(idx);
        self
    }

    #[inline(always)]
    pub fn elevate_udegree(&mut self) -> &mut Self {
        self.0.elevate_udegree();
        self
    }

    #[inline(always)]
    pub fn elevate_vdegree(&mut self) -> &mut Self {
        self.0.elevate_vdegree();
        self
    }

    #[inline(always)]
    pub fn syncro_uvdegrees(&mut self) -> &mut Self {
        self.0.syncro_uvdegrees();
        self
    }

    #[inline(always)]
    pub fn syncro_uvknots(&mut self) -> &mut Self {
        self.0.syncro_uvknots();
        self
    }

    #[inline(always)]
    pub fn ucut(&mut self, u: f64) -> Self {
        Self::new(self.0.ucut(u))
    }

    #[inline(always)]
    pub fn vcut(&mut self, v: f64) -> Self {
        Self::new(self.0.vcut(v))
    }

    #[inline(always)]
    pub fn knot_normalize(&mut self) -> &mut Self {
        self.0.knot_normalize();
        self
    }

    #[inline(always)]
    pub fn knot_translate(&mut self, x: f64, y: f64) -> &mut Self {
        self.0.knot_translate(x, y);
        self
    }

    #[inline(always)]
    pub fn optimize(&mut self) -> &mut Self {
        self.0.optimize();
        self
    }

    #[inline(always)]
    pub fn splitted_boundary(&self) -> [NurbsCurve<V>; 4] {
        TryFrom::try_from(
            self.0
                .splitted_boundary()
                .iter()
                .cloned()
                .map(NurbsCurve::new)
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    #[inline(always)]
    pub fn boundary(&self) -> NurbsCurve<V> {
        NurbsCurve::new(self.0.boundary())
    }
}

impl<V: Homogeneous<Scalar = f64>> SearchNearestParameter<D2> for NurbsSurface<V>
where
    Self: ParametricSurface<Point = V::Point, Vector = <V::Point as EuclideanSpace>::Diff>,
    V::Point: EuclideanSpace<Scalar = f64> + MetricSpace<Metric = f64>,
    <V::Point as EuclideanSpace>::Diff: SsnpVector<Point = V::Point>,
{
    type Point = V::Point;

    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: V::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let hint = match hint.into() {
            SPHint2D::Parameter(x, y) => (x, y),
            SPHint2D::Range(range0, range1) => {
                surface::presearch(self, point, (range0, range1), PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                surface::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        surface::search_nearest_parameter(self, point, hint, trials)
    }
}

impl<V: Homogeneous<Scalar = f64>> NurbsSurface<V>
where
    V::Point: Bounded<Scalar = f64>,
{
    #[inline(always)]
    pub fn roughly_bounding_box(&self) -> BoundingBox<V::Point> {
        self.0
            .control_points
            .iter()
            .flatten()
            .map(|pt| pt.to_point())
            .collect()
    }
}

impl<V: Clone> Invertible for NurbsSurface<V> {
    #[inline(always)]
    fn invert(&mut self) {
        self.swap_axes();
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        let mut surface = self.clone();
        surface.swap_axes();
        surface
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> ParametricSurface
    for NurbsSurface<V>
{
    type Point = V::Point;
    type Vector = <V::Point as EuclideanSpace>::Diff;
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        if m < 7 && n < 7 {
            let mut ders = [[V::zero(); 8]; 8];
            (0..=m).for_each(|i| (0..=n).for_each(|j| ders[i][j] = self.0.der_mn(i, j, u, v)));
            let ders = std::array::from_fn::<_, 8, _>(|i| &ders[i][..=n]);
            multi_rat_der(&ders[..=m])
        } else {
            let ders = (0..=m)
                .map(|i| (0..=n).map(|j| self.0.der_mn(i, j, u, v)).collect())
                .collect::<Vec<Vec<_>>>();
            multi_rat_der(&ders)
        }
    }
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> V::Point {
        self.0.subs(u, v).to_point()
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        rat_der(&[self.0.subs(u, v), self.0.uder(u, v)])
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> <V::Point as EuclideanSpace>::Diff {
        rat_der(&[self.0.subs(u, v), self.0.vder(u, v)])
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> <V::Point as EuclideanSpace>::Diff {
        rat_der(&[self.0.subs(u, v), self.0.uder(u, v), self.0.uuder(u, v)])
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> <V::Point as EuclideanSpace>::Diff {
        multi_rat_der(&[
            [self.0.subs(u, v), self.0.vder(u, v)],
            [self.0.uder(u, v), self.0.uvder(u, v)],
        ])
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> <V::Point as EuclideanSpace>::Diff {
        rat_der(&[self.0.subs(u, v), self.0.vder(u, v), self.0.vvder(u, v)])
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        NurbsSurface::parameter_range(self)
    }
}

impl ParametricSurface3D for NurbsSurface<Vector4> {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        let pt = self.0.subs(u, v);
        let ud = self.0.uder(u, v);
        let vd = self.0.vder(u, v);
        rat_der(&[pt, ud]).cross(rat_der(&[pt, vd])).normalize()
    }
}

impl<V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>> ParameterDivision2D
    for NurbsSurface<V>
where
    V::Point: MetricSpace<Metric = f64> + HashGen<f64>,
{
    #[inline(always)]
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        surface::parameter_division(self, range, tol)
    }
}

impl<V> BoundedSurface for NurbsSurface<V> where Self: ParametricSurface {}

impl IncludeCurve<NurbsCurve<Vector3>> for NurbsSurface<Vector3> {
    #[inline(always)]
    fn include(&self, curve: &NurbsCurve<Vector3>) -> bool {
        let pt = curve.subs(curve.knot_vec()[0]);
        let mut hint = match self.search_parameter(pt, None, INCLUDE_CURVE_TRIALS) {
            Some(got) => got,
            None => return false,
        };
        let uknot_vec = self.uknot_vec();
        let vknot_vec = self.vknot_vec();
        let degree = curve.degree() * 6;
        let (knots, _) = curve.knot_vec().to_single_multi();
        for i in 1..knots.len() {
            for j in 1..=degree {
                let p = j as f64 / degree as f64;
                let t = knots[i - 1] * (1.0 - p) + knots[i] * p;
                let pt = curve.subs(t);
                hint = match self.search_parameter(pt, Some(hint), INCLUDE_CURVE_TRIALS) {
                    Some(got) => got,
                    None => return false,
                };
                if !self.subs(hint.0, hint.1).near(&pt)
                    || hint.0 < uknot_vec[0] - TOLERANCE
                    || hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE
                    || hint.1 < vknot_vec[0] - TOLERANCE
                    || hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE
                {
                    return false;
                }
            }
        }
        true
    }
}

impl IncludeCurve<BSplineCurve<Point3>> for NurbsSurface<Vector4> {
    #[inline(always)]
    fn include(&self, curve: &BSplineCurve<Point3>) -> bool {
        let pt = curve.front();
        let mut hint = match self.search_parameter(pt, None, INCLUDE_CURVE_TRIALS) {
            Some(got) => got,
            None => return false,
        };
        let uknot_vec = self.uknot_vec();
        let vknot_vec = self.vknot_vec();
        let degree = curve.degree() * 6;
        let (knots, _) = curve.knot_vec().to_single_multi();
        for i in 1..knots.len() {
            for j in 1..=degree {
                let p = j as f64 / degree as f64;
                let t = knots[i - 1] * (1.0 - p) + knots[i] * p;
                let pt = curve.subs(t);
                hint = match self.search_parameter(pt, Some(hint), INCLUDE_CURVE_TRIALS) {
                    Some(got) => got,
                    None => return false,
                };
                if !self.subs(hint.0, hint.1).near(&pt)
                    || hint.0 < uknot_vec[0] - TOLERANCE
                    || hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE
                    || hint.1 < vknot_vec[0] - TOLERANCE
                    || hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE
                {
                    return false;
                }
            }
        }
        true
    }
}

impl IncludeCurve<NurbsCurve<Vector4>> for NurbsSurface<Vector4> {
    #[inline(always)]
    fn include(&self, curve: &NurbsCurve<Vector4>) -> bool {
        let pt = curve.front();
        let mut hint = match self.search_parameter(pt, None, INCLUDE_CURVE_TRIALS) {
            Some(got) => got,
            None => return false,
        };
        let uknot_vec = self.uknot_vec();
        let vknot_vec = self.vknot_vec();
        let degree = curve.degree() * 6;
        let (knots, _) = curve.knot_vec().to_single_multi();
        for i in 1..knots.len() {
            for j in 1..=degree {
                let p = j as f64 / degree as f64;
                let t = knots[i - 1] * (1.0 - p) + knots[i] * p;
                let pt = curve.subs(t);
                hint = match self.search_parameter(pt, Some(hint), INCLUDE_CURVE_TRIALS) {
                    Some(got) => got,
                    None => return false,
                };
                if !self.subs(hint.0, hint.1).near(&pt)
                    || hint.0 < uknot_vec[0] - TOLERANCE
                    || hint.0 - uknot_vec[0] > uknot_vec.range_length() + TOLERANCE
                    || hint.1 < vknot_vec[0] - TOLERANCE
                    || hint.1 - vknot_vec[0] > vknot_vec.range_length() + TOLERANCE
                {
                    return false;
                }
            }
        }
        true
    }
}

impl<M, V: Copy> Transformed<M> for NurbsSurface<V>
where
    M: Copy + std::ops::Mul<V, Output = V>,
{
    #[inline(always)]
    fn transform_by(&mut self, trans: M) {
        self.0
            .control_points
            .iter_mut()
            .flatten()
            .for_each(move |v| *v = trans * *v)
    }
}

impl<V> SearchParameter<D2> for NurbsSurface<V>
where
    V: Homogeneous<Scalar = f64> + ControlPoint<f64, Diff = V>,
    V::Point: ControlPoint<f64, Diff = <V::Point as EuclideanSpace>::Diff>
        + MetricSpace<Metric = f64>
        + Tolerance,
    <V::Point as EuclideanSpace>::Diff: SspVector<Point = V::Point>,
{
    type Point = V::Point;

    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: V::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        let hint = match hint.into() {
            SPHint2D::Parameter(x, y) => (x, y),
            SPHint2D::Range(range0, range1) => {
                surface::presearch(self, point, (range0, range1), PRESEARCH_DIVISION)
            }
            SPHint2D::None => {
                surface::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        surface::search_parameter(self, point, hint, trials)
    }
}

impl<V: Homogeneous<Scalar = f64>> From<BSplineSurface<V::Point>> for NurbsSurface<V> {
    fn from(bsp: BSplineSurface<V::Point>) -> Self {
        let control_points = bsp
            .control_points
            .into_iter()
            .map(|vec| vec.into_iter().map(|p| V::from_point(p)).collect())
            .collect();
        Self(BSplineSurface {
            knot_vecs: bsp.knot_vecs,
            control_points,
        })
    }
}

#[test]
fn test_include2d() {
    let knot_vec = KnotVec::uniform_knot(2, 3);
    let ctrl_pts = vec![
        vec![
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.05, 0.0, 0.5),
            Vector3::new(0.15, 0.0, 0.3),
            Vector3::new(1.0, 0.0, 1.0),
        ],
        vec![
            Vector3::new(0.0, 0.01, 0.1),
            Vector3::new(0.02, 0.02, 0.1),
            Vector3::new(0.16, 0.12, 0.4),
            Vector3::new(0.7, 0.21, 0.7),
        ],
        vec![
            Vector3::new(0.0, 0.02, 0.4),
            Vector3::new(0.15, 0.3, 0.5),
            Vector3::new(0.6, 0.4, 1.0),
            Vector3::new(0.4, 0.2, 0.4),
        ],
        vec![
            Vector3::new(0.0, 1.0, 1.0),
            Vector3::new(0.1, 1.0, 1.0),
            Vector3::new(0.25, 0.5, 0.5),
            Vector3::new(0.3, 0.3, 0.3),
        ],
    ];
    let surface = BSplineSurface::new((knot_vec.clone(), knot_vec), ctrl_pts);
    use crate::prelude::Vector2;
    let bnd_box = BoundingBox::from_iter(&[Vector2::new(0.2, 0.3), Vector2::new(0.8, 0.6)]);
    let mut curve = surface.sectional_curve(bnd_box);
    curve.control_points_mut().for_each(|pt| *pt *= 3.0);
    let surface = NurbsSurface::new(surface);
    let curve = NurbsCurve::new(curve);
    assert!(surface.include(&curve));
}

#[test]
fn test_include3d() {
    let knot_vec = KnotVec::bezier_knot(2);
    let ctrl_pts = vec![
        vec![
            Vector4::new(-1.0, -1.0, 2.0, 1.0),
            Vector4::new(-1.0, 0.0, 0.0, 1.0),
            Vector4::new(-1.0, 1.0, 2.0, 1.0),
        ],
        vec![
            Vector4::new(0.0, -1.0, 0.0, 1.0),
            Vector4::new(0.0, 0.0, -2.0, 1.0),
            Vector4::new(0.0, 1.0, 0.0, 1.0),
        ],
        vec![
            Vector4::new(1.0, -1.0, 2.0, 1.0),
            Vector4::new(1.0, 0.0, 0.0, 1.0),
            Vector4::new(1.0, 1.0, 2.0, 1.0),
        ],
    ];
    let surface = NurbsSurface::new(BSplineSurface::new((knot_vec.clone(), knot_vec), ctrl_pts));

    let knot_vec = KnotVec::from(vec![
        0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0,
    ]);
    let ctrl_pts = vec![
        // the vector of the indices of control points
        Vector4::new(0.0, -2.0, 2.0, 2.0),
        Vector4::new(1.0, -1.0, 1.0, 1.0),
        Vector4::new(1.0, 0.0, 1.0, 1.0),
        Vector4::new(1.0, 1.0, 1.0, 1.0),
        Vector4::new(0.0, 2.0, 2.0, 2.0),
        Vector4::new(-1.0, 1.0, 1.0, 1.0),
        Vector4::new(-1.0, 0.0, 1.0, 1.0),
        Vector4::new(-1.0, -1.0, 1.0, 1.0),
        Vector4::new(0.0, -2.0, 2.0, 2.0),
    ];
    let mut curve = NurbsCurve::new(BSplineCurve::new(knot_vec, ctrl_pts));
    assert!(surface.include(&curve));
    *curve.control_point_mut(1) += Vector4::new(0.0, 0.0, 0.00001, 0.0);
    assert!(!surface.include(&curve));
}
