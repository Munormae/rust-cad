use crate::*;
use rayon::prelude::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use truck_base::entry_map::FxEntryMap as EntryMap;
#[derive(Clone, Debug)]
pub struct AdjacentFace<'a, P, C, S> {
    pub face: &'a Face<P, C, S>,
    pub common_edges: Vec<EdgeID<C>>,
}

trait As<T> {
    fn as_(self) -> T;
}

impl<T> As<T> for T {
    fn as_(self) -> T {
        self
    }
}

impl<'a, P, C, S> As<&'a Face<P, C, S>> for AdjacentFace<'a, P, C, S> {
    fn as_(self) -> &'a Face<P, C, S> {
        self.face
    }
}

type FaceAdjacencyMap<'a, P, C, S> = HashMap<&'a Face<P, C, S>, Vec<AdjacentFace<'a, P, C, S>>>;
impl<P, C, S> Shell<P, C, S> {
    #[inline(always)]
    pub const fn new() -> Shell<P, C, S> {
        Shell {
            face_list: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Shell<P, C, S> {
        Shell {
            face_list: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn face_iter(&self) -> FaceIter<'_, P, C, S> {
        self.iter()
    }
    #[inline(always)]
    pub fn face_iter_mut(&mut self) -> FaceIterMut<'_, P, C, S> {
        self.iter_mut()
    }
    #[inline(always)]
    pub fn face_into_iter(self) -> FaceIntoIter<P, C, S> {
        self.face_list.into_iter()
    }
    #[inline(always)]
    pub fn face_par_iter(&self) -> FaceParallelIter<'_, P, C, S>
    where
        P: Send,
        C: Send,
        S: Send,
    {
        self.par_iter()
    }

    #[inline(always)]
    pub fn face_par_iter_mut(&mut self) -> FaceParallelIterMut<'_, P, C, S>
    where
        P: Send,
        C: Send,
        S: Send,
    {
        self.par_iter_mut()
    }

    #[inline(always)]
    pub fn face_into_par_iter(self) -> FaceParallelIntoIter<P, C, S>
    where
        P: Send,
        C: Send,
        S: Send,
    {
        self.into_par_iter()
    }

    #[inline(always)]
    pub fn edge_iter(&self) -> impl Iterator<Item = Edge<P, C>> + '_ {
        self.face_iter().flat_map(Face::edge_iter)
    }

    #[inline(always)]
    pub fn edge_par_iter(&self) -> impl ParallelIterator<Item = Edge<P, C>> + '_
    where
        P: Send,
        C: Send,
        S: Send,
    {
        self.face_par_iter().flat_map(Face::boundaries).flatten()
    }

    #[inline(always)]
    pub fn vertex_iter(&self) -> impl Iterator<Item = Vertex<P>> + '_ {
        self.edge_iter().map(|edge| edge.front().clone())
    }

    #[inline(always)]
    pub fn vertex_par_iter(&self) -> impl ParallelIterator<Item = Vertex<P>> + '_
    where
        P: Send,
        C: Send,
        S: Send,
    {
        self.edge_par_iter().map(|edge| edge.front().clone())
    }

    #[inline(always)]
    pub fn append(&mut self, other: &mut Shell<P, C, S>) {
        self.face_list.append(&mut other.face_list);
    }

    pub fn shell_condition(&self) -> ShellCondition {
        self.edge_iter().collect::<Boundaries<C>>().condition()
    }

    pub fn extract_boundaries(&self) -> Vec<Wire<P, C>> {
        let boundaries: Boundaries<C> = self.edge_iter().collect();
        let mut vemap: HashMap<_, _> = self
            .edge_iter()
            .filter_map(|edge| {
                boundaries
                    .boundaries
                    .get(&edge.id())
                    .map(|_| (edge.front().id(), edge.clone()))
            })
            .collect();
        let mut res = Vec::new();
        while !vemap.is_empty() {
            let edge = self.vertex_iter().find_map(|v| vemap.get(&v.id())).unwrap();
            if let Some(mut cursor) = vemap.remove(&edge.front().id()) {
                let mut wire = Wire::from(vec![cursor.clone()]);
                loop {
                    cursor = match vemap.remove(&cursor.back().id()) {
                        None => break,
                        Some(got) => {
                            wire.push_back(got.clone());
                            got.clone()
                        }
                    };
                }
                res.push(wire);
            }
        }
        res
    }

