//! Parallel utilities that scale core geometric primitives.
use crate::bounding_box::{Bounded, BoundingBox};
use rayon::prelude::{
    FromParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelExtend,
    ParallelIterator,
};

/// Строит [`BoundingBox`] из параллельного итератора точек.
#[inline]
pub fn bounding_box_from_par_iter<V, I>(iter: I) -> BoundingBox<V>
where
    V: Bounded + Send,
    I: IntoParallelIterator<Item = V>,
{
    iter.into_par_iter()
        .fold(BoundingBox::new, |mut acc, point| {
            acc.push(point);
            acc
        })
        .reduce(BoundingBox::new, |mut lhs, rhs| {
            lhs += rhs;
            lhs
        })
}

/// Строит [`BoundingBox`] из параллельного итератора ссылок на точки.
#[inline]
pub fn bounding_box_from_par_iter_ref<'a, V, I>(iter: I) -> BoundingBox<V>
where
    V: Bounded + Send + Copy + 'a,
    I: IntoParallelIterator<Item = &'a V>,
{
    iter.into_par_iter()
        .fold(BoundingBox::new, |mut acc, &point| {
            acc.push(point);
            acc
        })
        .reduce(BoundingBox::new, |mut lhs, rhs| {
            lhs += rhs;
            lhs
        })
}

/// Вычисляет AABB для среза точек, распараллеливая по `par_iter`.
#[inline]
pub fn bounding_box_from_slice<V>(points: &[V]) -> BoundingBox<V>
where
    V: Bounded + Send + Sync + Copy,
{
    bounding_box_from_par_iter_ref(points.par_iter())
}

impl<V> FromParallelIterator<V> for BoundingBox<V>
where
    V: Bounded + Send,
{
    #[inline]
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = V>,
    {
        bounding_box_from_par_iter(par_iter)
    }
}

impl<'a, V> FromParallelIterator<&'a V> for BoundingBox<V>
where
    V: Bounded + Send + Sync + Copy + 'a,
{
    #[inline]
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = &'a V>,
    {
        bounding_box_from_par_iter_ref(par_iter)
    }
}

impl<V> ParallelExtend<V> for BoundingBox<V>
where
    V: Bounded + Send,
{
    #[inline]
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = V>,
    {
        let other = bounding_box_from_par_iter(par_iter);
        *self += other;
    }
}

impl<'a, V> ParallelExtend<&'a V> for BoundingBox<V>
where
    V: Bounded + Send + Sync + Copy + 'a,
{
    #[inline]
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = &'a V>,
    {
        let other = bounding_box_from_par_iter_ref(par_iter);
        *self += other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Point3;

    #[test]
    fn slice_par_bbox_matches_seq() {
        let points = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, -2.0, 3.0),
            Point3::new(-4.0, 5.0, -6.0),
        ];

        let mut seq = BoundingBox::new();
        seq.extend(points.iter());
        let par = bounding_box_from_slice(&points);

        assert_eq!(seq.min(), par.min());
        assert_eq!(seq.max(), par.max());
    }
}
