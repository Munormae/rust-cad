use super::*;
use crate::errors::Error;
use std::ops::*;

impl<P> BSplineCurve<P> {
    pub fn new(knot_vec: KnotVec, control_points: Vec<P>) -> BSplineCurve<P> {
        BSplineCurve::try_new(knot_vec, control_points).unwrap_or_else(|e| panic!("{}", e))
    }

    pub fn try_new(knot_vec: KnotVec, control_points: Vec<P>) -> Result<BSplineCurve<P>> {
        if control_points.is_empty() {
            Err(Error::EmptyControlPoints)
        } else if knot_vec.len() <= control_points.len() {
            Err(Error::TooShortKnotVector(
                knot_vec.len(),
                control_points.len(),
            ))
        } else if knot_vec.range_length().so_small() {
            Err(Error::ZeroRange)
        } else {
            Ok(BSplineCurve::new_unchecked(knot_vec, control_points))
        }
    }

    #[inline(always)]
    pub const fn new_unchecked(knot_vec: KnotVec, control_points: Vec<P>) -> BSplineCurve<P> {
        Self {
            knot_vec,
            control_points,
        }
    }

    #[inline(always)]
    pub fn debug_new(knot_vec: KnotVec, control_points: Vec<P>) -> BSplineCurve<P> {
        match cfg!(debug_assertions) {
            true => Self::new(knot_vec, control_points),
            false => Self::new_unchecked(knot_vec, control_points),
        }
    }

    #[inline(always)]
    pub const fn knot_vec(&self) -> &KnotVec {
        &self.knot_vec
    }

    #[inline(always)]
    pub fn knot(&self, idx: usize) -> f64 {
        self.knot_vec[idx]
    }

    #[inline(always)]
    pub const fn control_points(&self) -> &Vec<P> {
        &self.control_points
    }

    #[inline(always)]
    pub fn control_point(&self, idx: usize) -> &P {
        &self.control_points[idx]
    }

    #[inline(always)]
    pub fn control_point_mut(&mut self, idx: usize) -> &mut P {
        &mut self.control_points[idx]
    }

    #[inline(always)]
    pub fn control_points_mut(&mut self) -> impl Iterator<Item = &mut P> {
        self.control_points.iter_mut()
    }

    #[inline(always)]
    pub fn destruct(self) -> (KnotVec, Vec<P>) {
        (self.knot_vec, self.control_points)
    }

    #[inline(always)]
    pub fn transform_control_points<F: FnMut(&mut P)>(&mut self, f: F) {
        self.control_points.iter_mut().for_each(f)
    }

    #[inline(always)]
    pub fn degree(&self) -> usize {
        self.knot_vec.len() - self.control_points.len() - 1
    }

    #[inline(always)]
    pub fn is_clamped(&self) -> bool {
        self.knot_vec.is_clamped(self.degree())
    }

    #[inline(always)]
    pub fn knot_normalize(&mut self) -> &mut Self {
        self.knot_vec.try_normalize().unwrap();
        self
    }

    #[inline(always)]
    pub fn knot_translate(&mut self, x: f64) -> &mut Self {
        self.knot_vec.translate(x);
        self
    }
}

