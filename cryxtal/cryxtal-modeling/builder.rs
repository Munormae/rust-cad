use crate::{
    errors::Error,
    geom_impls::{self, ArcConnector, ExtrudeConnector, LineConnector, RevoluteConnector},
    topo_traits::*,
    Result,
};
use cryxtal_geometry::prelude::*;
use cryxtal_topology::*;
const PI: Rad<f64> = Rad(std::f64::consts::PI);
type Vertex = cryxtal_topology::Vertex<Point3>;
type Edge<C> = cryxtal_topology::Edge<Point3, C>;
type Wire<C> = cryxtal_topology::Wire<Point3, C>;
type Face<C, S> = cryxtal_topology::Face<Point3, C, S>;
type Shell<C, S> = cryxtal_topology::Shell<Point3, C, S>;

#[inline(always)]
/// Create a topology vertex from a point.
pub fn vertex<P: Into<Point3>>(p: P) -> Vertex {
    Vertex::new(p.into())
}

#[inline(always)]
/// Create vertices from an iterator of points.
pub fn vertices<P: Into<Point3>>(points: impl IntoIterator<Item = P>) -> Vec<Vertex> {
    points.into_iter().map(|p| Vertex::new(p.into())).collect()
}

/// Create an edge from two vertices using a line curve.
pub fn line<C>(vertex0: &Vertex, vertex1: &Vertex) -> Edge<C>
where
    Line<Point3>: ToSameGeometry<C>,
{
    let pt0 = vertex0.point();
    let pt1 = vertex1.point();
    Edge::new(vertex0, vertex1, Line(pt0, pt1).to_same_geometry())
}

/// Create a circular arc edge from two vertices and a transit point.
pub fn circle_arc<C>(vertex0: &Vertex, vertex1: &Vertex, transit: Point3) -> Edge<C>
where
    Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>: ToSameGeometry<C>,
{
    let pt0 = vertex0.point();
    let pt1 = vertex1.point();
    let curve = geom_impls::circle_arc_by_three_points(pt0, pt1, transit);
    Edge::new(vertex0, vertex1, curve.to_same_geometry())
}

/// Create a Bezier edge (BSpline of degree n) from two vertices and interior points.
pub fn bezier<C>(vertex0: &Vertex, vertex1: &Vertex, mut inter_points: Vec<Point3>) -> Edge<C>
where
    BSplineCurve<Point3>: ToSameGeometry<C>,
{
    let pt0 = vertex0.point();
    let pt1 = vertex1.point();
    let mut ctrl_pts = vec![pt0];
    ctrl_pts.append(&mut inter_points);
    ctrl_pts.push(pt1);
    let knot_vec = KnotVec::bezier_knot(ctrl_pts.len() - 1);
    let curve = BSplineCurve::new(knot_vec, ctrl_pts);
    Edge::new(vertex0, vertex1, curve.to_same_geometry())
}

/// Create a homotopy face between two edges.
pub fn homotopy<C, S>(edge0: &Edge<C>, edge1: &Edge<C>) -> Face<C, S>
where
    C: Invertible,
    Line<Point3>: ToSameGeometry<C>,
    HomotopySurface<C, C>: ToSameGeometry<S>,
{
    let wire = wire![
        edge0.clone(),
        line(edge0.back(), edge1.back()),
        edge1.inverse(),
        line(edge1.front(), edge0.front()),
    ];
    let curve0 = edge0.oriented_curve();
    let curve1 = edge1.oriented_curve();
    let homotopy = HomotopySurface::new(curve0, curve1);
    Face::new(vec![wire], homotopy.to_same_geometry())
}

