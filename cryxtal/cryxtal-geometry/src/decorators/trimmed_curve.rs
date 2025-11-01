use crate::prelude::*;
use std::ops::Bound;

impl<C> TrimmedCurve<C>
where
    C: ParametricCurve,
{
    #[inline(always)]
    pub const fn new(curve: C, range: (f64, f64)) -> Self {
        Self { curve, range }
    }
}

impl<C> ParametricCurve for TrimmedCurve<C>
where
    C: ParametricCurve,
{
    type Point = C::Point;
    type Vector = C::Vector;

    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        self.curve.der_n(n, t)
    }

    #[inline(always)]
    fn subs(&self, t: f64) -> Self::Point {
        self.curve.subs(t)
    }

    #[inline(always)]
    fn der(&self, t: f64) -> Self::Vector {
        self.curve.der(t)
    }

    #[inline(always)]
    fn der2(&self, t: f64) -> Self::Vector {
        self.curve.der2(t)
    }

    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(self.range.0), Bound::Included(self.range.1))
    }
}

impl<C> BoundedCurve for TrimmedCurve<C> where C: ParametricCurve {}

impl<C> Cut for TrimmedCurve<C>
where
    C: ParametricCurve + Clone,
{
    fn cut(&mut self, t: f64) -> Self {
        let (t0, t1) = self.range;
        self.range = (t0, t);
        Self::new(self.curve.clone(), (t, t1))
    }
}

impl<C> SearchParameter<D1> for TrimmedCurve<C>
where
    C: SearchParameter<D1>,
{
    type Point = C::Point;
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        pt: C::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        self.curve.search_parameter(pt, hint, trials)
    }
}

impl<C> SearchNearestParameter<D1> for TrimmedCurve<C>
where
    C: SearchNearestParameter<D1>,
{
    type Point = C::Point;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        pt: C::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        self.curve.search_nearest_parameter(pt, hint, trials)
    }
}

impl<C> ParameterDivision1D for TrimmedCurve<C>
where
    C: ParameterDivision1D,
{
    type Point = C::Point;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        self.curve.parameter_division(range, tol)
    }
}
