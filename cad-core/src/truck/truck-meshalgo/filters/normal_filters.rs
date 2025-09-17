use super::*;
use rustc_hash::FxHashMap as HashMap;

pub trait NormalFilters {
    fn normalize_normals(&mut self) -> &mut Self;
    fn add_naive_normals(&mut self, overwrite: bool) -> &mut Self;
    fn add_smooth_normals(&mut self, tol_ang: f64, overwrite: bool) -> &mut Self;
    fn make_face_compatible_to_normal(&mut self) -> &mut Self;
    fn make_normal_compatible_to_face(&mut self) -> &mut Self;
}

impl NormalFilters for PolygonMesh {
    fn normalize_normals(&mut self) -> &mut Self {
        let mut mesh = self.debug_editor();
        let PolygonMeshEditor {
            attributes: StandardAttributes { normals, .. },
            faces,
            ..
        } = &mut mesh;
        normals
            .iter_mut()
            .for_each(move |normal| *normal = normal.normalize());
        faces.face_iter_mut().flatten().for_each(|v| {
            if let Some(idx) = v.nor {
                if !normals[idx].magnitude2().near(&1.0) {
                    v.nor = None;
                }
            }
        });
        drop(mesh);
        self
    }
    fn make_face_compatible_to_normal(&mut self) -> &mut Self {
        let mut mesh = self.debug_editor();
        let PolygonMeshEditor {
            attributes: StandardAttributes {
                positions, normals, ..
            },
            faces,
            ..
        } = &mut mesh;
        for face in faces.face_iter_mut() {
            let normal = face.iter().fold(Vector3::zero(), |normal, v| {
                normal + v.nor.map(|i| normals[i]).unwrap_or_else(Vector3::zero)
            });
            let face_normal = FaceNormal::new(positions, face, 0).normal;
            if normal.dot(face_normal) < 0.0 {
                face.reverse();
            }
        }
        drop(mesh);
        self
    }
    fn make_normal_compatible_to_face(&mut self) -> &mut Self {
        let mut mesh = self.debug_editor();
        let PolygonMeshEditor {
            attributes: StandardAttributes {
                positions, normals, ..
            },
            faces,
            ..
        } = &mut mesh;
        for face in faces.face_iter_mut() {
            let face_normal = FaceNormal::new(positions, face, 0).normal;
            face.iter_mut().for_each(|v| {
                v.nor = v.nor.map(|idx| {
                    if normals[idx].dot(face_normal) < 0.0 {
                        normals.push(-normals[idx]);
                        normals.len() - 1
                    } else {
                        idx
                    }
                });
            })
        }
        drop(mesh);
        self
    }
    fn add_naive_normals(&mut self, overwrite: bool) -> &mut Self {
        let mut mesh = self.debug_editor();
        let PolygonMeshEditor {
            attributes: StandardAttributes {
                positions, normals, ..
            },
            faces,
            ..
        } = &mut mesh;
        if overwrite {
            normals.clear()
        }
        faces.face_iter_mut().for_each(move |face| {
            let normal = FaceNormal::new(positions, face, 0).normal;
            let mut added = false;
            face.iter_mut().for_each(|v| {
                if v.nor.is_none() || overwrite {
                    if !added {
                        normals.push(normal);
                        added = true;
                    }
                    v.nor = Some(normals.len() - 1);
                }
            });
        });
        drop(mesh);
        self
    }
    fn add_smooth_normals(&mut self, tol_ang: f64, overwrite: bool) -> &mut Self {
        let vnmap = self.clustering_normal_faces(tol_ang.cos());
        self.reflect_normal_clusters(vnmap, overwrite);
        self
    }
}

trait SubNormalFilter {
    fn clustering_normal_faces(&self, inf: f64) -> HashMap<usize, Vec<Vec<FaceNormal>>>;
    fn reflect_normal_clusters(
        &mut self,
        vnmap: HashMap<usize, Vec<Vec<FaceNormal>>>,
        overwrite: bool,
    );
}

impl SubNormalFilter for PolygonMesh {
    fn clustering_normal_faces(&self, inf: f64) -> HashMap<usize, Vec<Vec<FaceNormal>>> {
        let positions = self.positions();
        let mut vnmap = HashMap::default();
        self.face_iter()
            .enumerate()
            .for_each(|(i, face)| add_face_normal(positions, i, face, &mut vnmap, inf));
        vnmap
    }

    fn reflect_normal_clusters(
        &mut self,
        vnmap: HashMap<usize, Vec<Vec<FaceNormal>>>,
        overwrite: bool,
    ) {
        let mut mesh = self.debug_editor();
        let PolygonMeshEditor {
            attributes: StandardAttributes { normals, .. },
            faces,
            ..
        } = &mut mesh;
        if overwrite {
            normals.clear();
        }
        for (pos_id, vecs) in vnmap.into_iter() {
            for vec in vecs {
                let normal = vec
                    .iter()
                    .fold(Vector3::zero(), |sum, x| sum + x.normal)
                    .normalize();
                for FaceNormal { face_id, .. } in vec {
                    signup_vertex_normal(pos_id, face_id, normals, normal, faces, overwrite);
                }
            }
        }
    }
}

fn add_face_normal(
    positions: &[Point3],
    face_id: usize,
    face: &[Vertex],
    vnmap: &mut HashMap<usize, Vec<Vec<FaceNormal>>>,
    inf: f64,
) {
    let face_normal = FaceNormal::new(positions, face, face_id);
    face.iter().for_each(|v| {
        add_to_vnmap(v.pos, face_normal, vnmap, inf);
    })
}

fn add_to_vnmap(
    pos_id: usize,
    face_normal: FaceNormal,
    vnmap: &mut HashMap<usize, Vec<Vec<FaceNormal>>>,
    inf: f64,
) {
    match vnmap.get_mut(&pos_id) {
        Some(vecs) => {
            for vec in vecs.iter_mut() {
                let normal = vec
                    .iter()
                    .fold(Vector3::zero(), |sum, x| sum + x.normal)
                    .normalize();
                if face_normal.normal.dot(normal) > inf {
                    vec.push(face_normal);
                    return;
                }
            }
            vecs.push(vec![face_normal]);
        }
        None => {
            let vecs = vec![vec![face_normal]];
            vnmap.insert(pos_id, vecs);
        }
    }
}

fn signup_vertex_normal(
    pos_id: usize,
    face_id: usize,
    normals: &mut Vec<Vector3>,
    normal: Vector3,
    faces: &mut Faces,
    overwrite: bool,
) {
    let face = faces[face_id].as_mut();
    let j = (0..face.len()).find(|j| face[*j].pos == pos_id).unwrap();
    if face[j].nor.is_none() || overwrite {
        if let Some(n) = normals.last() {
            if n != &normal {
                normals.push(normal);
            }
        } else {
            normals.push(normal);
        }
        face[j].nor = Some(normals.len() - 1);
    }
}