/// Try to build a homotopy shell between two wires; returns an error if counts differ.
pub fn try_wire_homotopy<C, S>(wire0: &Wire<C>, wire1: &Wire<C>) -> Result<Shell<C, S>>
where
    C: Invertible,
    Line<Point3>: ToSameGeometry<C>,
    HomotopySurface<C, C>: ToSameGeometry<S>,
{
    if wire0.len() != wire1.len() {
        return Err(Error::NotSameNumberOfEdges);
    }
    use cryxtal_base::entry_map::FxEntryMap;
    let mut vemap = FxEntryMap::new(
        |(v0, v1): (&Vertex, &Vertex)| (v0.id(), v1.id()),
        |(v0, v1)| line(v0, v1),
    );
    let shell = wire0
        .edge_iter()
        .zip(wire1.edge_iter())
        .map(|(edge0, edge1)| {
            let (v0, v1) = (edge0.front(), edge1.front());
            let edge2 = vemap.entry_or_insert((v0, v1)).inverse();
            let (v0, v1) = (edge0.back(), edge1.back());
            let edge3 = vemap.entry_or_insert((v0, v1)).clone();
            let wire = wire![edge0.clone(), edge3, edge1.inverse(), edge2];
            let curve0 = edge0.oriented_curve();
            let curve1 = edge1.oriented_curve();
            let homotopy = HomotopySurface::new(curve0, curve1);
            Face::new(vec![wire], homotopy.to_same_geometry())
        })
        .collect();
    Ok(shell)
}

/// Try to attach a plane to wires and produce a face if they are coplanar.
pub fn try_attach_plane<C, S>(wires: impl Into<Vec<Wire<C>>>) -> Result<Face<C, S>>
where
    C: ParametricCurve3D + BoundedCurve,
    Plane: IncludeCurve<C> + ToSameGeometry<S>,
{
    let wires = wires.into();
    let _ = Face::try_new(wires.clone(), ())?;
    let pts = wires
        .iter()
        .map(|wire| {
            wire.edge_iter()
                .flat_map(|edge| {
                    let p0 = edge.front().point();
                    let curve = edge.curve();
                    let (t0, t1) = curve.range_tuple();
                    let p1 = curve.subs((t0 + t1) / 2.0);
                    [p0, p1]
                })
                .collect()
        })
        .collect::<Vec<_>>();

    let plane = match geom_impls::attach_plane(pts) {
        Some(got) => got,
        None => return Err(Error::WireNotInOnePlane),
    };
    Ok(Face::new_unchecked(wires, plane.to_same_geometry()))
}

#[inline(always)]
/// Clone via Mapped<()> implementation.
pub fn clone<T: Mapped<()>>(elem: &T) -> T {
    elem.mapped(())
}

#[inline(always)]
/// Transform element by 4x4 matrix via Mapped<Matrix4>.
pub fn transformed<T: Mapped<Matrix4>>(elem: &T, mat: Matrix4) -> T {
    elem.mapped(mat)
}

#[inline(always)]
/// Translate element by vector.
pub fn translated<T: Mapped<Matrix4>>(elem: &T, vector: Vector3) -> T {
    transformed(elem, Matrix4::from_translation(vector))
}

/// Rotate element around axis through origin by angle.
pub fn rotated<T: Mapped<Matrix4>>(elem: &T, origin: Point3, axis: Vector3, angle: Rad<f64>) -> T {
    let mat0 = Matrix4::from_translation(-origin.to_vec());
    let mat1 = Matrix4::from_axis_angle(axis, angle);
    let mat2 = Matrix4::from_translation(origin.to_vec());
    transformed(elem, mat2 * mat1 * mat0)
}

/// Non-uniformly scale element about origin.
pub fn scaled<T: Mapped<Matrix4>>(elem: &T, origin: Point3, scalars: Vector3) -> T {
    let mat0 = Matrix4::from_translation(-origin.to_vec());
    let mat1 = Matrix4::from_nonuniform_scale(scalars[0], scalars[1], scalars[2]);
    let mat2 = Matrix4::from_translation(origin.to_vec());
    transformed(elem, mat2 * mat1 * mat0)
}

/// Extrude element along vector.
pub fn tsweep<T, Swept>(elem: &T, vector: Vector3) -> Swept
where
    T: Sweep<Matrix4, LineConnector, ExtrudeConnector, Swept>,
{
    let trsl = Matrix4::from_translation(vector);
    elem.sweep(trsl, LineConnector, ExtrudeConnector { vector })
}

/// Revolve element around axis.
pub fn rsweep<T, Swept, R>(elem: &T, origin: Point3, axis: Vector3, angle: R) -> Swept
where
    T: ClosedSweep<Matrix4, ArcConnector, RevoluteConnector, Swept>,
    R: Into<Rad<f64>>,
{
    debug_assert!(axis.magnitude().near(&1.0));
    let angle = angle.into();
    let sign = f64::signum(angle.0);
    if angle.0.abs() >= 2.0 * PI.0 {
        whole_rsweep(elem, origin, sign * axis)
    } else {
        partial_rsweep(elem, origin, sign * axis, angle * sign)
    }
}

