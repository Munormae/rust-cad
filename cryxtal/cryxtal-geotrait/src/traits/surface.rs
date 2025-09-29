use super::*;

type Tuple = (f64, f64);
pub trait ParametricSurface: Clone {
    type Point;
    type Vector: Zero + Copy;
    fn subs(&self, u: f64, v: f64) -> Self::Point;
    fn uder(&self, u: f64, v: f64) -> Self::Vector;
    fn vder(&self, u: f64, v: f64) -> Self::Vector;
    fn uuder(&self, u: f64, v: f64) -> Self::Vector;
    fn uvder(&self, u: f64, v: f64) -> Self::Vector;
    fn vvder(&self, u: f64, v: f64) -> Self::Vector;
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector;
    fn ders(&self, max_order: usize, u: f64, v: f64) -> SurfaceDers<Self::Vector> {
        let mut ders = SurfaceDers::new(max_order);
        (0..=max_order)
            .for_each(|m| (0..=max_order - m).for_each(|n| ders[m][n] = self.der_mn(m, n, u, v)));
        ders
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        use Bound::Unbounded as X;
        ((X, X), (X, X))
    }

    #[inline(always)]
    fn try_range_tuple(&self) -> (Option<Tuple>, Option<Tuple>) {
        let ((u0, u1), (v0, v1)) = self.parameter_range();
        (
            bound2opt(u0).and_then(move |u0| bound2opt(u1).map(move |u1| (u0, u1))),
            bound2opt(v0).and_then(move |v0| bound2opt(v1).map(move |v1| (v0, v1))),
        )
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        None
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        None
    }
}

impl<S: ParametricSurface> ParametricSurface for &S {
    type Point = S::Point;
    type Vector = S::Vector;
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        (*self).subs(u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        (*self).uder(u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        (*self).vder(u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        (*self).uuder(u, v)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        (*self).uvder(u, v)
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        (*self).vvder(u, v)
    }
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        (*self).der_mn(m, n, u, v)
    }
    #[inline(always)]
    fn ders(&self, max_order: usize, u: f64, v: f64) -> SurfaceDers<Self::Vector> {
        (*self).ders(max_order, u, v)
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (*self).parameter_range()
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        (*self).u_period()
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        (*self).v_period()
    }
}

impl<S: ParametricSurface> ParametricSurface for Box<S> {
    type Point = S::Point;
    type Vector = S::Vector;
    #[inline(always)]
    fn subs(&self, u: f64, v: f64) -> Self::Point {
        (**self).subs(u, v)
    }
    #[inline(always)]
    fn uder(&self, u: f64, v: f64) -> Self::Vector {
        (**self).uder(u, v)
    }
    #[inline(always)]
    fn vder(&self, u: f64, v: f64) -> Self::Vector {
        (**self).vder(u, v)
    }
    #[inline(always)]
    fn uuder(&self, u: f64, v: f64) -> Self::Vector {
        (**self).uuder(u, v)
    }
    #[inline(always)]
    fn uvder(&self, u: f64, v: f64) -> Self::Vector {
        (**self).uvder(u, v)
    }
    #[inline(always)]
    fn vvder(&self, u: f64, v: f64) -> Self::Vector {
        (**self).vvder(u, v)
    }
    #[inline(always)]
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        (**self).der_mn(m, n, u, v)
    }
    #[inline(always)]
    fn ders(&self, max_order: usize, u: f64, v: f64) -> SurfaceDers<Self::Vector> {
        (**self).ders(max_order, u, v)
    }
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (**self).parameter_range()
    }
    #[inline(always)]
    fn u_period(&self) -> Option<f64> {
        (**self).u_period()
    }
    #[inline(always)]
    fn v_period(&self) -> Option<f64> {
        (**self).v_period()
    }
}

pub trait ParametricSurface2D: ParametricSurface<Point = Point2, Vector = Vector2> {}
impl<S: ParametricSurface<Point = Point2, Vector = Vector2>> ParametricSurface2D for S {}

pub trait ParametricSurface3D: ParametricSurface<Point = Point3, Vector = Vector3> {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        self.uder(u, v).cross(self.vder(u, v)).normalize()
    }
    fn normal_uder(&self, u: f64, v: f64) -> Vector3 {
        let uder = self.uder(u, v);
        let vder = self.vder(u, v);
        let uuder = self.uuder(u, v);
        let uvder = self.uvder(u, v);
        let cross = uder.cross(vder);
        let cross_uder = uuder.cross(vder) + uder.cross(uvder);
        let abs = cross.magnitude();
        let abs_uder = cross.dot(cross_uder) / abs;
        (cross_uder * abs - cross * abs_uder) / (abs * abs)
    }
    fn normal_vder(&self, u: f64, v: f64) -> Vector3 {
        let uder = self.uder(u, v);
        let vder = self.vder(u, v);
        let uvder = self.uvder(u, v);
        let vvder = self.vvder(u, v);
        let cross = uder.cross(vder);
        let cross_vder = uvder.cross(vder) + uder.cross(vvder);
        let abs = cross.magnitude();
        let abs_vder = cross.dot(cross_vder) / abs;
        (cross_vder * abs - cross * abs_vder) / (abs * abs)
    }
}

impl<S: ParametricSurface3D> ParametricSurface3D for &S {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        (*self).normal(u, v)
    }
}

impl<S: ParametricSurface3D> ParametricSurface3D for Box<S> {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        (**self).normal(u, v)
    }
}

pub trait BoundedSurface: ParametricSurface {
    #[inline(always)]
    fn range_tuple(&self) -> ((f64, f64), (f64, f64)) {
        let (urange, vrange) = self.try_range_tuple();
        (
            urange.expect(UNBOUNDED_ERROR),
            vrange.expect(UNBOUNDED_ERROR),
        )
    }
}

impl<S: BoundedSurface> BoundedSurface for &S {}

impl<S: BoundedSurface> BoundedSurface for Box<S> {}

pub trait IncludeCurve<C: ParametricCurve> {
    fn include(&self, curve: &C) -> bool;
}

pub trait ParameterDivision2D {
    fn parameter_division(&self, range: ((f64, f64), (f64, f64)), tol: f64)
        -> (Vec<f64>, Vec<f64>);
}

impl<S: ParameterDivision2D> ParameterDivision2D for &S {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        (*self).parameter_division(range, tol)
    }
}

impl<S: ParameterDivision2D> ParameterDivision2D for Box<S> {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        (**self).parameter_division(range, tol)
    }
}
