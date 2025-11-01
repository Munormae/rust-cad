//! Extensions for working with homogeneous and control-point arithmetic.
#![allow(missing_docs)]

use cgmath::*;

pub mod control_point {
    //! РЈРЅРёС„РёС†РёСЂРѕРІР°РЅРЅС‹Р№ С‚СЂРµР№С‚ РґР»СЏ РєРѕРЅС‚СЂРѕР»СЊРЅС‹С… С‚РѕС‡РµРє СЂР°Р·РЅРѕР№ СЂР°Р·РјРµСЂРЅРѕСЃС‚Рё.
    use cgmath::{
        BaseFloat, EuclideanSpace, Point1, Point2, Point3, Vector1, Vector2, Vector3, Vector4, Zero,
    };
    use std::fmt::Debug;
    use std::ops::*;

    /// РљРѕРЅС‚СЂРѕР»СЊРЅР°СЏ С‚РѕС‡РєР° NURBS/Bezier СЃ РїРѕРґРґРµСЂР¶РєРѕР№ Р°СЂРёС„РјРµС‚РёРєРё Рё РёРЅРґРµРєСЃР°С†РёРё.
    pub trait ControlPoint<S>:
        Add<Self::Diff, Output = Self>
        + Sub<Self::Diff, Output = Self>
        + Sub<Self, Output = Self::Diff>
        + Mul<S, Output = Self>
        + Div<S, Output = Self>
        + AddAssign<Self::Diff>
        + SubAssign<Self::Diff>
        + MulAssign<S>
        + DivAssign<S>
        + Copy
        + Clone
        + Debug
        + Index<usize, Output = S>
        + IndexMut<usize, Output = S>
    {
        type Diff: Add<Self::Diff, Output = Self::Diff>
            + Sub<Self::Diff, Output = Self::Diff>
            + Mul<S, Output = Self::Diff>
            + Div<S, Output = Self::Diff>
            + AddAssign<Self::Diff>
            + SubAssign<Self::Diff>
            + MulAssign<S>
            + DivAssign<S>
            + Zero
            + Copy
            + Clone
            + Debug
            + Index<usize, Output = S>
            + IndexMut<usize, Output = S>;
        /// Р Р°Р·РјРµСЂРЅРѕСЃС‚СЊ РїСЂРѕСЃС‚СЂР°РЅСЃС‚РІР° РєРѕРЅС‚СЂРѕР»СЊРЅРѕР№ С‚РѕС‡РєРё.
        const DIM: usize;
        /// РўРѕС‡РєР° РІ РЅР°С‡Р°Р»Рµ РєРѕРѕСЂРґРёРЅР°С‚ РёР»Рё РЅСѓР»РµРІРѕР№ РІРµРєС‚РѕСЂ.
        fn origin() -> Self;
        /// РџСЂРµРѕР±СЂР°Р·СѓРµС‚ С‚РѕС‡РєСѓ РІ РІРµРєС‚РѕСЂ СЂР°Р·РЅРѕСЃС‚РµР№.
        fn to_vec(self) -> Self::Diff;
        /// РЎРѕР·РґР°С‘С‚ С‚РѕС‡РєСѓ РёР· РІРµРєС‚РѕСЂР° СЂР°Р·РЅРѕСЃС‚РµР№.
        fn from_vec(vec: Self::Diff) -> Self;
    }
    impl<S: BaseFloat> ControlPoint<S> for Point1<S> {
        type Diff = Vector1<S>;
        const DIM: usize = 1;
        fn origin() -> Self {
            EuclideanSpace::origin()
        }
        fn to_vec(self) -> Self::Diff {
            EuclideanSpace::to_vec(self)
        }
        fn from_vec(vec: Self::Diff) -> Self {
            EuclideanSpace::from_vec(vec)
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Point2<S> {
        type Diff = Vector2<S>;
        const DIM: usize = 2;
        fn origin() -> Self {
            EuclideanSpace::origin()
        }
        fn to_vec(self) -> Self::Diff {
            EuclideanSpace::to_vec(self)
        }
        fn from_vec(vec: Self::Diff) -> Self {
            EuclideanSpace::from_vec(vec)
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Point3<S> {
        type Diff = Vector3<S>;
        const DIM: usize = 3;
        fn origin() -> Self {
            EuclideanSpace::origin()
        }
        fn to_vec(self) -> Self::Diff {
            EuclideanSpace::to_vec(self)
        }
        fn from_vec(vec: Self::Diff) -> Self {
            EuclideanSpace::from_vec(vec)
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Vector1<S> {
        type Diff = Vector1<S>;
        const DIM: usize = 1;
        fn origin() -> Self {
            Zero::zero()
        }
        fn to_vec(self) -> Self {
            self
        }
        fn from_vec(vec: Self::Diff) -> Self {
            vec
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Vector2<S> {
        type Diff = Vector2<S>;
        const DIM: usize = 2;
        fn origin() -> Self {
            Zero::zero()
        }
        fn to_vec(self) -> Self {
            self
        }
        fn from_vec(vec: Self::Diff) -> Self {
            vec
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Vector3<S> {
        type Diff = Vector3<S>;
        const DIM: usize = 3;
        fn origin() -> Self {
            Zero::zero()
        }
        fn to_vec(self) -> Self {
            self
        }
        fn from_vec(vec: Self::Diff) -> Self {
            vec
        }
    }
    impl<S: BaseFloat> ControlPoint<S> for Vector4<S> {
        type Diff = Vector4<S>;
        const DIM: usize = 4;
        fn origin() -> Self {
            Zero::zero()
        }
        fn to_vec(self) -> Self {
            self
        }
        fn from_vec(vec: Self::Diff) -> Self {
            vec
        }
    }
}

/// Р’РµРєС‚РѕСЂ РІ РѕРґРЅРѕСЂРѕРґРЅС‹С… РєРѕРѕСЂРґРёРЅР°С‚Р°С… СЃ РїРѕРґРґРµСЂР¶РєРѕР№ РѕРїРµСЂР°С†РёРё РІС‹РґРµР»РµРЅРёСЏ РІРµСЃР°.
pub trait Homogeneous: VectorSpace {
    /// РЎРІСЏР·Р°РЅРЅР°СЏ С‚РѕС‡РєР° РІ РґРµРєР°СЂС‚РѕРІРѕРј РїСЂРѕСЃС‚СЂР°РЅСЃС‚РІРµ.
    type Point: EuclideanSpace<Scalar = Self::Scalar>;
    /// РћС‚Р±СЂР°СЃС‹РІР°РµС‚ РІРµСЃРѕРІСѓСЋ РєРѕРјРїРѕРЅРµРЅС‚Сѓ.
    fn truncate(self) -> <Self::Point as EuclideanSpace>::Diff;
    /// Р’РѕР·РІСЂР°С‰Р°РµС‚ РІРµСЃ.
    fn weight(self) -> Self::Scalar;
    /// РЎС‚СЂРѕРёС‚ РѕРґРЅРѕСЂРѕРґРЅС‹Р№ РІРµРєС‚РѕСЂ РёР· С‚РѕС‡РєРё (СЃ РІРµСЃРѕРј 1).
    fn from_point(point: Self::Point) -> Self;
    #[inline(always)]
    /// РЎС‚СЂРѕРёС‚ РѕРґРЅРѕСЂРѕРґРЅС‹Р№ РІРµРєС‚РѕСЂ РїРѕ С‚РѕС‡РєРµ Рё РїСЂРѕРёР·РІРѕР»СЊРЅРѕРјСѓ РІРµСЃСѓ.
    fn from_point_weight(point: Self::Point, weight: Self::Scalar) -> Self {
        Self::from_point(point) * weight
    }
    #[inline(always)]
    /// Р’РѕР·РІСЂР°С‰Р°РµС‚ С‚РѕС‡РєСѓ, РЅРѕСЂРјР°Р»РёР·РѕРІР°РІ РІРµСЃ.
    fn to_point(self) -> Self::Point {
        Self::Point::from_vec(self.truncate() / self.weight())
    }
}

/// Р’С‹С‡РёСЃР»СЏРµС‚ СЂР°С†РёРѕРЅР°Р»СЊРЅСѓСЋ РїСЂРѕРёР·РІРѕРґРЅСѓСЋ Р·Р°РґР°РЅРЅРѕРіРѕ РїРѕСЂСЏРґРєР° РёР· РјР°СЃСЃРёРІР° РѕРґРЅРѕСЂРѕРґРЅС‹С… РїСЂРѕРёР·РІРѕРґРЅС‹С….
pub fn rat_der<V: Homogeneous>(ders: &[V]) -> <V::Point as EuclideanSpace>::Diff {
    let zero = <V::Point as EuclideanSpace>::Diff::zero();
    let len = ders.len();
    if len == 0 {
        zero
    } else if len == 1 {
        ders[0].to_point().to_vec()
    } else if len == 2 {
        let (s, sw) = (ders[0].truncate(), ders[0].weight());
        let (d, dw) = (ders[1].truncate(), ders[1].weight());
        (d * sw - s * dw) / (sw * sw)
    } else if len == 3 {
        let (s, sw) = (ders[0].truncate(), ders[0].weight());
        let (d, dw) = (ders[1].truncate(), ders[1].weight());
        let (d2, d2w) = (ders[2].truncate(), ders[2].weight());
        let two = <V::Scalar as num_traits::NumCast>::from(2).unwrap();
        let sw2 = sw * sw;
        d2 / sw - d * (dw / sw2 * two) + s * (dw * dw * two / (sw2 * sw) - d2w / sw2)
    } else if len < 32 {
        let mut evals = [zero; 32];
        rat_ders(ders, &mut evals);
        evals[ders.len() - 1]
    } else {
        let mut evals = vec![zero; ders.len()];
        rat_ders(ders, &mut evals);
        evals[ders.len() - 1]
    }
}

/// Р—Р°РїРѕР»РЅСЏРµС‚ РјР°СЃСЃРёРІ `evals` СЂР°С†РёРѕРЅР°Р»СЊРЅС‹РјРё РїСЂРѕРёР·РІРѕРґРЅС‹РјРё РІСЃРµС… РїРѕСЂСЏРґРєРѕРІ РґРѕ `ders.len()`.
pub fn rat_ders<V: Homogeneous>(ders: &[V], evals: &mut [<V::Point as EuclideanSpace>::Diff]) {
    assert!(evals.len() >= ders.len(),);
    let from = <V::Scalar as num_traits::NumCast>::from;
    for i in 0..ders.len() {
        let mut c = 1;
        let sum = (1..i).fold(evals[0] * ders[i].weight(), |sum, j| {
            c = c * (i - j + 1) / j;
            sum + evals[j] * (ders[i - j].weight() * from(c).unwrap())
        });
        evals[i] = (ders[i].truncate() - sum) / ders[0].weight();
    }
}

/// РњРЅРѕРіРѕРјРµСЂРЅР°СЏ СЂР°С†РёРѕРЅР°Р»СЊРЅР°СЏ РїСЂРѕРёР·РІРѕРґРЅР°СЏ РґР»СЏ РєРѕРјР±РёРЅРёСЂРѕРІР°РЅРЅРѕРіРѕ РјР°СЃСЃРёРІР° `ders`.
pub fn multi_rat_der<V, A>(ders: &[A]) -> <V::Point as EuclideanSpace>::Diff
where
    V: Homogeneous,
    A: AsRef<[V]>,
{
    let zero = <V::Point as EuclideanSpace>::Diff::zero();
    if ders.is_empty() {
        return zero;
    }
    let (m, n) = (ders.len(), ders[0].as_ref().len());
    if n == 0 {
        zero
    } else if (m, n) == (1, 1) {
        ders[0].as_ref()[0].to_point().to_vec()
    } else if m == 1 {
        rat_der(ders[0].as_ref())
    } else if (m, n) == (2, 1) {
        rat_der(&[ders[0].as_ref()[0], ders[1].as_ref()[0]])
    } else if n == 1 && m < 32 {
        let mut vders = [V::zero(); 32];
        for (vder, array) in vders.iter_mut().zip(ders) {
            *vder = array.as_ref()[0];
        }
        rat_der(&vders[..m])
    } else if n == 1 {
        let mut vders = vec![V::zero(); m];
        for (vder, array) in vders.iter_mut().zip(ders) {
            *vder = array.as_ref()[0];
        }
        rat_der(&vders)
    } else if (m, n) == (2, 2) {
        let two = <V::Scalar as num_traits::NumCast>::from(2).unwrap();
        let (der0, der1) = (ders[0].as_ref(), ders[1].as_ref());
        let (s, u, v, uv) = (der0[0], der1[0], der0[1], der1[1]);
        let (s, sw) = (s.truncate(), s.weight());
        let (u, uw) = (u.truncate(), u.weight());
        let (v, vw) = (v.truncate(), v.weight());
        let (uv, uvw) = (uv.truncate(), uv.weight());
        let sw2 = sw * sw;
        uv / sw - u * (vw / sw2) - v * (uw / sw2) + s * (uw * vw * two / (sw2 * sw) - uvw / sw2)
    } else if m < 8 && n < 8 {
        let mut evals = [[zero; 8]; 8];
        multi_rat_ders(ders, &mut evals);
        evals[m - 1][n - 1]
    } else {
        let mut evals = vec![vec![zero; m]; n];
        multi_rat_ders(ders, &mut evals);
        evals[m - 1][n - 1]
    }
}

/// РћРґРЅРѕРІСЂРµРјРµРЅРЅРѕРµ РІС‹С‡РёСЃР»РµРЅРёРµ РІСЃРµС… СЃРјРµС€Р°РЅРЅС‹С… СЂР°С†РёРѕРЅР°Р»СЊРЅС‹С… РїСЂРѕРёР·РІРѕРґРЅС‹С….
pub fn multi_rat_ders<V, A0, A1>(ders: &[A0], evals: &mut [A1])
where
    V: Homogeneous,
    A0: AsRef<[V]>,
    A1: AsMut<[<V::Point as EuclideanSpace>::Diff]>,
{
    let from = <V::Scalar as num_traits::NumCast>::from;
    let (m_max, n_max) = (ders.len(), ders[0].as_ref().len());
    for m in 0..m_max {
        for n in 0..n_max {
            let mut sum = <V::Point as EuclideanSpace>::Diff::zero();
            let mut c0 = 1;
            for i in 0..=m {
                let mut c1 = 1;
                let (evals, ders) = (evals[i].as_mut(), ders[m - i].as_ref());
                for j in 0..=n {
                    let (c0_s, c1_s) = (from(c0).unwrap(), from(c1).unwrap());
                    sum = sum + evals[j] * (ders[n - j].weight() * c0_s * c1_s);
                    c1 = c1 * (n - j) / (j + 1);
                }
                c0 = c0 * (m - i) / (i + 1);
            }
            let (eval_mn, der_mn) = (&mut evals[m].as_mut()[n], ders[m].as_ref()[n]);
            *eval_mn = (der_mn.truncate() - sum) / ders[0].as_ref()[0].weight();
        }
    }
}

impl<S: BaseFloat> Homogeneous for Vector2<S> {
    type Point = Point1<S>;
    #[inline(always)]
    fn truncate(self) -> Vector1<S> {
        Vector1::new(self[0])
    }
    #[inline(always)]
    fn weight(self) -> S {
        self[1]
    }
    #[inline(always)]
    fn from_point(point: Self::Point) -> Self {
        Vector2::new(point[0], S::one())
    }
}

impl<S: BaseFloat> Homogeneous for Vector3<S> {
    type Point = Point2<S>;
    #[inline(always)]
    fn truncate(self) -> Vector2<S> {
        self.truncate()
    }
    #[inline(always)]
    fn weight(self) -> S {
        self[2]
    }
    #[inline(always)]
    fn from_point(point: Self::Point) -> Self {
        Vector3::new(point[0], point[1], S::one())
    }
}

impl<S: BaseFloat> Homogeneous for Vector4<S> {
    type Point = Point3<S>;
    #[inline(always)]
    fn truncate(self) -> Vector3<S> {
        self.truncate()
    }
    #[inline(always)]
    fn weight(self) -> S {
        self[3]
    }
    #[inline(always)]
    fn from_point(point: Self::Point) -> Self {
        point.to_homogeneous()
    }
}

/// РњРѕРґСѓР»СЊ РїСЂРѕРёР·РІРѕРґРЅС‹С… РєСЂРёРІРѕР№ (РїРѕР»СѓС‡Р°РµС‚ Р·РЅР°С‡РµРЅРёСЏ РїРѕ РґР»РёРЅРµ РґСѓРіРё).
pub fn abs_ders<V>(ders: &[V], evals: &mut [f64])
where
    V: InnerSpace<Scalar = f64>,
{
    assert!(evals.len() >= ders.len(),);
    let n = ders.len();
    evals.iter_mut().for_each(|o| *o = 0.0);

    if n == 0 {
        return;
    }
    evals[0] = ders[0].magnitude();
    (1..n).for_each(|m| {
        let mut c = 1;
        let sum = (0..m).fold(0.0, |mut sum, i| {
            let x = ders[i + 1].dot(ders[m - 1 - i]);
            let y = evals[i + 1] * evals[m - 1 - i];
            sum += (x - y) * c as f64;
            c = c * (m - 1 - i) / (i + 1);
            sum
        });
        evals[m] = sum / evals[0];
    });
}
