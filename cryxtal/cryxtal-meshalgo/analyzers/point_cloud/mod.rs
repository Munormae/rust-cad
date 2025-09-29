use super::*;
mod hashed_point_cloud;
use hashed_point_cloud::HashedPointCloud;
mod sort_end_points;

/// Proximity queries between meshes and point clouds.
pub trait WithPointCloud {
    /// Checks if any point in the cloud is within tol of the mesh.
    fn is_clung_to_by(&self, point_cloud: &[Point3], tol: f64) -> bool;
    /// Checks if the neighborhood around the cloud is included by the mesh.
    fn neighborhood_include(&self, point_cloud: &[Point3], tol: f64) -> bool;
    /// Checks if the mesh collides with the neighborhood of the cloud.
    fn collide_with_neighborhood_of(&self, point_cloud: &[Point3], tol: f64) -> bool;
}

impl WithPointCloud for PolygonMesh {
    #[inline(always)]
    fn is_clung_to_by(&self, point_cloud: &[Point3], tol: f64) -> bool {
        nonpositive_tolerance!(tol);
        HashedPointCloud::from_points(point_cloud, tol * 2.0).distance2(self) < tol * tol
    }
    #[inline(always)]
    fn collide_with_neighborhood_of(&self, point_cloud: &[Point3], tol: f64) -> bool {
        HashedPointCloud::from_points(point_cloud, tol * 2.0).is_colliding(self, tol)
    }
    #[inline(always)]
    fn neighborhood_include(&self, point_cloud: &[Point3], tol: f64) -> bool {
        sort_end_points::pointcloud_in_polygon_neighborhood(self, point_cloud, tol)
    }
}

fn distance2_point_triangle(point: Point3, triangle: [Point3; 3]) -> f64 {
    let ab = triangle[1] - triangle[0];
    let ap = point - triangle[0];
    let bc = triangle[2] - triangle[1];
    let bp = point - triangle[1];
    let ca = triangle[0] - triangle[2];
    let cp = point - triangle[2];
    let nor = ab.cross(ca);

    let coef = f64::signum(ab.cross(nor).dot(ap))
        + f64::signum(bc.cross(nor).dot(bp))
        + f64::signum(ca.cross(nor).dot(cp));
    if coef < 2.0 || nor.magnitude().so_small() {
        let a = (ap - ab * f64::clamp(ab.dot(ap) / ab.dot(ab), 0.0, 1.0)).magnitude2();
        let b = (bp - bc * f64::clamp(bc.dot(bp) / bc.dot(bc), 0.0, 1.0)).magnitude2();
        let c = (cp - ca * f64::clamp(ca.dot(cp) / ca.dot(ca), 0.0, 1.0)).magnitude2();
        f64::min(f64::min(a, b), c)
    } else {
        nor.dot(ap) * nor.dot(ap) / nor.magnitude2()
    }
}
