use crate::cgmath_extend_traits::*;
use cgmath::*;
use num_traits::NumCast;
use std::fmt::Debug;

pub const MAX_DER_ORDER: usize = 31;

#[derive(Clone, Copy, PartialEq)]
pub struct CurveDers<V> {
    array: [V; MAX_DER_ORDER + 1],
    max_order: usize,
}

impl<V> CurveDers<V> {
    #[inline]
    pub fn new(max_order: usize) -> Self
    where
        V: Zero + Copy,
    {
        Self {
            array: [V::zero(); MAX_DER_ORDER + 1],
            max_order,
        }
    }

    #[inline]
    pub const fn max_order(&self) -> usize {
        self.max_order
    }

    #[inline]
    pub fn push(&mut self, value: V) {
        self.max_order += 1;
        self.array[self.max_order] = value;
    }

    #[inline]
    pub fn der(&self) -> Self
    where
        V: Zero + Copy,
    {
        let mut array = [V::zero(); MAX_DER_ORDER + 1];
        array[..MAX_DER_ORDER].copy_from_slice(&self.array[1..]);
        Self {
            array,
            max_order: self.max_order - 1,
        }
    }

    pub fn rat_ders(&self) -> CurveDers<<V::Point as EuclideanSpace>::Diff>
    where
        V: Homogeneous,
    {
        let from = <V::Scalar as NumCast>::from;
        let mut evals = [<V::Point as EuclideanSpace>::Diff::zero(); MAX_DER_ORDER + 1];
        for i in 0..=self.max_order {
            let mut c = 1;
            let sum = (1..i).fold(evals[0] * self[i].weight(), |sum, j| {
                c = c * (i - j + 1) / j;
                sum + evals[j] * (self[i - j].weight() * from(c).unwrap())
            });
            evals[i] = (self[i].truncate() - sum) / self[0].weight();
        }
        CurveDers {
            array: evals,
            max_order: self.max_order,
        }
    }

    pub fn abs_ders(&self) -> CurveDers<V::Scalar>
    where
        V: InnerSpace,
        V::Scalar: BaseFloat,
    {
        let mut evals = [V::Scalar::zero(); MAX_DER_ORDER + 1];
        evals[0] = self[0].magnitude();
        (1..=self.max_order).for_each(|m| {
            let mut c = 1;
            let sum = (0..m).fold(V::Scalar::zero(), |mut sum, i| {
                let x = self[i + 1].dot(self[m - 1 - i]);
                let y = evals[i + 1] * evals[m - 1 - i];
                let c_float = <V::Scalar as NumCast>::from(c).unwrap();
                sum += (x - y) * c_float;
                c = c * (m - 1 - i) / (i + 1);
                sum
            });
            evals[m] = sum / evals[0];
        });
        CurveDers {
            array: evals,
            max_order: self.max_order,
        }
    }

    pub fn combinatorial_der<W, U, B>(&self, other: &CurveDers<W>, binomial: B, order: usize) -> U
    where
        V: Copy,
        W: Copy,
        U: std::ops::Add + std::ops::Mul<f64, Output = U> + Zero,
        B: Fn(V, W) -> U,
    {
        let mut c = 1;
        (0..=order).fold(U::zero(), |sum, i| {
            let c_mult = c as f64;
            c = c * (order - i) / (i + 1);
            sum + binomial(self[i], other[order - i]) * c_mult
        })
    }

    pub fn combinatorial_ders<W, U, B>(&self, other: &CurveDers<W>, binomial: B) -> CurveDers<U>
    where
        V: Copy,
        W: Copy,
        U: std::ops::Add + std::ops::Mul<f64, Output = U> + Zero + Copy,
        B: Fn(V, W) -> U,
    {
        let max_order = self.max_order.min(other.max_order);
        (0..=max_order)
            .map(|n| self.combinatorial_der(other, &binomial, n))
            .collect()
    }

