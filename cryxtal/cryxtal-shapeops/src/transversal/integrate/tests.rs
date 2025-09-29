use cryxtal_meshalgo::tessellation::{MeshableShape, MeshedShape};
use cryxtal_modeling::*;
use cryxtal_geometry::prelude::{Point3, Vector3, Rad, EuclideanSpace};
use cryxtal_topology::Solid;
use cryxtal_polymesh::obj;

#[test]
#[cfg(feature = "shapeops-tests")]
fn punched_cube() {
    let v = builder::vertex(Point3::new(0.0, 0.0, 0.0));
    let e = builder::tsweep(&v, Vector3::unit_x());
    let f = builder::tsweep(&e, Vector3::unit_y());
    let cube: Solid<Point3, _, _> = builder::tsweep(&f, Vector3::unit_z());

    let v = builder::vertex(Point3::new(0.5, 0.25, -0.5));
    let w = builder::rsweep(&v, Point3::new(0.5, 0.5, 0.0), Vector3::unit_z(), Rad(7.0));
    let f = builder::try_attach_plane(&[w]).unwrap();
    let mut cylinder = builder::tsweep(&f, Vector3::unit_z() * 2.0);
    cylinder.not();
    let and = crate::and(&cube, &cylinder, 0.05).unwrap();

    let poly = and.triangulation(0.01).to_polygon();
    let file = std::fs::File::create("punched-cube.obj").unwrap();
    obj::write(&poly, file).unwrap();
}
