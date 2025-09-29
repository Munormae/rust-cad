use super::*;
use std::f64::consts::PI;

/// Mesh splitting utilities (components, planar extraction, face selection).
pub trait Splitting {
    /// Creates a sub-mesh from selected face indices.
    fn create_mesh_by_face_indices(&self, indices: &[usize]) -> PolygonMesh;
    /// Splits faces into planar/other clusters with tolerance.
    fn extract_planes(&self, tol: f64) -> (Vec<usize>, Vec<usize>);
    /// Connected components by face adjacency.
    fn components(&self, use_normal: bool) -> Vec<Vec<usize>>;
}

impl Splitting for PolygonMesh {
    fn create_mesh_by_face_indices(&self, indices: &[usize]) -> PolygonMesh {
        let positions = self.positions().clone();
        let uv_coords = self.uv_coords().clone();
        let normals = self.normals().clone();
        let faces: Faces = indices.iter().map(|i| &self.faces()[*i]).collect();
        PolygonMesh::new(
            StandardAttributes {
                positions,
                uv_coords,
                normals,
            },
            faces,
        )
    }

    fn extract_planes(&self, tol: f64) -> (Vec<usize>, Vec<usize>) {
        nonpositive_tolerance!(tol, 0.0);
        self.faces_into_two_clusters(|face: &[Vertex]| {
            is_in_the_plane(self.positions(), self.normals(), face, tol * tol)
        })
    }

    fn components(&self, use_normal: bool) -> Vec<Vec<usize>> {
        let face_adjacency = self.faces().face_adjacency(use_normal);
        get_components(&face_adjacency)
    }
}

#[doc(hidden)]
pub trait ExperimentalSplitters {
    fn faces_into_two_clusters<F: Fn(&[Vertex]) -> bool>(
        &self,
        func: F,
    ) -> (Vec<usize>, Vec<usize>);
    fn clustering_faces_by_gcurvature(
        &self,
        threshold: f64,
        preferred_upper: bool,
    ) -> (Vec<usize>, Vec<usize>);
    fn get_gcurve(&self) -> Vec<f64>;
}

impl ExperimentalSplitters for PolygonMesh {
    fn faces_into_two_clusters<F: Fn(&[Vertex]) -> bool>(
        &self,
        func: F,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut true_faces = Vec::new();
        let mut false_faces = Vec::new();
        for (i, face) in self.face_iter().enumerate() {
            match func(face) {
                true => true_faces.push(i),
                false => false_faces.push(i),
            }
        }
        (true_faces, false_faces)
    }

    fn clustering_faces_by_gcurvature(
        &self,
        threshold: f64,
        preferred_upper: bool,
    ) -> (Vec<usize>, Vec<usize>) {
        let gcurve = self.get_gcurve();
        self.faces_into_two_clusters(|face: &[Vertex]| {
            is_signed_up_upper(face, &gcurve, preferred_upper, threshold)
        })
    }

    fn get_gcurve(&self) -> Vec<f64> {
        let positions = self.positions();
        let mut angles = vec![0.0; positions.len()];
        let mut weights = vec![0.0; positions.len()];
        for face in self.tri_faces() {
            angles[face[0].pos] += get_angle(positions, face, 0, 1, 2);
            angles[face[1].pos] += get_angle(positions, face, 1, 2, 0);
            angles[face[2].pos] += get_angle(positions, face, 2, 0, 1);
            add_weights(&mut weights, positions, face);
        }
        for face in self.quad_faces() {
            angles[face[0].pos] += get_angle(positions, face, 0, 1, 3);
            angles[face[1].pos] += get_angle(positions, face, 1, 2, 0);
            angles[face[2].pos] += get_angle(positions, face, 2, 3, 1);
            angles[face[3].pos] += get_angle(positions, face, 3, 0, 1);
            add_weights(&mut weights, positions, face);
        }
        for face in self.other_faces() {
            let n = face.len() - 1;
            angles[face[0].pos] += get_angle(positions, face, 0, 1, n);
            for i in 1..n {
                angles[face[i].pos] += get_angle(positions, face, i, i + 1, i - 1);
            }
            angles[face[n].pos] += get_angle(positions, face, n, 0, n - 1);
            add_weights(&mut weights, positions, face);
        }

        angles
            .into_iter()
            .zip(weights)
            .map(|(ang, weight)| (PI * 2.0 - ang) / weight)
            .collect()
    }
}

fn get_components(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut unchecked = vec![true; adjacency.len()];
    let mut components = Vec::new();
    loop {
        let first = match unchecked.iter().position(|x| *x) {
            Some(idx) => idx,
            None => return components,
        };
        let mut stack = vec![first];
        let mut component = vec![first];
        unchecked[first] = false;
        while let Some(cursor) = stack.pop() {
            for i in &adjacency[cursor] {
                if unchecked[*i] {
                    unchecked[*i] = false;
                    component.push(*i);
                    stack.push(*i);
                }
            }
        }
        components.push(component);
    }
}

fn is_in_the_plane(positions: &[Point3], normals: &[Vector3], face: &[Vertex], tol2: f64) -> bool {
    let n = FaceNormal::new(positions, face, 0).normal;
    for v in face {
        if let Some(nor) = v.nor {
            if n.distance2(normals[nor]) < tol2 {
                return true;
            }
        }
    }
    false
}

fn is_signed_up_upper(
    face: &[Vertex],
    gcurve: &[f64],
    preferred_upper: bool,
    threshold: f64,
) -> bool {
    if preferred_upper {
        face.as_ref().iter().any(|v| gcurve[v.pos] > threshold)
    } else {
        face.as_ref().iter().all(|v| gcurve[v.pos] > threshold)
    }
}

fn get_angle(positions: &[Point3], face: &[Vertex], idx0: usize, idx1: usize, idx2: usize) -> f64 {
    let vec0 = positions[face[idx1].pos] - positions[face[idx0].pos];
    let vec1 = positions[face[idx2].pos] - positions[face[idx0].pos];
    vec0.angle(vec1).0
}

fn add_weights(weights: &mut [f64], positions: &[Point3], face: &[Vertex]) {
    let area = (2..face.len()).fold(0.0, |sum, i| {
        let vec0 = positions[face[i - 1].pos] - positions[face[0].pos];
        let vec1 = positions[face[i].pos] - positions[face[0].pos];
        sum + (vec0.cross(vec1)).magnitude() / 2.0
    }) / (face.len() as f64);
    for v in face {
        weights[v.pos] += area;
    }
}

#[test]
fn into_components_test() {
    let faces = vec![
        [(0, None, Some(0)), (1, None, Some(1)), (2, None, Some(2))].as_ref(),
        &[(0, None, Some(0)), (5, None, Some(5)), (1, None, Some(1))],
        &[(0, None, Some(7)), (5, None, Some(5)), (6, None, Some(6))],
        &[(5, None, Some(5)), (6, None, Some(6)), (7, None, Some(7))],
        &[
            (1, None, Some(1)),
            (4, None, Some(4)),
            (3, None, Some(3)),
            (2, None, Some(2)),
        ],
    ]
    .into_iter()
    .collect();
    let positions = vec![Point3::origin(); 8];
    let normals = vec![Vector3::unit_x(); 8];
    let mesh = PolygonMesh::new(
        StandardAttributes {
            positions,
            normals,
            ..Default::default()
        },
        faces,
    );
    let comp = mesh.components(true);
    assert_eq!(comp.len(), 2);
    assert_eq!(comp[0], vec![0, 1, 4]);
    assert_eq!(comp[1], vec![2, 3]);
}
