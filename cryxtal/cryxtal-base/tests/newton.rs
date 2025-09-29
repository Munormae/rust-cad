use proptest::prelude::*;
use std::f64::consts::PI;
use cryxtal_base::{cgmath64::*, newton::*, prop_assert_near, tolerance::*};

proptest! {
    #[test]
    fn test_newton1(
        a in prop::array::uniform4(-3.0f64..=3.0f64),
        x0 in -5.0f64..=5.0f64,
        delta in -0.25f64..=0.25f64,
    ) {
        let poly = |x: f64| a[0] + a[1] * x + a[2] * x * x + a[3] * x * x * x;
        let function = |x: f64| CalcOutput {
            value: poly(x) - poly(x0),
            derivation: a[1] + 2.0 * a[2] * x + 3.0 * a[3] * x * x,
        };
        // Ensure Newton's method has a reasonable slope near the root to avoid non-convergence.
        // Conditioning: derivative near start is not tiny and first Newton step is not huge
        let x_start = x0 + delta;
        let deriv_start = a[1] + 2.0 * a[2] * x_start + 3.0 * a[3] * x_start * x_start;
        let f_start = poly(x_start) - poly(x0);
        prop_assume!(deriv_start.abs() > 1.0e-6);
        prop_assume!((f_start / deriv_start).abs() < 1.0);
        match solve(function, x0 + delta, 200) {
            Ok(res) => prop_assert_near!(function(res).value, 0.0),
            Err(log) => prop_assert!(log.degenerate(), "{log}"),
        }
    }

    #[test]
    fn test_newton2(
        n in prop::array::uniform2(-10.0f64..=10.0f64),
        delta in prop::array::uniform2(-0.5f64..=0.5f64),
    ) {
        let n = Vector2::from(n);
        if n.so_small() {
            return Ok(());
        }
        let n = n.normalize();
        let function = |vec: Vector2| CalcOutput {
            value: Vector2::new(vec.magnitude2() - 1.0, vec.dot(n)),
            derivation: Matrix2::new(2.0 * vec.x, n.x, 2.0 * vec.y, n.y),
        };
        let hint = Matrix2::from_angle(-Rad(PI / 2.0)) * n + Vector2::from(delta);
        match solve(function, hint, 10) {
            Ok(res) => prop_assert_near!(function(res).value, Vector2::zero()),
            Err(log) => prop_assert!(log.degenerate(), "{log}"),
        }
    }
}