    pub fn element_wise_ders<W, U, B>(&self, other: &CurveDers<W>, binomial: B) -> CurveDers<U>
    where
        V: Copy,
        W: Copy,
        U: Copy + Zero,
        B: Fn(V, W) -> U,
    {
        self.iter()
            .zip(other.iter())
            .map(|(&v, &w)| binomial(v, w))
            .collect()
    }

    pub fn to_array<const LEN: usize>(&self) -> [V; LEN]
    where
        V: Copy,
    {
        if self.max_order > LEN {
            panic!("length of the returned array is longer than given CurveDers");
        }
        <[V; LEN]>::try_from(&self[..LEN]).unwrap()
    }
}

impl<V> std::ops::Deref for CurveDers<V> {
    type Target = [V];
    #[inline]
    fn deref(&self) -> &[V] {
        &self.array[..=self.max_order]
    }
}

impl<V> std::ops::DerefMut for CurveDers<V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [V] {
        &mut self.array[..=self.max_order]
    }
}

impl<V: Zero + Copy, const N: usize> TryFrom<[V; N]> for CurveDers<V> {
    type Error = &'static str;
    fn try_from(value: [V; N]) -> Result<Self, Self::Error> {
        if N == 0 {
            Err("empty array cannot convert to CurveDers.")
        } else if N > MAX_DER_ORDER + 1 {
            Err("the length of CurveDers must be less than MAX_DER_ORDER + 1.")
        } else {
            let mut array = [V::zero(); MAX_DER_ORDER + 1];
            array[..N].copy_from_slice(&value);
            Ok(Self {
                array,
                max_order: N - 1,
            })
        }
    }
}

impl<V: Zero + Copy> TryFrom<&[V]> for CurveDers<V> {
    type Error = &'static str;
    fn try_from(value: &[V]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err("empty slice cannot convert CurveDers.")
        } else if value.len() > MAX_DER_ORDER + 1 {
            Err("the length of CurveDers must be less than MAX_DER_ORDER + 1.")
        } else {
            let mut array = [V::zero(); MAX_DER_ORDER + 1];
            array[..value.len()].copy_from_slice(value);
            Ok(Self {
                array,
                max_order: value.len() - 1,
            })
        }
    }
}

impl<V: Zero + Copy> FromIterator<V> for CurveDers<V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        let mut array = [V::zero(); MAX_DER_ORDER + 1];
        let mut iter = iter.into_iter();
        let len = array.iter_mut().zip(&mut iter).map(|(o, v)| *o = v).count();
        if iter.next().is_some() {
            panic!("too long iterator");
        }
        Self {
            array,
            max_order: len - 1,
        }
    }
}

impl<V: VectorSpace> std::ops::Mul<V::Scalar> for CurveDers<V> {
    type Output = Self;
    fn mul(self, rhs: V::Scalar) -> Self::Output {
        let mut array = self.array;
        array.iter_mut().for_each(|a| *a = *a * rhs);
        Self {
            array,
            max_order: self.max_order,
        }
    }
}

impl<V: VectorSpace> std::ops::Div<V::Scalar> for CurveDers<V> {
    type Output = Self;
    fn div(self, rhs: V::Scalar) -> Self::Output {
        let mut array = self.array;
        array.iter_mut().for_each(|a| *a = *a / rhs);
        Self {
            array,
            max_order: self.max_order,
        }
    }
}

impl<V> AbsDiffEq for CurveDers<V>
where
    V: AbsDiffEq,
    V::Epsilon: Copy,
{
    type Epsilon = V::Epsilon;
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.max_order == other.max_order
            && self
                .iter()
                .zip(other.iter())
                .all(|(v, w)| v.abs_diff_eq(w, epsilon))
    }
    fn default_epsilon() -> Self::Epsilon {
        V::default_epsilon()
    }
}

