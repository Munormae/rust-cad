#![allow(missing_docs)]

use crate::cgmath64::*;
use cgmath::AbsDiffEq;
use std::fmt::Debug;

pub const TOLERANCE: f64 = 1.0e-6;

pub const TOLERANCE2: f64 = TOLERANCE * TOLERANCE;

pub trait Tolerance: AbsDiffEq<Epsilon = f64> + Debug {
    fn near(&self, other: &Self) -> bool {
        self.abs_diff_eq(other, TOLERANCE)
    }
    fn near2(&self, other: &Self) -> bool {
        self.abs_diff_eq(other, TOLERANCE2)
    }
}

impl<T: AbsDiffEq<Epsilon = f64> + Debug> Tolerance for T {}

#[macro_export]
macro_rules! assert_near {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

#[macro_export]
macro_rules! prop_assert_near {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?}, right: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

#[test]
#[should_panic]
fn assert_near_without_msg() {
    assert_near!(1.0, 2.0)
}

#[test]
#[should_panic]
fn assert_near_with_msg() {
    assert_near!(1.0, 2.0, "{}", "test OK")
}

#[macro_export]
macro_rules! assert_near2 {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

#[macro_export]
macro_rules! prop_assert_near2 {
    ($left: expr, $right: expr $(,)?) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}",
        )
    }};
    ($left: expr, $right: expr, $($arg: tt)+) => {{
        let (left, right) = ($left, $right);
        prop_assert!(
            $crate::tolerance::Tolerance::near2(&left, &right),
            "assertion failed: `left` is near `right`\nleft: {left:?},\nright: {right:?}: {}",
            format_args!($($arg)+),
        )
    }};
}

#[test]
#[should_panic]
fn assert_near2_without_msg() {
    assert_near2!(1.0, 2.0)
}

#[test]
#[should_panic]
fn assert_near2_with_msg() {
    assert_near2!(1.0, 2.0, "{}", "test OK")
}

pub trait Origin: Tolerance + Zero {
    #[inline(always)]
    fn so_small(&self) -> bool {
        self.near(&Self::zero())
    }

    #[inline(always)]
    fn so_small2(&self) -> bool {
        self.near2(&Self::zero())
    }
}

impl<T: Tolerance + Zero> Origin for T {}
