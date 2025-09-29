use std::ops::Bound;
use cryxtal_base::cgmath64::*;

pub mod curve;
pub mod search_parameter;
pub mod surface;

pub use crate::traits::curve::*;
pub use crate::traits::search_parameter::*;
pub use crate::traits::surface::*;

pub type ParameterRange = (Bound<f64>, Bound<f64>);
fn bound2opt<T>(x: Bound<T>) -> Option<T> {
    match x {
        Bound::Included(x) => Some(x),
        Bound::Excluded(x) => Some(x),
        Bound::Unbounded => None,
    }
}
const UNBOUNDED_ERROR: &str = "Parameter range is unbounded.";

pub trait Invertible: Clone {
    fn invert(&mut self);
    #[inline(always)]
    fn inverse(&self) -> Self {
        let mut res = self.clone();
        res.invert();
        res
    }
}

impl Invertible for (usize, usize) {
    fn invert(&mut self) {
        *self = (self.1, self.0);
    }
    fn inverse(&self) -> Self {
        (self.1, self.0)
    }
}

impl<P: Clone> Invertible for Vec<P> {
    #[inline(always)]
    fn invert(&mut self) {
        self.reverse();
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        self.iter().rev().cloned().collect()
    }
}

impl<T: Invertible> Invertible for Box<T> {
    #[inline(always)]
    fn invert(&mut self) {
        (**self).invert()
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        Box::new((**self).inverse())
    }
}

pub trait Transformed<T>: Clone {
    fn transform_by(&mut self, trans: T);
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        let mut res = self.clone();
        res.transform_by(trans);
        res
    }
}

impl<S: Transformed<T>, T> Transformed<T> for Box<S> {
    #[inline(always)]
    fn transform_by(&mut self, trans: T) {
        (**self).transform_by(trans)
    }
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        Box::new((**self).transformed(trans))
    }
}

macro_rules! impl_transformed {
    ($point: ty, $matrix: ty) => {
        impl Transformed<$matrix> for $point {
            #[inline(always)]
            fn transform_by(&mut self, trans: $matrix) {
                *self = trans.transform_point(*self)
            }
            #[inline(always)]
            fn transformed(&self, trans: $matrix) -> Self {
                trans.transform_point(*self)
            }
        }
    };
}
impl_transformed!(Point2, Matrix3);
impl_transformed!(Point3, Matrix3);
impl_transformed!(Point3, Matrix4);

pub trait ToSameGeometry<T> {
    fn to_same_geometry(&self) -> T;
}
