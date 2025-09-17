use crate::errors::Error;
use crate::*;
use std::fmt::Debug;

impl<V: Copy + Debug, A: Attributes<V>> PolygonMesh<V, A> {
    pub fn new(attributes: A, faces: Faces<V>) -> Self {
        Self::try_new(attributes, faces).unwrap_or_else(|e| panic!("{e:?}"))
    }

    pub fn try_new(attributes: A, faces: Faces<V>) -> Result<Self, Error<V>> {
        faces
            .is_compatible(&attributes)
            .map(|_| Self::new_unchecked(attributes, faces))
    }

    #[inline(always)]
    pub const fn new_unchecked(attributes: A, faces: Faces<V>) -> Self {
        Self { attributes, faces }
    }

    #[inline(always)]
    pub fn debug_new(attributes: A, faces: Faces<V>) -> Self {
        match cfg!(debug_assertions) {
            true => Self::new(attributes, faces),
            false => Self::new_unchecked(attributes, faces),
        }
    }

    #[inline(always)]
    pub const fn attributes(&self) -> &A {
        &self.attributes
    }

    #[inline(always)]
    pub const fn faces(&self) -> &Faces<V> {
        &self.faces
    }

    #[inline(always)]
    pub const fn tri_faces(&self) -> &Vec<[V; 3]> {
        &self.faces.tri_faces
    }

    #[inline(always)]
    pub const fn quad_faces(&self) -> &Vec<[V; 4]> {
        &self.faces.quad_faces
    }

    #[inline(always)]
    pub const fn other_faces(&self) -> &Vec<Vec<V>> {
        &self.faces.other_faces
    }

    #[inline(always)]
    pub fn face_iter(&self) -> impl Iterator<Item = &[V]> {
        self.faces.face_iter()
    }

    #[inline(always)]
    pub fn face_iter_mut(&mut self) -> impl Iterator<Item = &mut [V]> {
        self.faces.face_iter_mut()
    }

    #[inline(always)]
    pub fn editor(&mut self) -> PolygonMeshEditor<'_, V, A> {
        PolygonMeshEditor {
            attributes: &mut self.attributes,
            faces: &mut self.faces,
            bound_check: true,
        }
    }

    #[inline(always)]
    pub fn uncheck_editor(&mut self) -> PolygonMeshEditor<'_, V, A> {
        PolygonMeshEditor {
            attributes: &mut self.attributes,
            faces: &mut self.faces,
            bound_check: false,
        }
    }

    #[inline(always)]
    pub fn debug_editor(&mut self) -> PolygonMeshEditor<'_, V, A> {
        PolygonMeshEditor {
            attributes: &mut self.attributes,
            faces: &mut self.faces,
            bound_check: cfg!(debug_assertions),
        }
    }
}

impl PolygonMesh {
    pub fn merge(&mut self, mut mesh: PolygonMesh) {
        let n_pos = self.positions().len();
        let n_uv = self.uv_coords().len();
        let n_nor = self.normals().len();
        mesh.faces.face_iter_mut().for_each(move |face| {
            face.iter_mut().for_each(|v| {
                v.pos += n_pos;
                v.uv = v.uv.map(|uv| uv + n_uv);
                v.nor = v.nor.map(|nor| nor + n_nor);
            })
        });
        self.attributes.positions.extend(mesh.attributes.positions);
        self.attributes.uv_coords.extend(mesh.attributes.uv_coords);
        self.attributes.normals.extend(mesh.attributes.normals);
        self.faces.naive_concat(mesh.faces);
    }

    #[inline(always)]
    pub fn bounding_box(&self) -> BoundingBox<Point3> {
        self.positions().iter().collect()
    }

