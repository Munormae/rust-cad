use crate::builder;
use cryxtal_geometry::prelude::*;
use cryxtal_topology::*;
use std::f64::consts::PI;

pub fn rect<C>(r#box: BoundingBox<Point2>, plane: Plane) -> Wire<Point3, C>
where
    Line<Point3>: ToSameGeometry<C>,
{
    let (min, max) = (r#box.min(), r#box.max());
    let v = builder::vertices([
        plane.subs(min.x, min.y),
        plane.subs(max.x, min.y),
        plane.subs(max.x, max.y),
        plane.subs(min.x, max.y),
    ]);
    wire![
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
        builder::line(&v[3], &v[0]),
    ]
}

pub fn circle<C>(start: Point3, origin: Point3, axis: Vector3, division: usize) -> Wire<Point3, C>
where
    Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>: ToSameGeometry<C>,
{
    let origin = origin + (start - origin).dot(axis) * axis;
    let radius = start - origin;
    let y = axis.cross(radius);
    let mat = Matrix4::from_cols(
        radius.extend(0.0),
        y.extend(0.0),
        axis.extend(0.0),
        origin.to_homogeneous(),
    );

    let make_vertices = move |i: usize| {
        let t = 2.0 * PI * i as f64 / division as f64;
        let p = Point3::new(f64::cos(t), f64::sin(t), 0.0);
        Vertex::new(mat.transform_point(p))
    };
    let v = (0..division).map(make_vertices).collect::<Vec<_>>();

    let make_edges = move |i: usize| {
        let t0 = 2.0 * PI * i as f64 / division as f64;
        let t1 = 2.0 * PI * (i + 1) as f64 / division as f64;
        let unit_circle = UnitCircle::new();
        let trimmed = TrimmedCurve::new(unit_circle, (t0, t1));
        let mut arc = Processor::new(trimmed);
        arc.transform_by(mat);
        Edge::new(&v[i], &v[(i + 1) % division], arc.to_same_geometry())
    };
    (0..division).map(make_edges).collect()
}

pub fn cuboid<C, S>(r#box: BoundingBox<Point3>) -> Solid<Point3, C, S>
where
    Line<Point3>: ToSameGeometry<C>,
    Plane: ToSameGeometry<S>,
{
    let (p, q) = (r#box.min(), r#box.max());
    let v = builder::vertices([
        (p.x, p.y, p.z),
        (q.x, p.y, p.z),
        (q.x, q.y, p.z),
        (p.x, q.y, p.z),
        (p.x, p.y, q.z),
        (q.x, p.y, q.z),
        (q.x, q.y, q.z),
        (p.x, q.y, q.z),
    ]);
    let e = [
        builder::line(&v[0], &v[1]),
        builder::line(&v[1], &v[2]),
        builder::line(&v[2], &v[3]),
        builder::line(&v[3], &v[0]),
        builder::line(&v[0], &v[4]),
        builder::line(&v[1], &v[5]),
        builder::line(&v[2], &v[6]),
        builder::line(&v[3], &v[7]),
        builder::line(&v[4], &v[5]),
        builder::line(&v[5], &v[6]),
        builder::line(&v[6], &v[7]),
        builder::line(&v[7], &v[4]),
    ];

    let wire0 = wire![
        e[3].inverse(),
        e[2].inverse(),
        e[1].inverse(),
        e[0].inverse(),
    ];
    let plane0 = Plane::new(v[0].point(), v[3].point(), v[1].point());
    let mut shell = shell![Face::new(vec![wire0], plane0.to_same_geometry())];

    (0..4).for_each(|i| {
        let wirei = wire![
            e[i].clone(),
            e[(i + 1) % 4 + 4].clone(),
            e[i + 8].inverse(),
            e[i + 4].inverse(),
        ];
        let planei = Plane::new(v[i].point(), v[i + 1].point(), v[i + 4].point());
        shell.push(Face::new(vec![wirei], planei.to_same_geometry()));
    });

    let wire5 = wire![e[8].clone(), e[9].clone(), e[10].clone(), e[11].clone(),];
    let plane5 = Plane::new(v[4].point(), v[5].point(), v[7].point());
    shell.push(Face::new(vec![wire5], plane5.to_same_geometry()));

    Solid::new(vec![shell])
}
