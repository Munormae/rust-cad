use cryxtal_modeling::*;

fn main() {
    use cryxtal_geometry::prelude::*;
    use cryxtal_shapeops as shapeops;
    use serde_json;
    let f = BSplineSurface::new(
        KnotVec::bezier_knot(2),
        KnotVec::bezier_knot(2),
        vec![
            [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 2.0, 0.0),
            ],
            [
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.5),
                Point3::new(1.0, 2.0, 0.0),
            ],
            [
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(2.0, 1.0, 0.0),
                Point3::new(2.0, 2.0, 0.0),
            ],
        ],
    );
    let mut cube = builder::try_attach_plane(&f, 0.05).unwrap();
    let mut cylinder = builder::tsweep(&f, Vector3::unit_z() * 2.0);
    cylinder.not();
    let and = shapeops::and(&cube, &cylinder, 0.05).unwrap();
    let json = serde_json::to_vec_pretty(&and).unwrap();
    std::fs::write("punched-cube-shapeops.json", json).unwrap();
}