    pub fn vertex_adjacency(&self) -> HashMap<VertexID<P>, Vec<VertexID<P>>> {
        let mut adjacency = EntryMap::new(|x| x, |_| Vec::new());
        let mut done_edge: HashSet<EdgeID<C>> = HashSet::default();
        self.edge_iter().for_each(|edge| {
            if done_edge.insert(edge.id()) {
                let v0 = edge.front().id();
                let v1 = edge.back().id();
                adjacency.entry_or_insert(v0).push(v1);
                adjacency.entry_or_insert(v1).push(v0);
            }
        });
        adjacency.into()
    }

    pub fn face_adjacency(&self) -> FaceAdjacencyMap<'_, P, C, S> {
        let mut edge_face_map = EntryMap::new(|edge: &Edge<P, C>| edge.id(), |_| Vec::new());
        self.face_iter().for_each(|face| {
            let insert = |edge| edge_face_map.entry_or_insert(edge).push(face);
            face.absolute_boundaries().iter().flatten().for_each(insert)
        });
        let mut adjacency = EntryMap::new(|x| x, |_| Vec::new());
        edge_face_map.into_iter().for_each(|(edge_id, vec)| {
            vec.iter().for_each(|face| {
                let adjacents = adjacency.entry_or_insert(*face);
                vec.iter().for_each(|face0| {
                    if face == face0 {
                        return;
                    }
                    let add_edge = |adjacent: &mut AdjacentFace<'_, P, C, S>| {
                        let res = &adjacent.face == face0;
                        if res {
                            adjacent.common_edges.push(edge_id);
                        }
                        res
                    };
                    let exists = adjacents.iter_mut().any(add_edge);
                    if !exists {
                        adjacents.push(AdjacentFace {
                            face: face0,
                            common_edges: vec![edge_id],
                        });
                    }
                });
            });
        });

        adjacency.into()
    }

    pub fn is_connected(&self) -> bool {
        let mut adjacency = self.vertex_adjacency();
        // Connecting another boundary of the same face with an edge
        for face in self {
            for wire in face.boundaries.windows(2) {
                let v0 = wire[0].front_vertex().unwrap();
                let v1 = wire[1].front_vertex().unwrap();
                adjacency.get_mut(&v0.id()).unwrap().push(v1.id());
                adjacency.get_mut(&v1.id()).unwrap().push(v0.id());
            }
        }
        check_connectivity(&mut adjacency)
    }

    pub fn connected_components(&self) -> Vec<Shell<P, C, S>> {
        let mut adjacency = self.face_adjacency();
        let components = create_components(&mut adjacency);
        components
            .into_iter()
            .map(|vec| vec.into_iter().cloned().collect())
            .collect()
    }

    pub fn singular_vertices(&self) -> Vec<Vertex<P>> {
        let mut vert_wise_adjacency =
            EntryMap::new(Vertex::clone, |_| EntryMap::new(Edge::id, |_| Vec::new()));
        self.face_iter()
            .flat_map(Face::absolute_boundaries)
            .for_each(|wire| {
                let first_edge = &wire[0];
                let mut edge_iter = wire.iter().peekable();
                while let Some(edge) = edge_iter.next() {
                    let adjacency = vert_wise_adjacency.entry_or_insert(edge.back());
                    let next_edge = *edge_iter.peek().unwrap_or(&first_edge);
                    adjacency.entry_or_insert(edge).push(next_edge.id());
                    adjacency.entry_or_insert(next_edge).push(edge.id());
                }
            });
        vert_wise_adjacency
            .into_iter()
            .filter_map(|(vertex, adjacency)| {
                Some(vertex).filter(|_| !check_connectivity(&mut adjacency.into()))
            })
            .collect()
    }

    #[doc(hidden)]
    pub fn try_mapped<Q, D, T>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
        mut curve_mapping: impl FnMut(&C) -> Option<D>,
        mut surface_mapping: impl FnMut(&S) -> Option<T>,
    ) -> Option<Shell<Q, D, T>> {
        let mut vertex_map = EntryMap::new(Vertex::id, move |v| v.try_mapped(&mut point_mapping));
        let mut edge_map = EntryMap::new(
            Edge::id,
            wire::edge_entry_map_try_closure(&mut vertex_map, &mut curve_mapping),
        );
        self.face_iter()
            .map(|face| {
                let wires = face
                    .absolute_boundaries()
                    .iter()
                    .map(|wire| wire.sub_try_mapped(&mut edge_map))
                    .collect::<Option<Vec<_>>>()?;
                let surface = surface_mapping(&*face.surface.lock())?;
                let mut new_face = Face::debug_new(wires, surface);
                if !face.orientation() {
                    new_face.invert();
                }
                Some(new_face)
            })
            .collect()
    }

    #[doc(hidden)]
    pub fn mapped<Q, D, T>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Q,
        mut curve_mapping: impl FnMut(&C) -> D,
        mut surface_mapping: impl FnMut(&S) -> T,
    ) -> Shell<Q, D, T> {
        let mut vertex_map = EntryMap::new(Vertex::id, |v| v.mapped(&mut point_mapping));
        let mut edge_map = EntryMap::new(
            Edge::id,
            wire::edge_entry_map_closure(&mut vertex_map, &mut curve_mapping),
        );
        self.face_iter()
            .map(|face| {
                let wires: Vec<Wire<_, _>> = face
                    .absolute_boundaries()
                    .iter()
                    .map(|wire| wire.sub_mapped(&mut edge_map))
                    .collect();
                let surface = surface_mapping(&*face.surface.lock());
                let mut new_face = Face::debug_new(wires, surface);
                if !face.orientation() {
                    new_face.invert();
                }
                new_face
            })
            .collect()
    }

    #[inline(always)]
    pub fn is_geometric_consistent(&self) -> bool
    where
        P: Tolerance,
        C: BoundedCurve<Point = P>,
        S: IncludeCurve<C>,
    {
        self.iter().all(|face| face.is_geometric_consistent())
    }

    pub fn cut_edge(
        &mut self,
        edge_id: EdgeID<C>,
        vertex: &Vertex<P>,
    ) -> Option<(Edge<P, C>, Edge<P, C>)>
    where
        P: Clone,
        C: Cut<Point = P> + SearchParameter<D1, Point = P>,
    {
        if self.vertex_iter().any(|v| &v == vertex) {
            return None;
        }
        let mut edges = None;
        self.iter_mut()
            .flat_map(|face| face.boundaries.iter_mut())
            .try_for_each(|wire| {
                let find_res = wire
                    .iter()
                    .enumerate()
                    .find(|(_, edge)| edge.id() == edge_id);
                let (idx, edge) = match find_res {
                    Some(got) => got,
                    None => return Some(()),
                };
                if edges.is_none() {
                    edges = Some(edge.absolute_clone().cut(vertex)?);
                }
                let edges = edges.as_ref().unwrap();
                let new_wire = match edge.orientation() {
                    true => Wire::from(vec![edges.0.clone(), edges.1.clone()]),
                    false => Wire::from(vec![edges.1.inverse(), edges.0.inverse()]),
                };
                let flag = wire.swap_edge_into_wire(idx, new_wire);
                debug_assert!(flag);
                Some(())
            });
        edges
    }

    pub fn remove_vertex_by_concat_edges(&mut self, vertex_id: VertexID<P>) -> Option<Edge<P, C>>
    where
        P: Debug,
        C: Concat<C, Point = P, Output = C> + Invertible + ParameterTransform,
    {
        let mut vec: Vec<(&mut Wire<P, C>, usize)> = self
            .face_iter_mut()
            .flat_map(|face| &mut face.boundaries)
            .filter_map(|wire| {
                let idx = wire
                    .edge_iter()
                    .enumerate()
                    .find(|(_, e)| e.back().id() == vertex_id)?
                    .0;
                Some((wire, idx))
            })
            .collect();
        if vec.len() > 2 || vec.is_empty() {
            None
        } else if vec.len() == 1 {
            let (wire, idx) = vec.pop().unwrap();
            let edge = wire[idx].concat(&wire[(idx + 1) % wire.len()]).ok()?;
            wire.swap_subwire_into_edges(idx, edge.clone());
            Some(edge)
        } else {
            let (wire0, idx0) = vec.pop().unwrap();
            let (wire1, idx1) = vec.pop().unwrap();
            if !wire0[idx0].is_same(&wire1[(idx1 + 1) % wire1.len()])
                || !wire0[(idx0 + 1) % wire0.len()].is_same(&wire1[idx1])
            {
                return None;
            }
            let edge = wire0[idx0].concat(&wire0[(idx0 + 1) % wire0.len()]).ok()?;
            wire1.swap_subwire_into_edges(idx1, edge.inverse());
            wire0.swap_subwire_into_edges(idx0, edge.clone());
            Some(edge)
        }
    }

    pub fn display(
        &self,
        format: ShellDisplayFormat,
    ) -> DebugDisplay<'_, Self, ShellDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