impl<P: ControlPoint<f64>> BSplineCurve<P> {
    #[inline(always)]
    pub fn get_closure(&self) -> impl Fn(f64) -> P + '_ {
        move |t| self.subs(t)
    }
    #[inline(always)]
    fn delta_control_points(&self, i: usize) -> P::Diff {
        if i == 0 {
            self.control_point(i).to_vec()
        } else if i == self.control_points.len() {
            self.control_points[i - 1].to_vec() * (-1.0)
        } else {
            self.control_points[i] - self.control_points[i - 1]
        }
    }

    pub fn derivation(&self) -> BSplineCurve<P::Diff> {
        let n = self.control_points.len();
        let k = self.degree();
        let knot_vec = self.knot_vec.clone();
        let mut new_points = Vec::with_capacity(n + 1);
        if k > 0 {
            let (knot_vec, new_points) = (&knot_vec, &mut new_points);
            (0..=n).for_each(move |i| {
                let delta = knot_vec[i + k] - knot_vec[i];
                let coef = (k as f64) * inv_or_zero(delta);
                new_points.push(self.delta_control_points(i) * coef);
            });
        } else {
            new_points = vec![P::Diff::zero(); n];
        }
        BSplineCurve::new_unchecked(knot_vec, new_points)
    }
    pub(super) fn sub_near_as_curve<F: Fn(&P, &P) -> bool>(
        &self,
        other: &BSplineCurve<P>,
        div_coef: usize,
        ord: F,
    ) -> bool {
        if !self.knot_vec.same_range(&other.knot_vec) {
            return false;
        }

        let division = std::cmp::max(self.degree(), other.degree()) * div_coef;
        for i in 0..(self.knot_vec.len() - 1) {
            let delta = self.knot_vec[i + 1] - self.knot_vec[i];
            if delta.so_small() {
                continue;
            }

            for j in 0..division {
                let t = self.knot_vec[i] + delta * (j as f64) / (division as f64);
                if !ord(&self.subs(t), &other.subs(t)) {
                    return false;
                }
            }
        }
        true
    }

    pub fn try_interpole(
        knot_vec: KnotVec,
        mut parameter_points: impl AsMut<[(f64, P)]>,
    ) -> Result<Self> {
        let parameter_points = parameter_points.as_mut();
        if knot_vec.len() <= parameter_points.len() {
            return Err(Error::TooShortKnotVector(
                knot_vec.len(),
                parameter_points.len(),
            ));
        }

        let degree = knot_vec.len() - parameter_points.len() - 1;

        let rows = parameter_points
            .iter()
            .map(|(t, _)| knot_vec.try_bspline_basis_functions(degree, 0, *t))
            .collect::<Result<Vec<_>>>()?;

        for i in 0..P::DIM {
            let mut rows = rows.clone();
            rows.iter_mut()
                .zip(parameter_points.iter())
                .for_each(|(row, (_, p))| row.push(p[i]));
            gaussian_elimination::gaussian_elimination(&mut rows)
                .ok_or(Error::GaussianEliminationFailure)?
                .into_iter()
                .zip(parameter_points.iter_mut())
                .for_each(|(res, (_, p))| p[i] = res);
        }

        let control_points = parameter_points.iter().map(|(_, p)| *p).collect::<Vec<_>>();
        Self::try_new(knot_vec, control_points)
    }

    pub fn interpole(knot_vec: KnotVec, parameter_points: impl AsMut<[(f64, P)]>) -> Self {
        Self::try_interpole(knot_vec, parameter_points).unwrap()
    }
}

impl<P> BSplineCurve<P>
where
    P: ControlPoint<f64> + MetricSpace<Metric = f64> + HashGen<f64>,
{
    pub fn quadratic_approximation<C>(
        curve: &C,
        range: (f64, f64),
        tol: f64,
        trials: usize,
    ) -> Option<Self>
    where
        C: ParametricCurve<Point = P, Vector = P::Diff>,
    {
        for n in 0..trials {
            let mut knot_vec = KnotVec::uniform_knot(2, n + 1);
            knot_vec.transform(range.1 - range.0, range.0);
            let len = n + 2;
            let parameter_points = (0..=len)
                .map(|i| {
                    let rat = i as f64 / len as f64;
                    let t = range.0 + (range.1 - range.0) * rat;
                    (t, curve.subs(t))
                })
                .collect::<Vec<_>>();
            let bsp = Self::try_interpole(knot_vec, parameter_points).ok()?;
            let is_approx = (0..len).all(|i| {
                let gen = *bsp.control_point(i);
                let rat = 0.5 + (0.2 * HashGen::hash1(gen) - 0.1);
                let t = range.0 + (range.1 - range.0) * rat;
                curve.subs(t).distance2(bsp.subs(t)) < tol * tol
            });
            if is_approx {
                return Some(bsp);
            }
        }
        None
    }
}