    #[inline(always)]
    pub fn to_positions_mesh(&self) -> PolygonMesh<usize, Vec<Point3>> {
        let faces = self.faces();
        let tri_faces = faces
            .tri_faces()
            .iter()
            .map(|face| [face[0].pos, face[1].pos, face[2].pos])
            .collect::<Vec<_>>();
        let quad_faces = faces
            .quad_faces()
            .iter()
            .map(|face| [face[0].pos, face[1].pos, face[2].pos, face[3].pos])
            .collect::<Vec<_>>();
        let other_faces = faces
            .other_faces()
            .iter()
            .map(|face| face.iter().map(|x| x.pos).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        PolygonMesh {
            attributes: self.positions().clone(),
            faces: Faces {
                tri_faces,
                quad_faces,
                other_faces,
            },
        }
    }
}

impl Invertible for PolygonMesh {
    #[inline(always)]
    fn invert(&mut self) {
        self.attributes.normals.iter_mut().for_each(|n| *n = -*n);
        self.faces.invert();
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        Self {
            attributes: StandardAttributes {
                positions: self.attributes.positions.clone(),
                uv_coords: self.attributes.uv_coords.clone(),
                normals: self.attributes.normals.iter().map(|n| -*n).collect(),
            },
            faces: self.faces.inverse(),
        }
    }
}

impl PolygonMesh {
    #[inline(always)]
    pub const fn positions(&self) -> &Vec<Point3> {
        &self.attributes.positions
    }

    #[inline(always)]
    pub fn positions_mut(&mut self) -> &mut [Point3] {
        &mut self.attributes.positions
    }

    #[inline(always)]
    pub fn push_position(&mut self, position: Point3) {
        self.attributes.positions.push(position)
    }

    #[inline(always)]
    pub fn extend_positions<I: IntoIterator<Item = Point3>>(&mut self, iter: I) {
        self.attributes.positions.extend(iter)
    }

    #[inline(always)]
    pub const fn uv_coords(&self) -> &Vec<Vector2> {
        &self.attributes.uv_coords
    }

    #[inline(always)]
    pub fn uv_coords_mut(&mut self) -> &mut [Vector2] {
        &mut self.attributes.uv_coords
    }

    #[inline(always)]
    pub fn push_uv_coord(&mut self, uv_coord: Vector2) {
        self.attributes.uv_coords.push(uv_coord)
    }

    #[inline(always)]
    pub fn extend_uv_coords<I: IntoIterator<Item = Vector2>>(&mut self, iter: I) {
        self.attributes.uv_coords.extend(iter)
    }

    #[inline(always)]
    pub const fn normals(&self) -> &Vec<Vector3> {
        &self.attributes.normals
    }

    #[inline(always)]
    pub fn normals_mut(&mut self) -> &mut [Vector3] {
        &mut self.attributes.normals
    }

    #[inline(always)]
    pub fn extend_normals<I: IntoIterator<Item = Vector3>>(&mut self, iter: I) {
        self.attributes.normals.extend(iter)
    }
}

impl<V, A: Default> Default for PolygonMesh<V, A> {
    fn default() -> Self {
        Self {
            attributes: A::default(),
            faces: Faces::default(),
        }
    }
}

impl<'de, V, A> Deserialize<'de> for PolygonMesh<V, A>
where
    V: Copy + Debug + Deserialize<'de>,
    A: Attributes<V> + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PolygonMesh_<V, A> {
            attributes: A,
            faces: Faces<V>,
        }
        let PolygonMesh_ { attributes, faces } = PolygonMesh_::<V, A>::deserialize(deserializer)?;
        Self::try_new(attributes, faces).map_err(serde::de::Error::custom)
    }
}

impl<V: Clone, A: TransformedAttributes> Transformed<Matrix4> for PolygonMesh<V, A> {
    #[inline(always)]
    fn transform_by(&mut self, trans: Matrix4) {
        self.attributes.transform_by(trans);
    }
    #[inline(always)]
    fn transformed(&self, trans: Matrix4) -> Self {
        Self {
            attributes: self.attributes.transformed(trans),
            faces: self.faces.clone(),
        }
    }
}

#[derive(Debug)]
pub struct PolygonMeshEditor<'a, V: Copy + Debug, A: Attributes<V>> {
    pub attributes: &'a mut A,
    pub faces: &'a mut Faces<V>,
    bound_check: bool,
}

impl<V: Copy + Debug, A: Attributes<V>> PolygonMeshEditor<'_, V, A> {
    #[inline(always)]
    fn is_compatible(&self) -> Result<(), Error<V>> {
        self.faces.is_compatible(&*self.attributes)
    }

    #[inline(always)]
    pub fn try_drop(mut self) -> Result<(), Error<V>> {
        self.bound_check = false;
        self.is_compatible()
    }
}

impl<V: Copy + Debug, A: Attributes<V>> Drop for PolygonMeshEditor<'_, V, A> {
    #[inline(always)]
    fn drop(&mut self) {
        if self.bound_check {
            self.is_compatible().unwrap_or_else(|e| panic!("{e:?}"));
        }
    }
}