impl<P, C, S> Clone for Shell<P, C, S> {
    #[inline(always)]
    fn clone(&self) -> Shell<P, C, S> {
        Shell {
            face_list: self.face_list.clone(),
        }
    }
}

impl<P, C, S, T> From<T> for Shell<P, C, S>
where
    Vec<Face<P, C, S>>: From<T>,
{
    #[inline(always)]
    fn from(faces: T) -> Shell<P, C, S> {
        Shell {
            face_list: faces.into(),
        }
    }
}

impl<P, C, S> FromIterator<Face<P, C, S>> for Shell<P, C, S> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = Face<P, C, S>>>(iter: I) -> Shell<P, C, S> {
        Shell {
            face_list: Vec::from_iter(iter),
        }
    }
}

impl<P, C, S> IntoIterator for Shell<P, C, S> {
    type Item = Face<P, C, S>;
    type IntoIter = std::vec::IntoIter<Face<P, C, S>>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.face_list.into_iter()
    }
}

impl<'a, P, C, S> IntoIterator for &'a Shell<P, C, S> {
    type Item = &'a Face<P, C, S>;
    type IntoIter = std::slice::Iter<'a, Face<P, C, S>>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.face_list.iter()
    }
}

impl<P, C, S> AsRef<Vec<Face<P, C, S>>> for Shell<P, C, S> {
    #[inline(always)]
    fn as_ref(&self) -> &Vec<Face<P, C, S>> {
        &self.face_list
    }
}