impl<V: Debug> Debug for CurveDers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.array[..=self.max_order].fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SurfaceDers<V> {
    array: [[V; MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1],
    max_order: usize,
}

impl<V> SurfaceDers<V> {
    #[inline]
    pub fn new(max_order: usize) -> Self
    where
        V: Zero + Copy,
    {
        Self {
            array: [[V::zero(); MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1],
            max_order,
        }
    }
    #[inline]
    pub const fn max_order(&self) -> usize {
        self.max_order
    }

    #[inline]
    pub fn slice_iter(&self) -> impl Iterator<Item = &[V]> {
        self.array[..=self.max_order]
            .iter()
            .enumerate()
            .map(|(i, v)| &v[..=self.max_order - i])
    }

    #[inline]
    pub fn slice_iter_mut(&mut self) -> impl Iterator<Item = &mut [V]> {
        self.array[..=self.max_order]
            .iter_mut()
            .enumerate()
            .map(|(i, v)| &mut v[..=self.max_order - i])
    }

    #[inline]
    pub fn uder(&self) -> Self
    where
        V: Zero + Copy,
    {
        let mut array = [[V::zero(); MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1];
        array[..MAX_DER_ORDER].copy_from_slice(&self.array[1..]);
        Self {
            array,
            max_order: self.max_order - 1,
        }
    }

    #[inline]
    pub fn vder(&self) -> Self
    where
        V: Zero + Copy,
    {
        let mut array = [[V::zero(); MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1];
        array.iter_mut().zip(&self.array).for_each(|(arr, sarr)| {
            arr[..MAX_DER_ORDER].copy_from_slice(&sarr[1..]);
        });
        Self {
            array,
            max_order: self.max_order - 1,
        }
    }

    pub fn rat_ders(&self) -> SurfaceDers<<V::Point as EuclideanSpace>::Diff>
    where
        V: Homogeneous,
    {
        let zero = <V::Point as EuclideanSpace>::Diff::zero();
        let from = <V::Scalar as NumCast>::from;
        let mut evals = [[zero; MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1];
        for m in 0..=self.max_order {
            for n in 0..=self.max_order - m {
                let mut sum = zero;
                let mut c0 = 1;
                for i in 0..=m {
                    let mut c1 = 1;
                    let (evals, ders) = (evals[i].as_mut(), &self[m - i]);
                    for j in 0..=n {
                        let (c0_s, c1_s) = (from(c0).unwrap(), from(c1).unwrap());
                        sum = sum + evals[j] * (ders[n - j].weight() * c0_s * c1_s);
                        c1 = c1 * (n - j) / (j + 1);
                    }
                    c0 = c0 * (m - i) / (i + 1);
                }
                let (eval_mn, der_mn) = (&mut evals[m].as_mut()[n], self[m][n]);
                *eval_mn = (der_mn.truncate() - sum) / self[0][0].weight();
            }
        }
        SurfaceDers {
            array: evals,
            max_order: self.max_order,
        }
    }

    pub fn composite_der(&self, curve_ders: &CurveDers<Vector2<V::Scalar>>, order: usize) -> V
    where
        V: VectorSpace,
        V::Scalar: BaseFloat,
    {
        if order > self.max_order || order > curve_ders.max_order {
            panic!("calculating derivative with order={order}, but the orders of given derivatives are less than {order}.");
        }
        (1..=order).fold(V::zero(), |sum, len| {
            let iter = CompositionIter::<32>::try_new(order, len).unwrap();
            iter.fold(sum, |sum, idx| {
                let idx = &idx[..len];
                let mult = <V::Scalar as NumCast>::from(multiplicity(idx)).unwrap();
                sum + tensor(self, curve_ders, idx) * mult
            })
        })
    }

    pub fn composite_ders(&self, curve_ders: &CurveDers<Vector2<V::Scalar>>) -> CurveDers<V>
    where
        V: VectorSpace,
        V::Scalar: BaseFloat,
    {
        let max_order = self.max_order.min(curve_ders.max_order);
        let mut res = CurveDers::new(max_order);
        res[0] = self[0][0];
        let iter = res[1..].iter_mut().enumerate();
        iter.for_each(|(i, o)| *o = self.composite_der(curve_ders, i + 1));
        res
    }

    pub fn element_wise_ders<W, B, U>(&self, other: &SurfaceDers<W>, binomial: B) -> SurfaceDers<U>
    where
        V: Copy,
        W: Copy,
        B: Fn(V, W) -> U,
        U: Copy + Zero,
    {
        let max_order = self.max_order.min(other.max_order);
        let mut res = SurfaceDers::new(max_order);
        res.slice_iter_mut()
            .zip(self.slice_iter())
            .zip(other.slice_iter())
            .for_each(|((o, a), b)| {
                o.iter_mut()
                    .zip(a)
                    .zip(b)
                    .for_each(|((o, &a), &b)| *o = binomial(a, b))
            });
        res
    }
}

impl<V> std::ops::Index<usize> for SurfaceDers<V> {
    type Output = [V];
    fn index(&self, index: usize) -> &[V] {
        if index > self.max_order {
            panic!("the index must be no more than {}.", self.max_order);
        }
        &self.array[index][..=self.max_order - index]
    }
}

impl<V> std::ops::IndexMut<usize> for SurfaceDers<V> {
    fn index_mut(&mut self, index: usize) -> &mut [V] {
        if index > self.max_order {
            panic!("the index must be no more than {}.", self.max_order);
        }
        &mut self.array[index][..=self.max_order - index]
    }
}

impl<V: Zero + Copy> TryFrom<&[&[V]]> for SurfaceDers<V> {
    type Error = &'static str;
    fn try_from(value: &[&[V]]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("Empty array cannot convert to `SurfaceDers`.");
        }
        let mut array = [[V::zero(); MAX_DER_ORDER + 1]; MAX_DER_ORDER + 1];
        let max_order = value.len() - 1;

        let mut iter = value.iter().zip(&mut array).enumerate();
        iter.try_for_each(|(i, (&slice, subarray))| {
            if i + slice.len() != max_order + 1 {
                Err("Inconsistent slice length and order.")
            } else {
                subarray[..=max_order - i].copy_from_slice(slice);
                Ok(())
            }
        })?;

        Ok(Self { array, max_order })
    }
}

impl<V: VectorSpace> std::ops::Mul<V::Scalar> for SurfaceDers<V> {
    type Output = Self;
    fn mul(self, rhs: V::Scalar) -> Self::Output {
        let mut array = self.array;
        array.iter_mut().flatten().for_each(|a| *a = *a * rhs);
        Self {
            array,
            max_order: self.max_order,
        }
    }
}

impl<V: VectorSpace> std::ops::Div<V::Scalar> for SurfaceDers<V> {
    type Output = Self;
    fn div(self, rhs: V::Scalar) -> Self::Output {
        let mut array = self.array;
        array.iter_mut().flatten().for_each(|a| *a = *a / rhs);
        Self {
            array,
            max_order: self.max_order,
        }
    }
}

impl<V> AbsDiffEq for SurfaceDers<V>
where
    V: AbsDiffEq,
    V::Epsilon: Copy,
{
    type Epsilon = V::Epsilon;
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.max_order == other.max_order
            && self
                .slice_iter()
                .zip(other.slice_iter())
                .all(|(slice0, slice1)| {
                    slice0
                        .iter()
                        .zip(slice1)
                        .all(|(v, w)| v.abs_diff_eq(w, epsilon))
                })
    }
    fn default_epsilon() -> Self::Epsilon {
        V::default_epsilon()
    }
}

impl<V: Debug> Debug for SurfaceDers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("[")?;
        for (i, a) in self.slice_iter().enumerate() {
            a[..=self.max_order - i].fmt(f)?;
            if self.max_order != i {
                f.pad(", ")?;
            }
        }
        f.pad("]")
    }
}

#[test]
fn surface_ders_debug() {
    let ders = SurfaceDers::<f64>::new(2);
    let string = format!("{ders:?}");
    assert_eq!(string, "[[0.0, 0.0, 0.0], [0.0, 0.0], [0.0]]");
}

fn can_init(len: usize, n: usize, max: usize) -> bool {
    !(len > n || max * len < n)
}

fn init(array: &mut [usize], n: usize, max: usize) {
    if array.is_empty() {
        return;
    }
    array[0] = (n - array.len() + 1).min(max);
    let (n, max) = (n - array[0], array[0]);
    init(&mut array[1..], n, max)
}

fn next(array: &mut [usize]) -> bool {
    let n = array[1..].iter().sum::<usize>() + 1;
    let max = array[0] - 1;
    if array.len() == 1 {
        false
    } else if next(&mut array[1..]) {
        true
    } else if can_init(array.len() - 1, n, max) {
        array[0] -= 1;
        init(&mut array[1..], n, max);
        true
    } else {
        false
    }
}

#[derive(Clone, Debug)]
struct CompositionIter<const MAX: usize> {
    current: [usize; MAX],
    end: bool,
    len: usize,
}

impl<const MAX: usize> CompositionIter<MAX> {
    fn try_new(n: usize, len: usize) -> Option<Self> {
        if !(len < MAX && can_init(len, n, n)) {
            return None;
        }
        let mut current = [0; MAX];
        init(&mut current[..len], n, n);
        Some(Self {
            current,
            len,
            end: false,
        })
    }
}

impl<const MAX: usize> Iterator for CompositionIter<MAX> {
    type Item = [usize; MAX];
    fn next(&mut self) -> Option<Self::Item> {
        if self.end {
            return None;
        }
        let current = self.current;
        self.end = !next(&mut self.current[..self.len]);
        Some(current)
    }
}

fn factorial(n: usize) -> u128 {
    (2..=n).fold(1, |f, i| f * i as u128)
}

fn multiplicity(array: &[usize]) -> u128 {
    let n = array.iter().sum::<usize>();
    let mut res = factorial(n);
    array.iter().for_each(|&a| res /= factorial(a));
    let mut mult = 1;
    array.windows(2).for_each(|x| {
        if x[0] == x[1] {
            mult += 1;
        } else {
            res /= factorial(mult);
            mult = 1;
        }
    });
    res / factorial(mult)
}

fn tensor<S, V, A>(sder: &A, cder: &[Vector2<S>], idx: &[usize]) -> V
where
    S: BaseFloat,
    V: VectorSpace<Scalar = S>,
    A: std::ops::Index<usize, Output = [V]>,
{
    let n: u128 = 2u128.pow(idx.len() as u32);
    (0..n).fold(V::zero(), |sum, mut i| {
        let (t, mult) = idx.iter().fold((0, S::one()), |(t, mult), &j| {
            let k = (i % 2) as usize;
            i /= 2;
            (t + k, mult * cder[j][k])
        });
        sum + sder[idx.len() - t][t] * mult
    })
}

#[test]
fn test_composition_iter() {
    let iter = CompositionIter::<8>::try_new(10, 4).unwrap();
    let vec: Vec<_> = iter.collect();
    let iter = vec.iter().map(|idx| {
        idx[..4].iter().for_each(|&i| assert_ne!(i, 0));
        idx[4..].iter().for_each(|&i| assert_eq!(i, 0));
        &idx[..4]
    });
    let vec: Vec<_> = iter.collect();

    assert_eq!(vec.len(), 9);
    assert_eq!(vec[0], &[7, 1, 1, 1]);
    assert_eq!(vec[1], &[6, 2, 1, 1]);
    assert_eq!(vec[2], &[5, 3, 1, 1]);
    assert_eq!(vec[3], &[5, 2, 2, 1]);
    assert_eq!(vec[4], &[4, 4, 1, 1]);
    assert_eq!(vec[5], &[4, 3, 2, 1]);
    assert_eq!(vec[6], &[4, 2, 2, 2]);
    assert_eq!(vec[7], &[3, 3, 3, 1]);
    assert_eq!(vec[8], &[3, 3, 2, 2]);
}
