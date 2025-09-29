use proptest::prelude::*;
use std::f64::consts::PI;
use cryxtal_geometry::prelude::*;
use cryxtal_base::cgmath64::Vector2;
use cryxtal_base::prop_assert_near;

proptest! {
    #[test]
    fn test_der_mn(
        (u, v) in (0f64..=PI, 0f64..=2.0 * PI),
        (m, n) in (0usize..=4, 0usize..=4),
        center in prop::array::uniform3(-100f64..=100.0),
        radius in 0.1f64..=10.0,
        u_derivate in prop::bool::ANY,
    ) {
        let sphere = Sphere::new(Point3::from(center), radius);

        const EPS: f64 = 1.0e-4;
        let (der0, der1) = if u_derivate {
            let der0 = sphere.der_mn(m + 1, n, u, v);
            let der1 = (sphere.der_mn(m, n, u + EPS, v) - sphere.der_mn(m, n, u - EPS, v)) / (2.0 * EPS);
            (der0, der1)
        } else {
            let der0 = sphere.der_mn(m, n + 1, u, v);
            let der1 = (sphere.der_mn(m, n, u, v + EPS) - sphere.der_mn(m, n, u, v - EPS)) / (2.0 * EPS);
            (der0, der1)
        };
        prop_assert!((der0 - der1).magnitude() < 0.01 * der0.magnitude());
    }
}

fn exec_search_parameter_test(
    center: [f64; 3],
    radius: f64,
    (u, v): (f64, f64),
    disp: [f64; 3],
    sign: [bool; 3],
) -> std::result::Result<(), TestCaseError> {
    let center = Point3::from(center);
    let sphere = Sphere::new(center, radius);
    let pt = sphere.subs(u, v);
    let (u0, v0) = sphere.search_parameter(pt, None, 100).unwrap();
    prop_assert_near!(Vector2::new(u, v), Vector2::new(u0, v0));
    let boolnum = |t: bool| if t { 1.0 } else { -1.0 };
    let pt = pt
        + Vector3::new(
            disp[0] * boolnum(sign[0]),
            disp[1] * boolnum(sign[1]),
            disp[2] * boolnum(sign[2]),
        );
    prop_assert!(sphere.search_parameter(pt, None, 100).is_none());
    let (u, v) = sphere.search_nearest_parameter(pt, None, 100).unwrap();
    prop_assert_near!(
        sphere.subs(u, v),
        center + (pt - center).normalize() * radius
    );
    Ok(())
}

proptest! {
    #[test]
    fn search_parameter_test(
        center in prop::array::uniform3(-50f64..=50f64),
        radius in 0.1f64..100f64,
        (u, v) in (0f64..=PI, 0f64..=(2.0 * PI)),
        disp in prop::array::uniform3(0.01f64..0.1f64),
        sign in prop::array::uniform3(prop::bool::ANY),
    ) {
        exec_search_parameter_test(center, radius, (u, v), disp, sign)?;
    }

}

#[test]
fn sphere_derivation_test() {
    let center = Point3::new(1.0, 2.0, 3.0);
    let radius = 4.56;
    let sphere = Sphere::new(center, radius);
    const N: usize = 100;
    for i in 0..N {
        for j in 0..N {
            let u = PI * i as f64 / N as f64;
            let v = 2.0 * PI * j as f64 / N as f64;
            let normal = sphere.normal(u, v);
            assert!(normal.dot(sphere.uder(u, v)).so_small());
            assert!(normal.dot(sphere.vder(u, v)).so_small());
        }
    }
}
