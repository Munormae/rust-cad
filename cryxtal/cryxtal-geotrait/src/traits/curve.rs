use super::*;
use cryxtal_base::assert_near;
use cryxtal_base::{cgmath64::*, hash::HashGen, tolerance::*};
use std::fmt::Debug;
use thiserror::Error;

pub trait Curve {}

pub trait ParametricCurve: Clone {
    type Point;
    type Vector: Zero + Copy;
    fn subs(&self, t: f64) -> Self::Point;
    fn der(&self, t: f64) -> Self::Vector;
    fn der2(&self, t: f64) -> Self::Vector;
    fn der_n(&self, n: usize, t: f64) -> Self::Vector;
    fn ders(&self, n: usize, t: f64) -> CurveDers<Self::Vector> {
        (0..=n).map(|i| self.der_n(i, t)).collect()
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Unbounded, Bound::Unbounded)
    }
    #[inline(always)]
    fn try_range_tuple(&self) -> Option<(f64, f64)> {
        let (x, y) = self.parameter_range();
        bound2opt(x).and_then(move |x| bound2opt(y).map(move |y| (x, y)))
    }

    #[inline(always)]
    fn period(&self) -> Option<f64> {
        None
    }
}

pub trait BoundedCurve: ParametricCurve {
    #[inline(always)]
    fn range_tuple(&self) -> (f64, f64) {
        self.try_range_tuple().expect(UNBOUNDED_ERROR)
    }
    #[inline(always)]
    fn front(&self) -> Self::Point {
        let (x, _) = self.parameter_range();
        self.subs(bound2opt(x).expect(UNBOUNDED_ERROR))
    }
    #[inline(always)]
    fn back(&self) -> Self::Point {
        let (_, y) = self.parameter_range();
        self.subs(bound2opt(y).expect(UNBOUNDED_ERROR))
    }
}

impl ParametricCurve for (usize, usize) {
    type Point = usize;
    type Vector = usize;
    fn der_n(&self, _: usize, _: f64) -> Self::Vector {
        self.1 - self.0
    }
    fn subs(&self, t: f64) -> Self::Point {
        match t < 0.5 {
            true => self.0,
            false => self.1,
        }
    }
    fn der(&self, _: f64) -> Self::Vector {
        self.1 - self.0
    }
    fn der2(&self, _: f64) -> Self::Vector {
        self.1 - self.0
    }
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(0.0), Bound::Included(1.0))
    }
}

impl BoundedCurve for (usize, usize) {}

impl<C: ParametricCurve> ParametricCurve for &C {
    type Point = C::Point;
    type Vector = C::Vector;
    fn subs(&self, t: f64) -> Self::Point {
        (*self).subs(t)
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Self::Vector {
        (*self).der(t)
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Self::Vector {
        (*self).der2(t)
    }
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        (*self).der_n(n, t)
    }
    #[inline(always)]
    fn ders(&self, n: usize, t: f64) -> CurveDers<Self::Vector> {
        (*self).ders(n, t)
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (*self).parameter_range()
    }
    #[inline(always)]
    fn period(&self) -> Option<f64> {
        (*self).period()
    }
}

impl<C: BoundedCurve> BoundedCurve for &C {
    #[inline(always)]
    fn front(&self) -> Self::Point {
        (*self).front()
    }
    #[inline(always)]
    fn back(&self) -> Self::Point {
        (*self).back()
    }
}

impl<C: ParametricCurve> ParametricCurve for Box<C> {
    type Point = C::Point;
    type Vector = C::Vector;
    fn subs(&self, t: f64) -> Self::Point {
        (**self).subs(t)
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Self::Vector {
        (**self).der(t)
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Self::Vector {
        (**self).der2(t)
    }
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        (**self).der_n(n, t)
    }
    #[inline(always)]
    fn ders(&self, n: usize, t: f64) -> CurveDers<Self::Vector> {
        (**self).ders(n, t)
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (**self).parameter_range()
    }
    #[inline(always)]
    fn period(&self) -> Option<f64> {
        (**self).period()
    }
}

impl<C: BoundedCurve> BoundedCurve for Box<C> {
    #[inline(always)]
    fn front(&self) -> Self::Point {
        (**self).front()
    }
    #[inline(always)]
    fn back(&self) -> Self::Point {
        (**self).back()
    }
}

impl<C: Cut> Cut for Box<C> {
    #[inline(always)]
    fn cut(&mut self, t: f64) -> Self {
        Box::new((**self).cut(t))
    }
}

pub trait ParametricCurve2D: ParametricCurve<Point = Point2, Vector = Vector2> {}
impl<C: ParametricCurve<Point = Point2, Vector = Vector2>> ParametricCurve2D for C {}
pub trait ParametricCurve3D: ParametricCurve<Point = Point3, Vector = Vector3> {}
impl<C: ParametricCurve<Point = Point3, Vector = Vector3>> ParametricCurve3D for C {}

pub trait ParameterDivision1D {
    type Point;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>);
}

impl<C: ParameterDivision1D> ParameterDivision1D for &C {
    type Point = C::Point;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        (*self).parameter_division(range, tol)
    }
}