impl<P, C, S> AsRef<[Face<P, C, S>]> for Shell<P, C, S> {
    #[inline(always)]
    fn as_ref(&self) -> &[Face<P, C, S>] {
        &self.face_list
    }
}

impl<P, C, S> std::ops::Deref for Shell<P, C, S> {
    type Target = Vec<Face<P, C, S>>;
    #[inline(always)]
    fn deref(&self) -> &Vec<Face<P, C, S>> {
        &self.face_list
    }
}

impl<P, C, S> std::ops::DerefMut for Shell<P, C, S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Vec<Face<P, C, S>> {
        &mut self.face_list
    }
}

impl<P, C, S> std::borrow::Borrow<Vec<Face<P, C, S>>> for Shell<P, C, S> {
    #[inline(always)]
    fn borrow(&self) -> &Vec<Face<P, C, S>> {
        &self.face_list
    }
}

impl<P, C, S> std::borrow::Borrow<[Face<P, C, S>]> for Shell<P, C, S> {
    #[inline(always)]
    fn borrow(&self) -> &[Face<P, C, S>] {
        &self.face_list
    }
}

impl<P, C, S> Extend<Face<P, C, S>> for Shell<P, C, S> {
    #[inline(always)]
    fn extend<T: IntoIterator<Item = Face<P, C, S>>>(&mut self, iter: T) {
        self.face_list.extend(iter)
    }
}

impl<P, C, S> Default for Shell<P, C, S> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            face_list: Vec::new(),
        }
    }
}

impl<P, C, S> PartialEq for Shell<P, C, S> {
    fn eq(&self, other: &Self) -> bool {
        self.face_list == other.face_list
    }
}

impl<P, C, S> Eq for Shell<P, C, S> {}

pub type FaceIter<'a, P, C, S> = std::slice::Iter<'a, Face<P, C, S>>;
pub type FaceIterMut<'a, P, C, S> = std::slice::IterMut<'a, Face<P, C, S>>;
pub type FaceIntoIter<P, C, S> = std::vec::IntoIter<Face<P, C, S>>;
pub type FaceParallelIter<'a, P, C, S> = <Vec<Face<P, C, S>> as IntoParallelRefIterator<'a>>::Iter;
pub type FaceParallelIterMut<'a, P, C, S> =
    <Vec<Face<P, C, S>> as IntoParallelRefMutIterator<'a>>::Iter;
pub type FaceParallelIntoIter<P, C, S> = <Vec<Face<P, C, S>> as IntoParallelIterator>::Iter;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ShellCondition {
    Irregular,
    Regular,
    Oriented,
    Closed,
}

impl std::ops::BitAnd for ShellCondition {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        match (self, other) {
            (Self::Irregular, _) => Self::Irregular,
            (_, Self::Irregular) => Self::Irregular,
            (Self::Regular, _) => Self::Regular,
            (_, Self::Regular) => Self::Regular,
            (Self::Oriented, _) => Self::Oriented,
            (_, Self::Oriented) => Self::Oriented,
            (Self::Closed, Self::Closed) => Self::Closed,
        }
    }
}

#[derive(Debug, Clone)]
struct Boundaries<C> {
    checked: HashSet<EdgeID<C>>,
    boundaries: HashMap<EdgeID<C>, bool>,
    condition: ShellCondition,
}