impl<V: Homogeneous> BSplineCurve<V> {
    pub fn lift_up(curve: BSplineCurve<V::Point>) -> Self {
        let control_points = curve
            .control_points
            .into_iter()
            .map(V::from_point)
            .collect();
        BSplineCurve::new_unchecked(curve.knot_vec, control_points)
    }
}

impl<P: ControlPoint<f64>> ParametricCurve for BSplineCurve<P> {
    type Point = P;
    type Vector = P::Diff;
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> P::Diff {
        self.control_points
            .iter()
            .zip(self.knot_vec.bspline_basis_functions(self.degree(), n, t))
            .fold(P::Diff::zero(), |sum, (p, b)| sum + p.to_vec() * b)
    }
    #[inline(always)]
    fn subs(&self, t: f64) -> P {
        P::from_vec(self.der_n(0, t))
    }
    #[inline(always)]
    fn der(&self, t: f64) -> P::Diff {
        self.der_n(1, t)
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> P::Diff {
        self.der_n(2, t)
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (
            Bound::Included(self.knot_vec[0]),
            Bound::Included(self.knot_vec[self.knot_vec.len() - 1]),
        )
    }
}

impl<P: ControlPoint<f64>> BoundedCurve for BSplineCurve<P> {}

impl<P: ControlPoint<f64> + Tolerance> BSplineCurve<P> {
    pub fn is_const(&self) -> bool {
        self.control_points
            .iter()
            .all(move |vec| vec.near(&self.control_points[0]))
    }

    pub fn add_knot(&mut self, x: f64) -> &mut Self {
        if x < self.knot_vec[0] {
            self.knot_vec.add_knot(x);
            self.control_points.insert(0, P::origin());
            return self;
        }

        let k = self.degree();
        let n = self.control_points.len();

        let idx = self.knot_vec.add_knot(x);
        let start = idx.saturating_sub(k);
        let end = if idx > n {
            self.control_points.push(P::origin());
            n + 1
        } else {
            self.control_points
                .insert(idx - 1, *self.control_point(idx - 1));
            idx
        };
        for i in start..end {
            let i0 = end + start - i - 1;
            let delta = self.knot_vec[i0 + k + 1] - self.knot_vec[i0];
            let a = (self.knot_vec[idx] - self.knot_vec[i0]) * inv_or_zero(delta);
            let p = self.delta_control_points(i0) * (1.0 - a);
            self.control_points[i0] -= p;
        }
        self
    }

    pub fn remove_knot(&mut self, idx: usize) -> &mut Self {
        let _ = self.try_remove_knot(idx);
        self
    }

    pub fn try_remove_knot(&mut self, idx: usize) -> Result<&mut BSplineCurve<P>> {
        let k = self.degree();
        let n = self.control_points.len();
        let knot_vec = &self.knot_vec;

        if idx < k + 1 || idx >= n {
            return Err(Error::CannotRemoveKnot(idx));
        }

        let mut new_points = Vec::with_capacity(k + 1);
        new_points.push(*self.control_point(idx - k - 1));
        for i in (idx - k)..idx {
            let delta = knot_vec[i + k + 1] - knot_vec[i];
            let a = inv_or_zero(delta) * (knot_vec[idx] - knot_vec[i]);
            if a.so_small() {
                break;
            } else {
                let p = *new_points.last().unwrap();
                let p = p + (self.control_points[i] - p) / a;
                new_points.push(p);
            }
        }

        if !new_points.last().unwrap().near(self.control_point(idx)) {
            return Err(Error::CannotRemoveKnot(idx));
        }

        for (i, vec) in new_points.into_iter().skip(1).enumerate() {
            self.control_points[idx - k + i] = vec;
        }

        self.control_points.remove(idx);
        self.knot_vec.remove(idx);
        Ok(self)
    }