impl<C: ParameterDivision1D> ParameterDivision1D for Box<C> {
    type Point = C::Point;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        (**self).parameter_division(range, tol)
    }
}

pub trait ParameterTransform: BoundedCurve {
    fn parameter_transform(&mut self, scalar: f64, r#move: f64) -> &mut Self;
    fn parameter_transformed(&self, scalar: f64, r#move: f64) -> Self {
        let mut curve = self.clone();
        curve.parameter_transform(scalar, r#move);
        curve
    }

    fn parameter_normalization(&mut self) -> &mut Self {
        let (t0, t1) = self.range_tuple();
        let a = 1.0 / (t1 - t0);
        let b = -t0 * a;
        self.parameter_transform(a, b)
    }
}

impl<C: ParameterTransform> ParameterTransform for Box<C> {
    #[inline(always)]
    fn parameter_transform(&mut self, scalar: f64, r#move: f64) -> &mut Self {
        (**self).parameter_transform(scalar, r#move);
        self
    }
    #[inline(always)]
    fn parameter_transformed(&self, scalar: f64, r#move: f64) -> Self {
        Box::new((**self).parameter_transformed(scalar, r#move))
    }
    #[inline(always)]
    fn parameter_normalization(&mut self) -> &mut Self {
        (**self).parameter_normalization();
        self
    }
}

pub trait Concat<Rhs: BoundedCurve<Point = Self::Point, Vector = Self::Vector>>:
    BoundedCurve
where
    Self::Point: Debug,
{
    type Output: BoundedCurve<Point = Self::Point, Vector = Self::Vector>;

    fn try_concat(&self, rhs: &Rhs) -> Result<Self::Output, ConcatError<Self::Point>>;

    fn concat(&self, rhs: &Rhs) -> Self::Output {
        self.try_concat(rhs).unwrap_or_else(|err| panic!("{}", err))
    }
}

impl<Rhs, C> Concat<Rhs> for Box<C>
where
    Rhs: BoundedCurve<Point = C::Point, Vector = C::Vector>,
    C: Concat<Rhs>,
    C::Point: Debug,
{
    type Output = C::Output;
    fn try_concat(&self, rhs: &Rhs) -> Result<Self::Output, ConcatError<C::Point>> {
        (**self).try_concat(rhs)
    }
    fn concat(&self, rhs: &Rhs) -> Self::Output {
        (**self).concat(rhs)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Error)]
pub enum ConcatError<Point: Debug> {
    #[error("The end parameter {0} of the first curve is different from the start parameter {1} of the second curve.")]
    DisconnectedParameters(f64, f64),

    #[error("The end point {0:?} of the first curve is different from the start point {1:?} of the second curve.")]
    DisconnectedPoints(Point, Point),
}

impl<T: Debug> ConcatError<T> {
    #[inline(always)]
    pub fn point_map<U: Debug, F>(self, f: F) -> ConcatError<U>
    where
        F: Fn(T) -> U,
    {
        match self {
            ConcatError::DisconnectedParameters(a, b) => ConcatError::DisconnectedParameters(a, b),
            ConcatError::DisconnectedPoints(p, q) => ConcatError::DisconnectedPoints(f(p), f(q)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CurveCollector<C> {
    Singleton,

    Curve(C),
}

impl<C> CurveCollector<C> {
    #[inline(always)]
    pub fn try_concat<Rhs>(&mut self, curve: &Rhs) -> Result<&mut Self, ConcatError<C::Point>>
    where
        C: Concat<Rhs, Output = C>,
        C::Point: Debug,
        Rhs: BoundedCurve<Point = C::Point, Vector = C::Vector> + Into<C>,
    {
        match self {
            CurveCollector::Singleton => {
                *self = CurveCollector::Curve(curve.clone().into());
            }
            CurveCollector::Curve(ref mut curve0) => {
                *curve0 = curve0.try_concat(curve)?;
            }
        }
        Ok(self)
    }

    #[inline(always)]
    pub fn concat<Rhs>(&mut self, curve: &Rhs) -> &mut Self
    where
        C: Concat<Rhs, Output = C>,
        C::Point: Debug,
        Rhs: BoundedCurve<Point = C::Point, Vector = C::Vector> + Into<C>,
    {
        self.try_concat(curve)
            .unwrap_or_else(|error| panic!("{}", error))
    }

    #[inline(always)]
    pub fn is_singleton(&self) -> bool {
        match self {
            CurveCollector::Singleton => true,
            CurveCollector::Curve(_) => false,
        }
    }

    #[inline(always)]
    pub fn unwrap(self) -> C {
        match self {
            CurveCollector::Curve(curve) => curve,
            CurveCollector::Singleton => panic!("This curve collector is singleton."),
        }
    }
}

impl<C> From<CurveCollector<C>> for Option<C> {
    #[inline(always)]
    fn from(collector: CurveCollector<C>) -> Option<C> {
        match collector {
            CurveCollector::Singleton => None,
            CurveCollector::Curve(curve) => Some(curve),
        }
    }
}

pub trait Cut: BoundedCurve {
    fn cut(&mut self, t: f64) -> Self;
}

pub fn parameter_transform_random_test<C>(curve: &C, trials: usize)
where
    C: ParameterTransform,
    C::Point: Debug + Tolerance,
    C::Vector: Debug + Tolerance + std::ops::Mul<f64, Output = C::Vector>,
{
    (0..trials).for_each(move |_| exec_parameter_transform_random_test(curve))
}

fn exec_parameter_transform_random_test<C>(curve: &C)
where
    C: ParameterTransform,
    C::Point: Debug + Tolerance,
    C::Vector: Debug + Tolerance + std::ops::Mul<f64, Output = C::Vector>,
{
    let a = rand::random::<f64>() + 0.5;
    let b = rand::random::<f64>() * 2.0;
    let transformed = curve.parameter_transformed(a, b);

    let (t0, t1) = curve.range_tuple();
    assert_near!(transformed.range_tuple().0, t0 * a + b);
    assert_near!(transformed.range_tuple().1, t1 * a + b);
    let p = rand::random::<f64>();
    let t = (1.0 - p) * t0 + p * t1;
    assert_near!(transformed.subs(t * a + b), curve.subs(t));
    assert_near!(transformed.der(t * a + b) * a, curve.der(t));
    assert_near!(transformed.der2(t * a + b) * a * a, curve.der2(t));
    assert_near!(transformed.front(), curve.front());
    assert_near!(transformed.back(), curve.back());
}

pub fn concat_random_test<C0, C1>(curve0: &C0, curve1: &C1, trials: usize)
where
    C0: Concat<C1>,
    C0::Point: Debug + Tolerance,
    C0::Vector: Debug + Tolerance,
    C0::Output: BoundedCurve<Point = C0::Point, Vector = C0::Vector> + Debug,
    C1: BoundedCurve<Point = C0::Point, Vector = C0::Vector>,
{
    (0..trials).for_each(move |_| exec_concat_random_test(curve0, curve1))
}

fn exec_concat_random_test<C0, C1>(curve0: &C0, curve1: &C1)
where
    C0: Concat<C1>,
    C0::Point: Debug + Tolerance,
    C0::Vector: Debug + Tolerance,
    C0::Output: BoundedCurve<Point = C0::Point, Vector = C0::Vector> + Debug,
    C1: BoundedCurve<Point = C0::Point, Vector = C0::Vector>,
{
    let concatted = curve0.try_concat(curve1).unwrap();
    let (t0, t1) = curve0.range_tuple();
    let (_, t2) = curve1.range_tuple();
    assert_near!(concatted.range_tuple().0, t0);
    assert_near!(concatted.range_tuple().1, t2);

    let p = rand::random::<f64>();
    let t = t0 * (1.0 - p) + t1 * p;
    assert_near!(concatted.subs(t), curve0.subs(t));
    assert_near!(concatted.der(t), curve0.der(t));
    assert_near!(concatted.der2(t), curve0.der2(t));
    assert_near!(concatted.front(), curve0.front());

    let p = rand::random::<f64>();
    let t = t1 * (1.0 - p) + t2 * p;
    assert_near!(concatted.subs(t), curve1.subs(t));
    assert_near!(concatted.der(t), curve1.der(t));
    assert_near!(concatted.der2(t), curve1.der2(t));
    assert_near!(concatted.back(), curve1.back());
}

pub fn cut_random_test<C>(curve: &C, trials: usize)
where
    C: Cut,
    C::Point: Debug + Tolerance,
    C::Vector: Debug + Tolerance,
{
    (0..trials).for_each(move |_| exec_cut_random_test(curve))
}

fn exec_cut_random_test<C>(curve: &C)
where
    C: Cut,
    C::Point: Debug + Tolerance,
    C::Vector: Debug + Tolerance,
{
    let mut part0 = curve.clone();
    let (t0, t1) = curve.range_tuple();
    let p = rand::random::<f64>();
    let t = t0 * (1.0 - p) + t1 * p;
    let part1 = part0.cut(t);
    assert_near!(part0.range_tuple().0, t0);
    assert_near!(part0.range_tuple().1, t);
    assert_near!(part1.range_tuple().0, t);
    assert_near!(part1.range_tuple().1, t1);

    let p = rand::random::<f64>();
    let s = t0 * (1.0 - p) + t * p;
    assert_near!(part0.subs(s), curve.subs(s));
    assert_near!(part0.front(), curve.front());
    assert_near!(part0.back(), curve.subs(t));

    let p = rand::random::<f64>();
    let s = t * (1.0 - p) + t1 * p;
    assert_near!(part1.subs(s), curve.subs(s));
    assert_near!(part1.front(), curve.subs(t));
    assert_near!(part1.back(), curve.back());
}