impl<C> Boundaries<C> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            checked: Default::default(),
            boundaries: Default::default(),
            condition: ShellCondition::Oriented,
        }
    }

    #[inline(always)]
    fn insert<P>(&mut self, edge: &Edge<P, C>) {
        self.condition = self.condition
            & match (
                self.checked.insert(edge.id()),
                self.boundaries.insert(edge.id(), edge.orientation()),
            ) {
                (true, None) => ShellCondition::Oriented,
                (false, None) => ShellCondition::Irregular,
                (true, Some(_)) => panic!("unexpected case!"),
                (false, Some(ori)) => {
                    self.boundaries.remove(&edge.id());
                    match edge.orientation() == ori {
                        true => ShellCondition::Regular,
                        false => ShellCondition::Oriented,
                    }
                }
            }
    }

    #[inline(always)]
    fn condition(&self) -> ShellCondition {
        if self.condition == ShellCondition::Oriented && self.boundaries.is_empty() {
            ShellCondition::Closed
        } else {
            self.condition
        }
    }
}

impl<P, C> FromIterator<Edge<P, C>> for Boundaries<C> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = Edge<P, C>>>(iter: I) -> Self {
        let mut boundaries = Boundaries::new();
        iter.into_iter().for_each(|edge| boundaries.insert(&edge));
        boundaries
    }
}

fn check_connectivity<T>(adjacency: &mut HashMap<T, Vec<T>>) -> bool
where
    T: Eq + Clone + Hash,
{
    create_one_component(adjacency);
    adjacency.is_empty()
}

fn create_components<T, U>(adjacency: &mut HashMap<T, Vec<U>>) -> Vec<Vec<T>>
where
    T: Eq + Clone + Hash,
    U: As<T>,
{
    let mut res = Vec::new();
    loop {
        let component = create_one_component(adjacency);
        match component.is_empty() {
            true => break,
            false => res.push(component),
        }
    }
    res
}

fn create_one_component<T, U>(adjacency: &mut HashMap<T, Vec<U>>) -> Vec<T>
where
    T: Eq + Hash + Clone,
    U: As<T>,
{
    let mut iter = adjacency.keys();
    let first = match iter.next() {
        Some(key) => key.clone(),
        None => return Vec::new(),
    };
    let mut stack = vec![first.as_()];
    let mut res = Vec::new();
    while let Some(i) = stack.pop() {
        if let Some(vec) = adjacency.remove(&i) {
            res.push(i);
            stack.extend(vec.into_iter().map(|x| x.as_()));
        }
    }
    res
}

impl<P: Debug, C: Debug, S: Debug> Debug for DebugDisplay<'_, Shell<P, C, S>, ShellDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            ShellDisplayFormat::FacesList { face_format } => f
                .debug_list()
                .entries(
                    self.entity
                        .face_iter()
                        .map(|face| face.display(face_format)),
                )
                .finish(),
            ShellDisplayFormat::FacesListTuple { face_format } => f
                .debug_tuple("Shell")
                .field(&DebugDisplay {
                    entity: self.entity,
                    format: ShellDisplayFormat::FacesList { face_format },
                })
                .finish(),
        }
    }
}

impl<P: Send, C: Send, S: Send> FromParallelIterator<Face<P, C, S>> for Shell<P, C, S> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = Face<P, C, S>>,
    {
        Self::from(Vec::from_par_iter(par_iter))
    }
}

impl<P: Send, C: Send, S: Send> IntoParallelIterator for Shell<P, C, S> {
    type Item = Face<P, C, S>;
    type Iter = FaceParallelIntoIter<P, C, S>;
    fn into_par_iter(self) -> Self::Iter {
        self.face_list.into_par_iter()
    }
}

impl<'a, P: Send + 'a, C: Send + 'a, S: Send + 'a> IntoParallelRefIterator<'a> for Shell<P, C, S> {
    type Item = &'a Face<P, C, S>;
    type Iter = FaceParallelIter<'a, P, C, S>;
    fn par_iter(&'a self) -> Self::Iter {
        self.face_list.par_iter()
    }
}

impl<'a, P: Send + 'a, C: Send + 'a, S: Send + 'a> IntoParallelRefMutIterator<'a>
    for Shell<P, C, S>
{
    type Item = &'a mut Face<P, C, S>;
    type Iter = FaceParallelIterMut<'a, P, C, S>;
    fn par_iter_mut(&'a mut self) -> Self::Iter {
        self.face_list.par_iter_mut()
    }
}

impl<P: Send, C: Send, S: Send> ParallelExtend<Face<P, C, S>> for Shell<P, C, S> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = Face<P, C, S>>,
    {
        self.face_list.par_extend(par_iter)
    }
}