    fn elevate_degree_bezier(&mut self) -> &mut Self {
        let k = self.degree();
        self.knot_vec.add_knot(self.knot_vec[0]);
        self.knot_vec
            .add_knot(self.knot_vec[self.knot_vec.len() - 1]);
        self.control_points.push(P::origin());
        for i in 0..=(k + 1) {
            let i0 = k + 1 - i;
            let a = (i0 as f64) / ((k + 1) as f64);
            let p = self.delta_control_points(i0) * a;
            self.control_points[i0] -= p;
        }
        self
    }

    pub fn elevate_degree(&mut self) -> &mut Self {
        let mut result = CurveCollector::Singleton;
        for mut bezier in self.bezier_decomposition() {
            result.concat(bezier.elevate_degree_bezier());
        }
        *self = result.unwrap();
        self
    }

    #[inline(always)]
    pub fn clamp(&mut self) -> &mut Self {
        let degree = self.degree();

        let s = self.knot_vec.multiplicity(0);
        for _ in s..=degree {
            self.add_knot(self.knot_vec[0]);
        }

        let n = self.knot_vec.len();
        let s = self.knot_vec.multiplicity(n - 1);
        for _ in s..=degree {
            self.add_knot(self.knot_vec[n - 1]);
        }
        self
    }

    pub fn optimize(&mut self) -> &mut Self {
        loop {
            let n = self.knot_vec.len();
            let closure = |flag, i| flag && self.try_remove_knot(n - i).is_err();
            if (1..=n).fold(true, closure) {
                break;
            }
        }
        self
    }

    pub fn syncro_degree(&mut self, other: &mut Self) {
        let (degree0, degree1) = (self.degree(), other.degree());
        for _ in degree0..degree1 {
            self.elevate_degree();
        }
        for _ in degree1..degree0 {
            other.elevate_degree();
        }
    }

    pub fn syncro_knots(&mut self, other: &mut BSplineCurve<P>) {
        self.knot_normalize();
        other.knot_normalize();

        let mut i = 0;
        let mut j = 0;
        while !self.knot(i).near2(&1.0) || !other.knot(j).near2(&1.0) {
            if self.knot(i) - other.knot(j) > TOLERANCE {
                self.add_knot(other.knot(j));
            } else if other.knot(j) - self.knot(i) > TOLERANCE {
                other.add_knot(self.knot(i));
            }
            i += 1;
            j += 1;
        }

        use std::cmp::Ordering;
        match usize::cmp(&self.knot_vec.len(), &other.knot_vec.len()) {
            Ordering::Less => {
                (0..(other.knot_vec.len() - self.knot_vec.len())).for_each(|_| {
                    self.add_knot(1.0);
                });
            }
            Ordering::Greater => (0..(self.knot_vec.len() - other.knot_vec.len())).for_each(|_| {
                other.add_knot(1.0);
            }),
            _ => {}
        }
    }

    pub fn bezier_decomposition(&self) -> Vec<BSplineCurve<P>> {
        let mut bspline = self.clone();
        bspline.clamp();
        let (knots, _) = self.knot_vec.to_single_multi();
        let n = knots.len();

        let mut result = Vec::new();
        for i in 2..n {
            result.push(bspline.cut(knots[n - i]));
        }
        result.push(bspline);
        result.reverse();
        result
    }

    pub fn make_locally_injective(&mut self) -> &mut Self {
        let mut iter = self.bezier_decomposition().into_iter();
        for bezier in iter.by_ref() {
            if !bezier.is_const() {
                *self = bezier;
                break;
            }
        }
        let mut x = 0.0;
        for mut bezier in iter {
            if bezier.is_const() {
                x += bezier.knot_vec.range_length();
            } else {
                self.concat(bezier.knot_translate(-x));
            }
        }
        self
    }

