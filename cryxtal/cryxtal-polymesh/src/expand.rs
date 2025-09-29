use crate::*;
use std::fmt::Debug;
use std::hash::Hash;
use cryxtal_base::entry_map::FxEntryMap as EntryMap;

impl<V: Copy + Hash + Debug + Eq, A: Attributes<V>> PolygonMesh<V, A> {
    pub fn expands<T: Copy>(
        &self,
        contraction: impl Fn(A::Output) -> T,
    ) -> PolygonMesh<usize, Vec<T>> {
        let mut vec = Vec::<T>::new();
        let mut vertex_map = EntryMap::new(
            |x| x,
            |vertex| {
                let idx = vec.len();
                vec.push(contraction(self.attributes.get(vertex).unwrap()));
                idx
            },
        );
        let faces: Faces<usize> = self
            .face_iter()
            .map(|face| {
                face.iter()
                    .cloned()
                    .map(|vertex| *vertex_map.entry_or_insert(vertex))
                    .collect::<Vec<_>>()
            })
            .collect();
        PolygonMesh::new(vec, faces)
    }
}
