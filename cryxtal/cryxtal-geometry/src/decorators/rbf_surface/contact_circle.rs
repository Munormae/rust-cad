use super::*;

pub(super) trait FilletableSurface:
    ParametricSurface3D + SearchParameter<D2, Point = Point3>
{
}
impl<S: ParametricSurface3D + SearchParameter<D2, Point = Point3>> FilletableSurface for S {}

impl ContactCircle {
    #[inline]
    pub const fn center(self) -> Point3 {
        self.center
    }

    #[inline]
    pub const fn axis(self) -> Vector3 {
        self.axis
    }

    #[inline]
    pub const fn angle(self) -> Rad<f64> {
        self.angle
    }

    #[inline]
    pub const fn curve_parameter(self) -> f64 {
        self.t
    }

    #[inline]
    pub const fn contact_point0(self) -> ContactPoint {
        self.contact_point0
    }

    #[inline]
    pub const fn contact_point1(self) -> ContactPoint {
        self.contact_point1
    }

    pub(super) fn try_new(
        point_on_curve: (Point3, Vector3),
        t: f64,
        surface0: &impl FilletableSurface,
        surface1: &impl FilletableSurface,
        radius: f64,
    ) -> Option<Self> {
        let (p, der) = point_on_curve;
        let (mut p0, mut p1) = (p, p);
        let (mut u0, mut v0) = surface0.search_parameter(p0, None, 10)?;
        let (mut u1, mut v1) = surface1.search_parameter(p1, None, 10)?;
        let center = (0..100).find_map(|_| {
            let (n0, n1) = (surface0.normal(u0, v0), surface1.normal(u1, v1));
            let (c, q0, q1) = contact_points((p, der), (p0, n0), (p1, n1), radius);
            if p0.near(&q0) && p1.near(&q1) {
                Some(c)
            } else {
                (p0, (u0, v0)) = next_point(surface0, (u0, v0), (p0, q0));
                (p1, (u1, v1)) = next_point(surface1, (u1, v1), (p1, q1));
                None
            }
        })?;
        let (vec0, vec1) = (p0 - center, p1 - center);
        Some(Self {
            center,
            axis: vec0.cross(vec1).normalize(),
            angle: vec0.angle(vec1),
            t,
            contact_point0: (p0, (u0, v0).into()).into(),
            contact_point1: (p1, (u1, v1).into()).into(),
        })
    }
}

impl ParametricCurve for ContactCircle {
    type Point = Point3;
    type Vector = Vector3;
    fn der_n(&self, n: usize, t: f64) -> Self::Vector {
        let radius = self.contact_point0.point - self.center;
        let angle = Rad(PI / 2.0) * n as f64 + self.angle * t;
        let rot = Matrix3::from_axis_angle(self.axis, angle);
        let c = self.center.to_vec() * if n == 0 { 1.0 } else { 0.0 };
        c + rot * radius * self.angle.0.powi(n as i32)
    }
    fn subs(&self, t: f64) -> Self::Point {
        Point3::from_vec(self.der_n(0, t))
    }
    fn der(&self, t: f64) -> Self::Vector {
        self.der_n(1, t)
    }
    fn der2(&self, t: f64) -> Self::Vector {
        self.der_n(2, t)
    }
    fn parameter_range(&self) -> ParameterRange {
        use std::ops::Bound;
        (Bound::Included(0.0), Bound::Included(1.0))
    }
}

impl ToSameGeometry<NurbsCurve<Vector4>> for ContactCircle {
    fn to_same_geometry(&self) -> NurbsCurve<Vector4> {
        let (sin2, cos2) = (self.angle / 2.0).sin_cos();
        let x_axis = self.contact_point0.point - self.center;
        let z_axis = self.axis;
        let y_axis = z_axis.cross(x_axis);
        let mat = Matrix4::from_cols(
            x_axis.extend(0.0),
            y_axis.extend(0.0),
            z_axis.extend(0.0),
            self.center.to_homogeneous(),
        );
        NurbsCurve::new(BSplineCurve::new_unchecked(
            KnotVec::bezier_knot(2),
            vec![
                mat * Vector4::new(1.0, 0.0, 0.0, 1.0),
                mat * Vector4::new(cos2, sin2, 0.0, cos2),
                mat * Vector4::new(self.angle.cos(), self.angle.sin(), 0.0, 1.0),
            ],
        ))
    }
}

fn contact_points(
    // point and derivation
    point_on_curve: (Point3, Vector3),
    // origin and normal
    plane0: (Point3, Vector3),
    // origin and normal
    plane1: (Point3, Vector3),
    radius: f64,
) -> (Point3, Point3, Point3) {
    let ((p, der), (p0, n0), (p1, n1)) = (point_on_curve, plane0, plane1);
    let sign = f64::signum(n0.cross(n1).dot(der));
    let mat = Matrix3::from_cols(der, n0, n1).transpose();
    let vec = Vector3::new(
        der.dot(p.to_vec()),
        n0.dot(p0.to_vec()) - sign * radius,
        n1.dot(p1.to_vec()) - sign * radius,
    );
    let center = Point3::from_vec(mat.invert().unwrap() * vec);
    let q0 = center + sign * radius * n0;
    let q1 = center + sign * radius * n1;
    (center, q0, q1)
}

fn next_point(
    surface: &impl FilletableSurface,
    (u, v): (f64, f64),
    (p, q): (Point3, Point3),
) -> (Point3, (f64, f64)) {
    let ders = surface.ders(2, u, v);
    let (uder, vder) = (ders[1][0], ders[0][1]);
    let d = q - p;
    let uu = uder.dot(uder);
    let uv = uder.dot(vder);
    let vv = vder.dot(vder);
    let mat = Matrix2::new(uu, uv, uv, vv);
    let vec = Vector2::new(uder.dot(d), vder.dot(d));
    let del = mat.invert().unwrap() * vec;
    let (u, v) = (u + del.x, v + del.y);
    (surface.subs(u, v), (u, v))
}