    #[inline(always)]
    pub fn near_as_curve(&self, other: &BSplineCurve<P>) -> bool {
        self.sub_near_as_curve(other, 1, |x, y| x.near(y))
    }

    #[inline(always)]
    pub fn near2_as_curve(&self, other: &BSplineCurve<P>) -> bool {
        self.sub_near_as_curve(other, 1, |x, y| x.near2(y))
    }
}

impl<P: ControlPoint<f64>> ParameterTransform for BSplineCurve<P> {
    #[inline(always)]
    fn parameter_transform(&mut self, scalar: f64, r#move: f64) -> &mut Self {
        self.knot_vec.transform(scalar, r#move);
        self
    }
}

impl<P: ControlPoint<f64> + Tolerance> Cut for BSplineCurve<P> {
    fn cut(&mut self, mut t: f64) -> BSplineCurve<P> {
        let degree = self.degree();

        let idx = match self.knot_vec.floor(t) {
            Some(idx) => idx,
            None => {
                let bspline = self.clone();
                let knot_vec = KnotVec::from(vec![t, self.knot_vec[0]]);
                let ctrl_pts = vec![P::origin()];
                *self = BSplineCurve::new(knot_vec, ctrl_pts);
                return bspline;
            }
        };
        let s = if t.near(&self.knot_vec[idx]) {
            t = self.knot_vec[idx];
            self.knot_vec.multiplicity(idx)
        } else {
            0
        };

        for _ in s..=degree {
            self.add_knot(t);
        }

        let k = self.knot_vec.floor(t).unwrap();
        let m = self.knot_vec.len();
        let n = self.control_points.len();
        let knot_vec0 = self.knot_vec.sub_vec(0..=k);
        let knot_vec1 = self.knot_vec.sub_vec((k - degree)..m);
        let control_points0 = Vec::from(&self.control_points[0..(k - degree)]);
        let control_points1 = Vec::from(&self.control_points[(k - degree)..n]);
        *self = BSplineCurve::new_unchecked(knot_vec0, control_points0);
        BSplineCurve::new_unchecked(knot_vec1, control_points1)
    }
}

impl<P: ControlPoint<f64> + Tolerance> Concat<BSplineCurve<P>> for BSplineCurve<P> {
    type Output = BSplineCurve<P>;

