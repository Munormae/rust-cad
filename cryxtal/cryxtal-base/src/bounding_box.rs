//! Утилиты работы с осевыми ограничивающими параллелепипедами (AABB).

use cgmath::*;
use serde::*;
use std::cmp::Ordering;
use std::ops::Index;

/// Минимальное и максимальное значение векторной точки, задающее AABB.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BoundingBox<V>(V, V);

/// Тип, пригодный к заключению в AABB.
pub trait Bounded:
    Copy + MetricSpace<Metric = Self::Scalar> + Index<usize, Output = Self::Scalar> + PartialEq
{
    /// Скалярный тип координат.
    type Scalar: BaseFloat;
    /// Тип вектора-диагонали.
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
    /// Создаёт пустой AABB.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Расширяет бокс так, чтобы он включил указанную точку.
    #[inline(always)]
    pub fn push(&mut self, point: V) {
        self.0 = self.0.min(point);
        self.1 = self.1.max(point);
    }

    /// Возвращает `true`, если бокс пуст (минимум выше максимума).
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        let delta = self.0.diagonal(self.1);
        V::max_component(delta) > V::Scalar::zero()
    }

    /// Возвращает максимальную точку AABB.
    #[inline(always)]
    pub const fn max(self) -> V {
        self.1
    }

    /// Возвращает минимальную точку AABB.
    #[inline(always)]
    pub const fn min(self) -> V {
        self.0
    }

    /// Вычисляет диагональный вектор.
    #[inline(always)]
    pub fn diagonal(self) -> V::Vector {
        self.1.diagonal(self.0)
    }

    /// Возвращает длину диагонали.
    #[inline(always)]
    pub fn diameter(self) -> V::Scalar {
        match self.is_empty() {
            true => num_traits::Float::neg_infinity(),
            false => self.0.distance(self.1),
        }
    }

    /// Возвращает максимальное смещение по одной координате.
    #[inline(always)]
    pub fn size(self) -> V::Scalar {
        V::max_component(self.diagonal())
    }

    /// Возвращает центр бокса.
    #[inline(always)]
    pub fn center(self) -> V {
        self.0.mid(self.1)
    }

    /// Проверяет, лежит ли точка внутри или на границе бокса.
    #[inline(always)]
    pub fn contains(self, pt: V) -> bool {
        self + BoundingBox(pt, pt) == self
    }

    #[inline(always)]
    /// Возвращает `true`, если AABB пересекается с другим боксом (включая касание).
    pub fn intersects(&self, other: &Self) -> bool {
        let intersection = *self ^ *other;
        !intersection.is_empty()
    }
}

impl<V> BoundingBox<V> where V: Index<usize> {}

impl<V: Bounded> FromIterator<V> for BoundingBox<V> {
    /// Создаёт бокс, охватывающий все элементы итератора.
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> BoundingBox<V> {
        let mut bbox = BoundingBox::new();
        bbox.extend(iter);
        bbox
    }
}

impl<'a, V: Bounded> FromIterator<&'a V> for BoundingBox<V> {
    /// Строит бокс из итератора ссылок.
    fn from_iter<I: IntoIterator<Item = &'a V>>(iter: I) -> BoundingBox<V> {
        let mut bbox = BoundingBox::new();
        bbox.extend(iter);
        bbox
    }
}

impl<V: Bounded> Extend<V> for BoundingBox<V> {
    /// Дополняет бокс точками из итератора.
    #[inline(always)]
    fn extend<I: IntoIterator<Item = V>>(&mut self, iter: I) {
        iter.into_iter().for_each(|point| self.push(point));
    }
}

impl<'a, V: Bounded> Extend<&'a V> for BoundingBox<V> {
    /// Дополняет бокс ссылками на точки.
    #[inline(always)]
    fn extend<I: IntoIterator<Item = &'a V>>(&mut self, iter: I) {
        iter.into_iter().for_each(|point| self.push(*point));
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

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Point1, Point3};

    #[test]
    fn intersects_detects_overlap() {
        let mut a = BoundingBox::new();
        a.push(Point3::new(0.0, 0.0, 0.0));
        a.push(Point3::new(1.0, 1.0, 1.0));

        let mut b = BoundingBox::new();
        b.push(Point3::new(0.5, 0.5, 0.5));
        b.push(Point3::new(2.0, 2.0, 2.0));

        let mut c = BoundingBox::new();
        c.push(Point3::new(2.1, 0.0, 0.0));
        c.push(Point3::new(3.0, 1.0, 1.0));

        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn extend_accepts_references() {
        let points = [Point1::new(1.0f32), Point1::new(-2.0), Point1::new(4.0)];
        let mut bbox = BoundingBox::new();
        bbox.extend(points.iter());

        assert_eq!(bbox.min()[0], -2.0);
        assert_eq!(bbox.max()[0], 4.0);
    }
}
