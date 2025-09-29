use cgmath::*;
use serde::*;
use std::cmp::Ordering;
use std::ops::Index;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BoundingBox<V>(V, V);

pub trait Bounded:
    Copy + MetricSpace<Metric = Self::Scalar> + Index<usize, Output = Self::Scalar> + PartialEq
{
    type Scalar: BaseFloat;
    type Vector;
    #[doc(hidden)]
    fn infinity() -> Self;
    #[doc(hidden)]
    fn neg_infinity() -> Self;
    #[doc(hidden)]
    fn max(self, other: Self) -> Self;
    #[doc(hidden)]
    fn min(self, other: Self) -> Self;
    #[doc(hidden)]
    fn max_component(one: Self::Vector) -> Self::Scalar;
    #[doc(hidden)]
    fn diagonal(self, other: Self) -> Self::Vector;
    #[doc(hidden)]
    fn mid(self, other: Self) -> Self;
}

macro_rules! pr2 {
    ($a: expr, $b: expr) => {
        $b
    };
}
macro_rules! impl_bounded {
    ($typename: ident, $vectortype: ident, $($num: expr),*) => {
        impl<S: BaseFloat> Bounded for $typename<S> {
            type Scalar = S;
            type Vector = $vectortype<S>;
            fn infinity() -> $typename<S> {
                $typename::new($(pr2!($num, S::infinity())),*)
            }
            fn neg_infinity() -> $typename<S> {
                $typename::new($(pr2!($num, S::neg_infinity())),*)
            }
            fn max(self, other: Self) -> Self {
                $typename::new(
                    $(
                        if self[$num] < other[$num] {
                            other[$num]
                        } else {
                            self[$num]
                        }
                    ),*
                )
            }
            fn min(self, other: Self) -> Self {
                $typename::new(
                    $(
                        if self[$num] > other[$num] {
                            other[$num]
                        } else {
                            self[$num]
                        }
                    ),*
                )
            }
            fn max_component(one: Self::Vector) -> S {
                let mut max = S::neg_infinity();
                $(if max < one[$num] { max = one[$num] })*
                max
            }
            fn diagonal(self, other: Self) -> Self::Vector { self - other }
            fn mid(self, other: Self) -> Self {
                self + (other - self) / (S::one() + S::one())
            }
        }
    };
}
impl_bounded!(Vector1, Vector1, 0);
impl_bounded!(Point1, Vector1, 0);
impl_bounded!(Vector2, Vector2, 0, 1);
impl_bounded!(Point2, Vector2, 0, 1);
impl_bounded!(Vector3, Vector3, 0, 1, 2);
impl_bounded!(Point3, Vector3, 0, 1, 2);
impl_bounded!(Vector4, Vector4, 0, 1, 2, 3);

impl<V: Bounded> Default for BoundingBox<V> {
    #[inline(always)]
    fn default() -> Self {
        BoundingBox(V::infinity(), V::neg_infinity())
    }
}

impl<V: Bounded> BoundingBox<V> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn push(&mut self, point: V) {
        self.0 = self.0.min(point);
        self.1 = self.1.max(point);
    }

    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0[0] > self.1[0]
    }

    #[inline(always)]
    pub const fn max(self) -> V {
        self.1
    }

    #[inline(always)]
    pub const fn min(self) -> V {
        self.0
    }

    #[inline(always)]
    pub fn diagonal(self) -> V::Vector {
        self.1.diagonal(self.0)
    }

    #[inline(always)]
    pub fn diameter(self) -> V::Scalar {
        match self.is_empty() {
            true => num_traits::Float::neg_infinity(),
            false => self.0.distance(self.1),
        }
    }

    #[inline(always)]
    pub fn size(self) -> V::Scalar {
        V::max_component(self.diagonal())
    }

    #[inline(always)]
    pub fn center(self) -> V {
        self.0.mid(self.1)
    }

    #[inline(always)]
    pub fn contains(self, pt: V) -> bool {
        self + BoundingBox(pt, pt) == self
    }
}

impl<V> BoundingBox<V> where V: Index<usize> {}

impl<'a, V: Bounded> FromIterator<&'a V> for BoundingBox<V> {
    fn from_iter<I: IntoIterator<Item = &'a V>>(iter: I) -> BoundingBox<V> {
        let mut bdd_box = BoundingBox::new();
        let bdd_box_mut = &mut bdd_box;
        iter.into_iter().for_each(move |pt| bdd_box_mut.push(*pt));
        bdd_box
    }
}

impl<V: Bounded> FromIterator<V> for BoundingBox<V> {
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> BoundingBox<V> {
        let mut bdd_box = BoundingBox::new();
        let bdd_box_mut = &mut bdd_box;
        iter.into_iter().for_each(move |pt| bdd_box_mut.push(pt));
        bdd_box
    }
}

impl<V: Bounded> std::ops::AddAssign<&BoundingBox<V>> for BoundingBox<V> {
    #[inline(always)]
    fn add_assign(&mut self, other: &BoundingBox<V>) {
        *self += *other
    }
}

impl<V: Bounded> std::ops::AddAssign<BoundingBox<V>> for BoundingBox<V> {
    #[inline(always)]
    fn add_assign(&mut self, other: BoundingBox<V>) {
        self.0 = self.0.min(other.0);
        self.1 = self.1.max(other.1);
    }
}

impl<V: Bounded> std::ops::Add<&BoundingBox<V>> for &BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn add(self, other: &BoundingBox<V>) -> BoundingBox<V> {
        *self + *other
    }
}

impl<V: Bounded> std::ops::Add<&BoundingBox<V>> for BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn add(self, other: &BoundingBox<V>) -> BoundingBox<V> {
        self + *other
    }
}

impl<V: Bounded> std::ops::Add<BoundingBox<V>> for &BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn add(self, other: BoundingBox<V>) -> BoundingBox<V> {
        other + self
    }
}

impl<V: Bounded> std::ops::Add<BoundingBox<V>> for BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn add(mut self, other: BoundingBox<V>) -> BoundingBox<V> {
        self += other;
        self
    }
}

impl<V: Bounded> std::ops::BitXorAssign<&BoundingBox<V>> for BoundingBox<V> {
    #[inline(always)]
    fn bitxor_assign(&mut self, other: &BoundingBox<V>) {
        *self ^= *other;
    }
}

impl<V: Bounded> std::ops::BitXorAssign<BoundingBox<V>> for BoundingBox<V> {
    #[inline(always)]
    fn bitxor_assign(&mut self, other: BoundingBox<V>) {
        self.0 = self.0.max(other.0);
        self.1 = self.1.min(other.1);
    }
}

impl<V: Bounded> std::ops::BitXor<&BoundingBox<V>> for &BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn bitxor(self, other: &BoundingBox<V>) -> BoundingBox<V> {
        *self ^ *other
    }
}

impl<V: Bounded> std::ops::BitXor<&BoundingBox<V>> for BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn bitxor(self, other: &BoundingBox<V>) -> BoundingBox<V> {
        self ^ *other
    }
}

impl<V: Bounded> std::ops::BitXor<BoundingBox<V>> for &BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn bitxor(self, other: BoundingBox<V>) -> BoundingBox<V> {
        other ^ self
    }
}

impl<V: Bounded> std::ops::BitXor<BoundingBox<V>> for BoundingBox<V> {
    type Output = BoundingBox<V>;

    #[inline(always)]
    fn bitxor(mut self, other: BoundingBox<V>) -> BoundingBox<V> {
        self ^= other;
        self
    }
}

impl<V: Bounded> PartialOrd for BoundingBox<V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let max = self + other;
        match (self == &max, other == &max) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }
}
