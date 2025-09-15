pub trait SPDimension {
    const DIM: usize;
    type Parameter;
    type Hint;
}

#[derive(Clone, Copy, Debug)]
pub enum D1 {}

impl SPDimension for D1 {
    const DIM: usize = 1;
    type Parameter = f64;
    type Hint = SPHint1D;
}

#[derive(Clone, Copy, Debug)]
pub enum D2 {}

impl SPDimension for D2 {
    const DIM: usize = 2;
    type Parameter = (f64, f64);
    type Hint = SPHint2D;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SPHint1D {
    Parameter(f64),
    Range(f64, f64),
    None,
}

impl From<f64> for SPHint1D {
    #[inline(always)]
    fn from(x: f64) -> SPHint1D {
        SPHint1D::Parameter(x)
    }
}

impl From<(f64, f64)> for SPHint1D {
    #[inline(always)]
    fn from(range: (f64, f64)) -> SPHint1D {
        SPHint1D::Range(range.0, range.1)
    }
}

impl From<Option<f64>> for SPHint1D {
    #[inline(always)]
    fn from(x: Option<f64>) -> SPHint1D {
        match x {
            Some(x) => x.into(),
            None => SPHint1D::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SPHint2D {
    Parameter(f64, f64),
    Range((f64, f64), (f64, f64)),
    None,
}

impl From<(f64, f64)> for SPHint2D {
    #[inline(always)]
    fn from(x: (f64, f64)) -> Self {
        Self::Parameter(x.0, x.1)
    }
}

impl From<((f64, f64), (f64, f64))> for SPHint2D {
    #[inline(always)]
    fn from(ranges: ((f64, f64), (f64, f64))) -> Self {
        Self::Range(ranges.0, ranges.1)
    }
}

impl From<Option<(f64, f64)>> for SPHint2D {
    #[inline(always)]
    fn from(x: Option<(f64, f64)>) -> Self {
        match x {
            Some(x) => x.into(),
            None => SPHint2D::None,
        }
    }
}

pub trait SearchParameter<Dim: SPDimension> {
    type Point;
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter>;
}

impl<Dim: SPDimension, T: SearchParameter<Dim>> SearchParameter<Dim> for &T {
    type Point = T::Point;
    #[inline(always)]
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_parameter(*self, point, hint, trials)
    }
}

impl<Dim: SPDimension, T: SearchParameter<Dim>> SearchParameter<Dim> for Box<T> {
    type Point = T::Point;
    #[inline(always)]
    fn search_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_parameter(&**self, point, hint, trials)
    }
}

pub trait SearchNearestParameter<Dim: SPDimension> {
    /// point
    type Point;
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter>;
}

impl<Dim: SPDimension, T: SearchNearestParameter<Dim>> SearchNearestParameter<Dim> for &T {
    type Point = T::Point;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_nearest_parameter(*self, point, hint, trials)
    }
}

impl<Dim: SPDimension, T: SearchNearestParameter<Dim>> SearchNearestParameter<Dim> for Box<T> {
    type Point = T::Point;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<Dim::Hint>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<Dim::Parameter> {
        T::search_nearest_parameter(&**self, point, hint, trials)
    }
}