fn partial_rsweep<T: MultiSweep<Matrix4, ArcConnector, RevoluteConnector, Swept>, Swept>(
    elem: &T,
    origin: Point3,
    axis: Vector3,
    angle: Rad<f64>,
) -> Swept {
    let division = if angle.0.abs() < PI.0 { 2 } else { 3 };
    let mat0 = Matrix4::from_translation(-origin.to_vec());
    let mat1 = Matrix4::from_axis_angle(axis, angle / division as f64);
    let mat2 = Matrix4::from_translation(origin.to_vec());
    let trsl = mat2 * mat1 * mat0;
    elem.multi_sweep(
        trsl,
        ArcConnector {
            origin,
            axis,
            angle: angle / division as f64,
        },
        RevoluteConnector { origin, axis },
        division,
    )
}

fn whole_rsweep<T: ClosedSweep<Matrix4, ArcConnector, RevoluteConnector, Swept>, Swept>(
    elem: &T,
    origin: Point3,
    axis: Vector3,
) -> Swept {
    const DIVISION: usize = 3;
    let mat0 = Matrix4::from_translation(-origin.to_vec());
    let mat1 = Matrix4::from_axis_angle(axis, PI * 2.0 / DIVISION as f64);
    let mat2 = Matrix4::from_translation(origin.to_vec());
    let trsl = mat2 * mat1 * mat0;
    elem.closed_sweep(
        trsl,
        ArcConnector {
            origin,
            axis,
            angle: PI * 2.0 / DIVISION as f64,
        },
        RevoluteConnector { origin, axis },
        DIVISION,
    )
}

pub fn cone<C, S, R>(wire: &Wire<C>, axis: Vector3, angle: R) -> Shell<C, S>
where
    C: ParametricCurve3D + BoundedCurve + Cut + Invertible + Transformed<Matrix4>,
    S: Invertible,
    R: Into<Rad<f64>>,
    Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>: ToSameGeometry<C>,
    RevolutedCurve<C>: ToSameGeometry<S>,
{
    let angle = angle.into();
    let closed = angle.0.abs() >= 2.0 * PI.0;
    let mut wire = wire.clone();
    if wire.is_empty() {
        return Shell::new();
    }
    let pt0 = wire.front_vertex().unwrap().point();
    let pt1 = wire.back_vertex().unwrap().point();
    let pt1_on_axis = (pt1 - pt0).cross(axis).so_small();
    if wire.len() == 1 && pt1_on_axis {
        let edge = wire.pop_back().unwrap();
        let v0 = edge.front().clone();
        let v2 = edge.back().clone();
        let mut curve = edge.curve();
        let (t0, t1) = curve.range_tuple();
        let t = (t0 + t1) * 0.5;
        let v1 = Vertex::new(curve.subs(t));
        let curve1 = curve.cut(t);
        wire.push_back(Edge::debug_new(&v0, &v1, curve));
        wire.push_back(Edge::debug_new(&v1, &v2, curve1));
    }
    let mut shell = rsweep(&wire, pt0, axis, angle);
    let mut edge = shell[0].boundaries()[0][0].clone();
    for i in 0..shell.len() / wire.len() {
        let idx = i * wire.len();
        let face = shell[idx].clone();
        let surface = face.oriented_surface();
        let old_wire = face.into_boundaries().pop().unwrap();
        let mut new_wire = Wire::new();
        new_wire.push_back(edge.clone());
        new_wire.push_back(old_wire[1].clone());
        let new_edge = if closed && i + 1 == shell.len() / wire.len() {
            shell[0].boundaries()[0][0].inverse()
        } else {
            let curve = old_wire[2].oriented_curve();
            Edge::debug_new(old_wire[2].front(), new_wire[0].front(), curve)
        };
        new_wire.push_back(new_edge.clone());
        shell[idx] = Face::debug_new(vec![new_wire], surface);
        edge = new_edge.inverse();
    }
    if pt1_on_axis {
        let mut edge = shell[wire.len() - 1].boundaries()[0][0].clone();
        for i in 0..shell.len() / wire.len() {
            let idx = (i + 1) * wire.len() - 1;
            let face = shell[idx].clone();
            let surface = face.oriented_surface();
            let old_wire = face.into_boundaries().pop().unwrap();
            let mut new_wire = Wire::new();
            new_wire.push_back(edge.clone());
            let new_edge = if closed && i + 1 == shell.len() / wire.len() {
                shell[wire.len() - 1].boundaries()[0][0].inverse()
            } else {
                let curve = old_wire[2].oriented_curve();
                Edge::debug_new(new_wire[0].back(), old_wire[2].back(), curve)
            };
            new_wire.push_back(new_edge.clone());
            new_wire.push_back(old_wire[3].clone());
            shell[idx] = Face::debug_new(vec![new_wire], surface);
            edge = new_edge.inverse();
        }
    }
    shell
}

