use std::ops::{Mul, Sub};

use crate::{cgmath64::*, tolerance::*};

#[derive(Clone, Debug)]
pub struct CalcOutput<V, M> {
    pub value: V,
    pub derivation: M,
}

pub trait Jacobian<V>: Mul<V, Output = V> + Sized {
    #[doc(hidden)]
    fn invert(self) -> Option<Self>;
}

impl Jacobian<f64> for f64 {
    #[inline(always)]
    fn invert(self) -> Option<Self> {
        match self.is_zero() {
            true => None,
            false => Some(1.0 / self),
        }
    }
}

macro_rules! impl_jacobian {
    ($matrix: ty, $vector: ty) => {
        impl Jacobian<$vector> for $matrix {
            #[inline(always)]
            fn invert(self) -> Option<Self> {
                SquareMatrix::invert(&self)
            }
        }
    };
}

impl_jacobian!(Matrix2, Vector2);
impl_jacobian!(Matrix3, Vector3);
impl_jacobian!(Matrix4, Vector4);

pub fn solve<V, M>(
    function: impl Fn(V) -> CalcOutput<V, M>,
    mut hint: V,
    trials: usize,
) -> Result<V, NewtonLog<V>>
where
    V: Sub<Output = V> + Copy + Tolerance,
    M: Jacobian<V>,
{
    let mut log = NewtonLog::new(cfg!(debug_assertions), trials);
    for _ in 0..=trials {
        log.push(hint);
        let CalcOutput { value, derivation } = function(hint);
        let Some(inv) = derivation.invert() else {
            log.set_degenerate(true);
            return Err(log);
        };
        let next = hint - inv * value;
        if next.near2(&hint) {
            return Ok(hint);
        }
        hint = next;
    }
    Err(log)
}

mod newtonlog {
    use std::fmt::*;
    #[derive(Clone, Debug)]
    pub struct NewtonLog<T> {
        log: Option<Vec<T>>,
        degenerate: bool,
    }

    impl<T> NewtonLog<T> {
        #[inline(always)]
        pub fn new(activate: bool, trials: usize) -> Self {
            match activate {
                true => NewtonLog {
                    log: Some(Vec::with_capacity(trials)),
                    degenerate: false,
                },
                false => NewtonLog {
                    log: None,
                    degenerate: false,
                },
            }
        }
        #[inline(always)]
        pub fn degenerate(&self) -> bool {
            self.degenerate
        }
        #[inline(always)]
        pub(super) fn push(&mut self, log: T) {
            if let Some(vec) = &mut self.log {
                vec.push(log)
            }
        }
        #[inline(always)]
        pub(super) fn set_degenerate(&mut self, degenerate: bool) {
            self.degenerate = degenerate
        }
    }

    impl<T: Debug> Display for NewtonLog<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self.degenerate {
                true => f.pad("Jacobian is dengenerate. ")?,
                false => f.pad("Newton method is not converges. ")?,
            }
            match &self.log {
                None => f.pad(
                    "If you want to see the Newton log, please re-run it with the debug build.",
                ),
                Some(vec) => {
                    f.pad("Newton Log:\n")?;
                    vec.iter()
                        .try_for_each(|log| f.write_fmt(format_args!("{log:?}\n")))
                }
            }
        }
    }
}
pub use newtonlog::NewtonLog;
