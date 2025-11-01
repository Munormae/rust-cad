use super::*;
use crate::errors::Error;
use cryxtal_base::tolerance::{Origin, Tolerance};
use std::slice::SliceIndex;
use std::vec::Vec;

impl KnotVec {
    pub const fn new() -> KnotVec {
        KnotVec(Vec::new())
    }

    #[inline(always)]
    pub fn range_length(&self) -> f64 {
        match self.is_empty() {
            true => 0.0,
            false => self[self.len() - 1] - self[0],
        }
    }

    #[inline(always)]
    pub fn same_range(&self, other: &KnotVec) -> bool {
        match (self.is_empty(), other.is_empty()) {
            (false, false) => {
                self[0].near(&other[0]) && self.range_length().near(&other.range_length())
            }
            (true, true) => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, idx: usize) -> f64 {
        self.0.remove(idx)
    }

    #[inline(always)]
    pub fn floor(&self, x: f64) -> Option<usize> {
        self.iter().rposition(|t| *t <= x)
    }

    #[inline(always)]
    pub fn multiplicity(&self, i: usize) -> usize {
        self.iter().filter(|u| self[i].near(u)).count()
    }

    #[inline(always)]
    pub fn add_knot(&mut self, knot: f64) -> usize {
        match self.floor(knot) {
            Some(idx) => {
                self.0.insert(idx + 1, knot);
                idx + 1
            }
            None => {
                self.0.insert(0, knot);
                0
            }
        }
    }

    pub fn bspline_basis_functions(&self, degree: usize, der_rank: usize, t: f64) -> Vec<f64> {
        match self.try_bspline_basis_functions(degree, der_rank, t) {
            Ok(got) => got,
            Err(error) => panic!("{}", error),
        }
    }

    pub fn try_bspline_basis_functions(
        &self,
        degree: usize,
        der_rank: usize,
        t: f64,
    ) -> Result<Vec<f64>> {
        let n = self.len() - 1;
        if self[0].near(&self[n]) {
            return Err(Error::ZeroRange);
        } else if n < degree {
            return Err(Error::TooLargeDegree(n + 1, degree));
        }
        if degree < der_rank {
            return Ok(vec![0.0; n - degree]);
        }

        let idx = {
            let idx = self
                .floor(t)
                .unwrap_or_else(|| self.floor(self[0]).unwrap());
            if idx == n {
                n - self.multiplicity(n)
            } else {
                idx
            }
        };

        if n < 32 {
            let mut eval = [0.0; 32];
            self.sub_bspline_basis_functions(degree, der_rank, t, idx, &mut eval);
            Ok(eval[..n - degree].to_vec())
        } else {
            let mut eval = vec![0.0; n];
            self.sub_bspline_basis_functions(degree, der_rank, t, idx, &mut eval);
            eval.truncate(n - degree);
            Ok(eval)
        }
    }

    fn sub_bspline_basis_functions(
        &self,
        degree: usize,
        der_rank: usize,
        t: f64,
        idx: usize,
        eval: &mut [f64],
    ) {
        let n = self.len() - 1;
        eval[idx] = 1.0;

        for k in 1..=(degree - der_rank) {
            let base = idx.saturating_sub(k);
            let delta = self[base + k] - self[base];
            let mut a = inv_or_zero(delta) * (t - self[base]);
            for i in base..=usize::min(idx, n - k - 1) {
                let delta = self[i + k + 1] - self[i + 1];
                let b = inv_or_zero(delta) * (self[i + k + 1] - t);
                eval[i] = a * eval[i] + b * eval[i + 1];
                a = 1.0 - b;
            }
        }

        for k in (degree - der_rank + 1)..=degree {
            let base = idx.saturating_sub(k);
            let delta = self[base + k] - self[base];
            let mut a = inv_or_zero(delta);
            for i in base..=usize::min(idx, n - k - 1) {
                let delta = self[i + k + 1] - self[i + 1];
                let b = inv_or_zero(delta);
                eval[i] = (a * eval[i] - b * eval[i + 1]) * k as f64;
                a = b;
            }
        }
    }

    #[doc(hidden)]
    pub fn maximum_points(&self, degree: usize) -> Vec<f64> {
        let n = self.len();
        let m = n - degree - 1;
        let range = self.range_length();
        const N: i32 = 100;

        let mut res = vec![0.0; m];
        let mut max = vec![0.0; m];
        for i in 1..N {
            let t = self[0] + range * (i as f64) / (N as f64);
            let vals = self.try_bspline_basis_functions(degree, 0, t).unwrap();
            for j in 0..m {
                if max[j] < vals[j] {
                    max[j] = vals[j];
                    res[j] = t;
                }
            }
        }

        res
    }

    pub fn transform(&mut self, scalar: f64, r#move: f64) -> &mut Self {
        assert!(scalar > 0.0, "The scalar {scalar} is not positive.");
        self.0
            .iter_mut()
            .for_each(move |vec| *vec = *vec * scalar + r#move);
        self
    }

    pub fn try_normalize(&mut self) -> Result<&mut Self> {
        let range = self.range_length();
        if range.so_small() {
            return Err(Error::ZeroRange);
        }
        Ok(self.transform(1.0 / range, -self[0] / range))
    }

    #[inline(always)]
    pub fn normalize(&mut self) -> &mut Self {
        self.try_normalize()
            .unwrap_or_else(|error| panic!("{}", error))
    }

    pub fn translate(&mut self, x: f64) -> &mut Self {
        self.transform(1.0, x)
    }

    pub fn invert(&mut self) -> &mut Self {
        let n = self.len();
        if n == 0 {
            return self;
        }
        let range = self[0] + self[n - 1];
        let clone = self.0.clone();
        for (knot1, knot0) in clone.iter().rev().zip(&mut self.0) {
            *knot0 = range - knot1;
        }
        self
    }

    #[inline(always)]
    pub fn is_clamped(&self, degree: usize) -> bool {
        self.multiplicity(0) > degree && self.multiplicity(self.len() - 1) > degree
    }

    pub fn try_concat(&mut self, other: &KnotVec, degree: usize) -> Result<&mut Self> {
        if !self.is_clamped(degree) || !other.is_clamped(degree) {
            return Err(Error::NotClampedKnotVector);
        }
        let back = self.0.last().unwrap();
        let front = other.0.first().unwrap();
        if front < back || !front.near(back) {
            return Err(Error::DifferentBackFront(*back, *front));
        }

        self.0.truncate(self.len() - degree - 1);
        self.0.extend(other.0.iter().copied());

        Ok(self)
    }

    #[inline(always)]
    pub fn concat(&mut self, other: &KnotVec, degree: usize) -> &mut Self {
        self.try_concat(other, degree)
            .unwrap_or_else(|error| panic!("{}", error))
    }

    #[inline(always)]
    pub fn sub_vec<I: SliceIndex<[f64], Output = [f64]>>(&self, range: I) -> KnotVec {
        KnotVec(Vec::from(&self.0[range]))
    }

    pub fn to_single_multi(&self) -> (Vec<f64>, Vec<usize>) {
        let mut knots = Vec::new();
        let mut mults = Vec::new();

        let mut iter = self.as_slice().iter().peekable();
        let mut mult = 1;
        while let Some(knot) = iter.next() {
            if let Some(next) = iter.peek() {
                if knot.near(next) {
                    mult += 1;
                } else {
                    knots.push(*knot);
                    mults.push(mult);
                    mult = 1;
                }
            } else {
                knots.push(*knot);
                mults.push(mult);
            }
        }
        (knots, mults)
    }

    pub fn from_single_multi(knots: Vec<f64>, mults: Vec<usize>) -> Result<KnotVec> {
        for i in 1..knots.len() {
            if knots[i - 1] > knots[i] {
                return Err(Error::NotSortedVector);
            }
        }

        let mut vec = Vec::new();
        for i in 0..knots.len() {
            for _ in 0..mults[i] {
                vec.push(knots[i]);
            }
        }
        Ok(KnotVec(vec))
    }

    pub fn try_from(vec: Vec<f64>) -> Result<KnotVec> {
        for i in 1..vec.len() {
            if vec[i - 1] > vec[i] {
                return Err(Error::NotSortedVector);
            }
        }
        Ok(KnotVec(vec))
    }

    pub fn bezier_knot(degree: usize) -> KnotVec {
        let mut vec = vec![0.0; degree + 1];
        vec.extend(std::iter::repeat_n(1.0, degree + 1));
        KnotVec(vec)
    }

    pub fn uniform_knot(degree: usize, division: usize) -> KnotVec {
        let mut vec = vec![0.0; degree + 1];
        vec.extend((1..division).map(|i| i as f64 / division as f64));
        vec.extend(std::iter::repeat_n(1.0, degree + 1));
        KnotVec(vec)
    }
}

impl From<Vec<f64>> for KnotVec {
    fn from(mut vec: Vec<f64>) -> KnotVec {
        vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
        KnotVec(vec)
    }
}

impl From<&[f64]> for KnotVec {
    #[inline(always)]
    fn from(vec: &[f64]) -> KnotVec {
        let mut copy_vec = vec.to_vec();
        copy_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
        KnotVec(copy_vec)
    }
}

impl From<&Vec<f64>> for KnotVec {
    #[inline(always)]
    fn from(vec: &Vec<f64>) -> KnotVec {
        let mut copy_vec = vec.clone();
        copy_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
        KnotVec(copy_vec)
    }
}

impl From<KnotVec> for Vec<f64> {
    #[inline(always)]
    fn from(knotvec: KnotVec) -> Vec<f64> {
        knotvec.0
    }
}

impl FromIterator<f64> for KnotVec {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> KnotVec {
        KnotVec::try_from(iter.into_iter().collect::<Vec<_>>()).unwrap()
    }
}

impl<'a> IntoIterator for &'a KnotVec {
    type Item = &'a f64;
    type IntoIter = std::slice::Iter<'a, f64>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl std::ops::Deref for KnotVec {
    type Target = Vec<f64>;
    #[inline(always)]
    fn deref(&self) -> &Vec<f64> {
        &self.0
    }
}

impl AsRef<[f64]> for KnotVec {
    #[inline(always)]
    fn as_ref(&self) -> &[f64] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for KnotVec {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        Self::try_from(vec).map_err(serde::de::Error::custom)
    }
}