    fn try_concat(&self, other: &BSplineCurve<P>) -> std::result::Result<Self, ConcatError<P>> {
        let mut curve0 = self.clone();
        let mut curve1 = other.clone();
        curve0.syncro_degree(&mut curve1);
        curve0.clamp();
        curve1.clamp();
        curve0
            .knot_vec
            .try_concat(&curve1.knot_vec, curve0.degree())
            .map_err(|err| match err {
                Error::DifferentBackFront(a, b) => ConcatError::DisconnectedParameters(a, b),
                _ => unreachable!(),
            })?;
        let front = curve0.control_points.last().unwrap();
        let back = curve1.control_points.first().unwrap();
        if !front.near(back) {
            return Err(ConcatError::DisconnectedPoints(*front, *back));
        }
        curve0.control_points.extend(curve1.control_points);
        Ok(curve0)
    }
}

impl<P> ParameterDivision1D for BSplineCurve<P>
where
    P: ControlPoint<f64>
        + EuclideanSpace<Scalar = f64, Diff = <P as ControlPoint<f64>>::Diff>
        + MetricSpace<Metric = f64>
        + HashGen<f64>,
{
    type Point = P;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<P>) {
        algo::curve::parameter_division(self, range, tol)
    }
}

impl<P> BSplineCurve<P>
where
    P: ControlPoint<f64>
        + EuclideanSpace<Scalar = f64, Diff = <P as ControlPoint<f64>>::Diff>
        + MetricSpace<Metric = f64>
        + Tolerance,
    <P as ControlPoint<f64>>::Diff: InnerSpace<Scalar = f64> + Tolerance,
{
    pub fn is_arc_of(&self, curve: &BSplineCurve<P>, mut hint: f64) -> Option<f64> {
        let degree = std::cmp::max(self.degree(), curve.degree()) * 3 + 1;
        let (knots, _) = self.knot_vec.to_single_multi();
        if !self.subs(knots[0]).near(&curve.subs(hint)) {
            return None;
        }

        for i in 1..knots.len() {
            let range = knots[i] - knots[i - 1];
            for j in 1..=degree {
                let t = knots[i - 1] + range * (j as f64) / (degree as f64);
                let pt = ParametricCurve::subs(self, t);
                let res = curve.search_nearest_parameter(pt, Some(hint), 100);
                let flag = res.map(|res| hint <= res && curve.subs(res).near(&pt));
                hint = match flag {
                    Some(true) => res.unwrap(),
                    _ => return None,
                };
            }
        }
        Some(hint)
    }
}
impl<P> SearchNearestParameter<D1> for BSplineCurve<P>
where
    P: ControlPoint<f64>
        + EuclideanSpace<Scalar = f64, Diff = <P as ControlPoint<f64>>::Diff>
        + MetricSpace<Metric = f64>
        + Tolerance,
    <P as ControlPoint<f64>>::Diff: InnerSpace<Scalar = f64> + Tolerance,
{
    type Point = P;

    #[inline(always)]
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        point: P,
        hint: H,
        trial: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(hint) => hint,
            SPHint1D::Range(x, y) => {
                algo::curve::presearch(self, point, (x, y), PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_nearest_parameter(self, point, hint, trial)
    }
}
impl<P> SearchParameter<D1> for BSplineCurve<P>
where
    P: ControlPoint<f64>
        + EuclideanSpace<Scalar = f64, Diff = <P as ControlPoint<f64>>::Diff>
        + MetricSpace<Metric = f64>,
    <P as ControlPoint<f64>>::Diff: InnerSpace<Scalar = f64> + Tolerance,
{
    type Point = P;
    #[inline(always)]
    fn search_parameter<H: Into<SPHint1D>>(&self, point: P, hint: H, trial: usize) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(hint) => hint,
            SPHint1D::Range(x, y) => {
                algo::curve::presearch(self, point, (x, y), PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_parameter(self, point, hint, trial)
    }
}

impl<P: Bounded> BSplineCurve<P> {
    #[inline(always)]
    pub fn roughly_bounding_box(&self) -> BoundingBox<P> {
        self.control_points.iter().collect()
    }
}

impl<P: Clone> Invertible for BSplineCurve<P> {
    #[inline(always)]
    fn invert(&mut self) {
        self.knot_vec.invert();
        self.control_points.reverse();
    }
}

impl<M, P> Transformed<M> for BSplineCurve<P>
where
    P: EuclideanSpace,
    M: Transform<P>,
{
    #[inline(always)]
    fn transform_by(&mut self, trans: M) {
        self.control_points
            .iter_mut()
            .for_each(|pt| *pt = trans.transform_point(*pt))
    }
}

impl<'de, P> Deserialize<'de> for BSplineCurve<P>
where
    P: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BSplineCurve_<P> {
            knot_vec: KnotVec,
            control_points: Vec<P>,
        }
        let BSplineCurve_ {
            knot_vec,
            control_points,
        } = BSplineCurve_::<P>::deserialize(deserializer)?;
        Self::try_new(knot_vec, control_points).map_err(serde::de::Error::custom)
    }
}

impl<P> BSplineCurve<P>
where
    P: ControlPoint<f64>
        + EuclideanSpace<Scalar = f64, Diff = <P as ControlPoint<f64>>::Diff>
        + MetricSpace<Metric = f64>
        + Tolerance
        + HashGen<f64>,
    <P as ControlPoint<f64>>::Diff: InnerSpace<Scalar = f64> + Tolerance,
{
    fn cubic_bezier_interpolation(
        pt0: P,
        pt1: P,
        der0: <P as EuclideanSpace>::Diff,
        der1: <P as EuclideanSpace>::Diff,
        range: (f64, f64),
    ) -> Self {
        let width = range.1 - range.0;
        let mut knot_vec = KnotVec::bezier_knot(3);
        knot_vec.transform(width, range.0);
        Self::debug_new(
            knot_vec,
            vec![pt0, pt0 + der0 * width / 3.0, pt1 - der1 * width / 3.0, pt1],
        )
    }

    fn sub_cubic_approximation<C>(
        curve: &C,
        range: (f64, f64),
        ends: (P, P),
        enders: (<P as EuclideanSpace>::Diff, <P as EuclideanSpace>::Diff),
        p_tol: f64,
        d_tol: f64,
        trialis: usize,
    ) -> Option<Self>
    where
        C: ParametricCurve<Point = P, Vector = <P as EuclideanSpace>::Diff>,
    {
        let bezier = Self::cubic_bezier_interpolation(ends.0, ends.1, enders.0, enders.1, range);
        let gen = ends.0.midpoint(ends.1);
        let p = 0.5 + (0.2 * HashGen::hash1(gen) - 0.1);
        let t = range.0 * (1.0 - p) + range.1 * p;
        let ders0 = bezier.ders(1, t);
        let ders1 = curve.ders(1, t);
        let pt_dist2 = (ders0[0] - ders1[0]).magnitude2();
        let der_dist2 = (ders0[1] - ders1[1]).magnitude2();
        if pt_dist2 > p_tol * p_tol || der_dist2 > d_tol * d_tol {
            if trialis == 0 {
                return None;
            }
            let t = (range.0 + range.1) / 2.0;
            let ders = curve.ders(1, t);
            let pt = <P as EuclideanSpace>::from_vec(ders[0]);
            let bspcurve0 = Self::sub_cubic_approximation(
                curve,
                (range.0, t),
                (ends.0, pt),
                (enders.0, ders[1]),
                p_tol,
                d_tol,
                trialis - 1,
            );
            let bspcurve1 = Self::sub_cubic_approximation(
                curve,
                (t, range.1),
                (pt, ends.1),
                (ders[1], enders.1),
                p_tol,
                d_tol,
                trialis - 1,
            );
            match (bspcurve0, bspcurve1) {
                (Some(x), Some(y)) => x.try_concat(&y).ok(),
                _ => None,
            }
        } else {
            Some(bezier)
        }
    }

    pub fn cubic_approximation<C>(
        curve: &C,
        range: (f64, f64),
        p_tol: f64,
        d_tol: f64,
        trials: usize,
    ) -> Option<Self>
    where
        C: ParametricCurve<Point = P, Vector = <P as EuclideanSpace>::Diff>,
    {
        let ends = (curve.subs(range.0), curve.subs(range.1));
        let enders = (curve.der(range.0), curve.der(range.1));
        Self::sub_cubic_approximation(curve, range, ends, enders, p_tol, d_tol, trials).map(
            |mut x| {
                x.optimize();
                x
            },
        )
    }
}

#[test]
fn cubic_bezier_interpolation_test() {
    let pt0 = Point2::new(0.0, 0.0);
    let pt1 = Point2::new(1.0, 0.0);
    let der0 = Vector2::new(0.5, 1.0);
    let der1 = Vector2::new(0.5, -1.0);
    let bspcurve = BSplineCurve::cubic_bezier_interpolation(pt0, pt1, der0, der1, (1.23, 3.45));
    let der = bspcurve.derivation();
    assert_near!(bspcurve.front(), pt0);
    assert_near!(bspcurve.back(), pt1);
    assert_near!(der.front(), der0);
    assert_near!(der.back(), der1);
}
