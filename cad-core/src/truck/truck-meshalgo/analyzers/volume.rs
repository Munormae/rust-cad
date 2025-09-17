use super::*;
use array_macro::array;

pub trait CalcVolume {
    fn volume(&self) -> f64;
    fn center_of_gravity(&self) -> Vector4;
}

impl CalcVolume for PolygonMesh {
    fn volume(&self) -> f64 {
        point_triangles(self).fold(0.0, |sum, [p, q, r]| {
            sum + (p.x + q.x + r.x) * ((q.y - p.y) * (r.z - p.z) - (r.y - p.y) * (q.z - p.z))
        }) / 6.0
    }
    fn center_of_gravity(&self) -> Vector4 {
        let arr = point_triangles(self).fold([0.0; 4], |sum, [p, q, r]| {
            let det = array![i => {
                let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                (q[j] - p[j]) * (r[k] - p[k]) - (r[j] - p[j]) * (q[k] - p[k])
            }; 3];
            let vals = array![i => {
                let s = p[i] + q[i] + r[i];
                s * s - p[i] * q[i] - q[i] * r[i] - r[i] * p[i]
            }; 3];
            let res = array![i => sum[i] + vals[i] * det[i]; 3];
            let res3 = sum[3] + (p.x + q.x + r.x) * det[0];
            [res[0], res[1], res[2], res3]
        });
        Vector4::new(arr[0] / 24.0, arr[1] / 24.0, arr[2] / 24.0, arr[3] / 6.0)
    }
}

fn point_triangles(poly: &PolygonMesh) -> impl Iterator<Item = [Point3; 3]> + '_ {
    poly.faces()
        .triangle_iter()
        .map(|faces| array![i => poly.positions()[faces[i].pos]; 3])
}

impl CalcVolume for truck_topology::Solid<Point3, PolylineCurve<Point3>, PolygonMesh> {
    fn volume(&self) -> f64 {
        self.face_iter()
            .map(|face| match face.orientation() {
                true => face.surface().volume(),
                false => -face.surface().volume(),
            })
            .sum::<f64>()
    }
    fn center_of_gravity(&self) -> Vector4 {
        self.face_iter()
            .map(|face| match face.orientation() {
                true => face.surface().center_of_gravity(),
                false => -face.surface().center_of_gravity(),
            })
            .sum::<Vector4>()
    }
}