#[cfg(test)]
mod partial_torus {
    use super::{Face as TFace, Shell as TShell};
    use crate::*;
    use cryxtal_base::cgmath64::cgmath::InnerSpace;
    use cryxtal_base::tolerance::TOLERANCE;
    use cryxtal_geometry::prelude::{Point3, Rad, Vector3};
    use cryxtal_geotrait::{
        BoundedCurve, BoundedSurface, ParametricCurve, ParametricSurface, ParametricSurface3D,
        SearchParameter,
    };
    use cryxtal_topology::shell;
    fn test_surface_orientation(surface: &Surface, sign: f64) {
        let rev = match surface {
            Surface::Plane(_) => return,
            Surface::RevolutedCurve(rev) => rev,
            _ => panic!(),
        };
        let ((u0, u1), (v0, v1)) = rev.range_tuple();
        let (u, v) = ((u0 + u1) / 2.0, (v0 + v1) / 2.0);
        let p = surface.subs(u, v);
        let vecp = Vector3::new(p.x, p.y, 0.0).normalize() * 0.75;
        let q = Point3::new(vecp.x, vecp.y, vecp.z);
        let n0 = (p - q).normalize() * sign;
        let n1 = surface.normal(u, v);
        assert!((n0 - n1).magnitude() < TOLERANCE)
    }

    fn test_boundary_orientation(face: &TFace<Curve, Surface>) {
        let surface = face.oriented_surface();
        let boundary = face.boundaries().pop().unwrap();
        let vec = boundary
            .iter()
            .flat_map(|edge| {
                let curve = edge.oriented_curve();
                let (t0, t1) = curve.range_tuple();
                [curve.subs(t0), curve.subs((t0 + t1) / 2.0), curve.subs(t1)]
            })
            .map(|p| surface.search_parameter(p, None, 100).unwrap())
            .collect::<Vec<_>>();
        let area = vec.windows(2).fold(0.0, |sum, v| {
            let ((u0, v0), (u1, v1)) = (v[0], v[1]);
            sum + (u0 + u1) * (v1 - v0)
        });
        assert!(area > 0.0)
    }

    fn test_shell(shell: &TShell<Curve, Surface>, sign: f64) {
        shell.iter().for_each(|face| {
            test_boundary_orientation(face);
            test_surface_orientation(&face.oriented_surface(), sign);
        })
    }

    #[test]
    fn partial_torus() {
        let v = builder::vertex(Point3::new(0.5, 0.0, 0.0));
        let w = builder::rsweep(&v, Point3::new(0.75, 0.0, 0.0), Vector3::unit_y(), Rad(7.0));
        let face = builder::try_attach_plane(&[w]).unwrap();
        test_shell(&shell![face.clone()], 1.0);
        let torus = builder::rsweep(
            &face,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            Rad(2.0),
        );
        test_shell(&torus.boundaries()[0], 1.0);
        assert!(torus.is_geometric_consistent());
        let torus = builder::rsweep(
            &face,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            Rad(5.0),
        );
        test_shell(&torus.boundaries()[0], 1.0);
        assert!(torus.is_geometric_consistent());
        let torus = builder::rsweep(
            &face,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            Rad(-2.0),
        );
        test_shell(&torus.boundaries()[0], -1.0);
        assert!(torus.is_geometric_consistent());
        let torus = builder::rsweep(
            &face,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            Rad(-5.0),
        );
        test_shell(&torus.boundaries()[0], -1.0);
        assert!(torus.is_geometric_consistent());
    }
}
