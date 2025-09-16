use crate::*;
use rayon::prelude::*;
use rustc_hash::FxHashSet as HashSet;
use std::collections::{vec_deque, VecDeque};
use std::iter::Peekable;
use truck_base::entry_map::FxEntryMap as EntryMap;

impl<P, C> Wire<P, C> {
    #[inline(always)]
    pub fn new() -> Wire<P, C> {
        Self::default()
    }

    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Wire<P, C> {
        Wire {
            edge_list: VecDeque::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn edge_iter(&self) -> EdgeIter<'_, P, C> {
        self.iter()
    }

    #[inline(always)]
    pub fn edge_iter_mut(&mut self) -> EdgeIterMut<'_, P, C> {
        self.iter_mut()
    }

    #[inline(always)]
    pub fn edge_into_iter(self) -> EdgeIntoIter<P, C> {
        self.into_iter()
    }

    #[inline(always)]
    pub fn edge_par_iter(&self) -> EdgeParallelIter<'_, P, C>
    where
        P: Send,
        C: Send,
    {
        self.par_iter()
    }

    #[inline(always)]
    pub fn edge_par_iter_mut(&mut self) -> EdgeParallelIterMut<'_, P, C>
    where
        P: Send,
        C: Send,
    {
        self.par_iter_mut()
    }

    #[inline(always)]
    pub fn edge_into_par_iter(self) -> EdgeParallelIntoIter<P, C>
    where
        P: Send,
        C: Send,
    {
        self.into_par_iter()
    }

    #[inline(always)]
    pub fn vertex_iter(&self) -> VertexIter<'_, P, C> {
        VertexIter {
            edge_iter: self.edge_iter().peekable(),
            unconti_next: None,
            cyclic: self.is_cyclic(),
        }
    }

    #[inline(always)]
    pub fn front_edge(&self) -> Option<&Edge<P, C>> {
        self.front()
    }

    #[inline(always)]
    pub fn front_vertex(&self) -> Option<&Vertex<P>> {
        self.front().map(|edge| edge.front())
    }

    #[inline(always)]
    pub fn back_edge(&self) -> Option<&Edge<P, C>> {
        self.back()
    }

    #[inline(always)]
    pub fn back_vertex(&self) -> Option<&Vertex<P>> {
        self.back().map(|edge| edge.back())
    }

    #[inline(always)]
    pub fn ends_vertices(&self) -> Option<(&Vertex<P>, &Vertex<P>)> {
        match (self.front_vertex(), self.back_vertex()) {
            (Some(got0), Some(got1)) => Some((got0, got1)),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn append(&mut self, other: &mut Wire<P, C>) {
        self.edge_list.append(&mut other.edge_list)
    }

    #[inline(always)]
    pub fn split_off(&mut self, at: usize) -> Wire<P, C> {
        Wire {
            edge_list: self.edge_list.split_off(at),
        }
    }

    #[inline(always)]
    pub fn invert(&mut self) -> &mut Self {
        *self = self.inverse();
        self
    }

    #[inline(always)]
    pub fn inverse(&self) -> Wire<P, C> {
        let edge_list = self.edge_iter().rev().map(|edge| edge.inverse()).collect();
        Wire { edge_list }
    }

    pub fn is_continuous(&self) -> bool {
        let mut iter = self.edge_iter();
        if let Some(edge) = iter.next() {
            let mut prev = edge.back();
            for edge in iter {
                if prev != edge.front() {
                    return false;
                }
                prev = edge.back();
            }
        }
        true
    }

    #[inline(always)]
    pub fn is_cyclic(&self) -> bool {
        self.front_vertex() == self.back_vertex()
    }

    #[inline(always)]
    pub fn is_closed(&self) -> bool {
        self.is_continuous() && self.is_cyclic()
    }

    pub fn is_simple(&self) -> bool {
        let mut set = HashSet::default();
        self.vertex_iter()
            .all(move |vertex| set.insert(vertex.id()))
    }

    pub fn disjoint_wires(wires: &[Wire<P, C>]) -> bool {
        let mut set = HashSet::default();
        wires.iter().all(move |wire| {
            let mut vec = Vec::new();
            let res = wire.vertex_iter().all(|v| {
                vec.push(v.id());
                !set.contains(&v.id())
            });
            set.extend(vec);
            res
        })
    }

    pub fn swap_edge_into_wire(&mut self, idx: usize, wire: Wire<P, C>) -> bool {
        if wire.is_empty() || self[idx].ends() != wire.ends_vertices().unwrap() {
            return false;
        }
        let mut new_wire: Vec<_> = self.drain(0..idx).collect();
        new_wire.extend(wire);
        self.pop_front();
        new_wire.extend(self.drain(..));
        *self = new_wire.into();
        true
    }

    pub(super) fn swap_subwire_into_edges(&mut self, mut idx: usize, edge: Edge<P, C>) {
        if idx + 1 == self.len() {
            self.rotate_left(1);
            idx -= 1;
        }
        let mut new_wire: Vec<_> = self.drain(0..idx).collect();
        new_wire.push(edge);
        self.pop_front();
        self.pop_front();
        new_wire.extend(self.drain(..));
        *self = new_wire.into();
    }

    pub(super) fn sub_try_mapped<'a, Q, D, KF, KV>(
        &'a self,
        edge_map: &mut EdgeEntryMapForTryMapping<'a, P, C, Q, D, KF, KV>,
    ) -> Option<Wire<Q, D>>
    where
        KF: FnMut(&'a Edge<P, C>) -> EdgeID<C>,
        KV: FnMut(&'a Edge<P, C>) -> Option<Edge<Q, D>>,
    {
        self.edge_iter()
            .map(|edge| {
                let new_edge = edge_map.entry_or_insert(edge).as_ref()?;
                match edge.orientation() {
                    true => Some(new_edge.clone()),
                    false => Some(new_edge.inverse()),
                }
            })
            .collect()
    }

    #[doc(hidden)]
    pub fn try_mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
        mut curve_mapping: impl FnMut(&C) -> Option<D>,
    ) -> Option<Wire<Q, D>> {
        let mut vertex_map = EntryMap::new(Vertex::id, move |v| v.try_mapped(&mut point_mapping));
        let mut edge_map = EntryMap::new(
            Edge::id,
            edge_entry_map_try_closure(&mut vertex_map, &mut curve_mapping),
        );
        self.sub_try_mapped(&mut edge_map)
    }

    pub(super) fn sub_mapped<'a, Q, D, KF, KV>(
        &'a self,
        edge_map: &mut EdgeEntryMapForMapping<'a, P, C, Q, D, KF, KV>,
    ) -> Wire<Q, D>
    where
        KF: FnMut(&'a Edge<P, C>) -> EdgeID<C>,
        KV: FnMut(&'a Edge<P, C>) -> Edge<Q, D>,
    {
        self.edge_iter()
            .map(|edge| {
                let new_edge = edge_map.entry_or_insert(edge);
                match edge.orientation() {
                    true => new_edge.clone(),
                    false => new_edge.inverse(),
                }
            })
            .collect()
    }

    #[doc(hidden)]
    pub fn mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Q,
        mut curve_mapping: impl FnMut(&C) -> D,
    ) -> Wire<Q, D> {
        let mut vertex_map = EntryMap::new(Vertex::id, move |v| v.mapped(&mut point_mapping));
        let mut edge_map = EntryMap::new(
            Edge::id,
            edge_entry_map_closure(&mut vertex_map, &mut curve_mapping),
        );
        self.sub_mapped(&mut edge_map)
    }

    #[inline(always)]
    pub fn is_geometric_consistent(&self) -> bool
    where
        P: Tolerance,
        C: BoundedCurve<Point = P>,
    {
        self.iter().all(|edge| edge.is_geometric_consistent())
    }

    #[inline(always)]
    pub fn display(&self, format: WireDisplayFormat) -> DebugDisplay<'_, Self, WireDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

type EdgeEntryMapForTryMapping<'a, P, C, Q, D, KF, KV> =
    EntryMap<EdgeID<C>, Option<Edge<Q, D>>, KF, KV, &'a Edge<P, C>>;
type EdgeEntryMapForMapping<'a, P, C, Q, D, KF, KV> =
    EntryMap<EdgeID<C>, Edge<Q, D>, KF, KV, &'a Edge<P, C>>;

pub(super) fn edge_entry_map_try_closure<'a, P, C, Q, D, KF, VF>(
    vertex_map: &'a mut EntryMap<VertexID<P>, Option<Vertex<Q>>, KF, VF, &'a Vertex<P>>,
    curve_mapping: &'a mut impl FnMut(&C) -> Option<D>,
) -> impl FnMut(&'a Edge<P, C>) -> Option<Edge<Q, D>> + 'a
where
    KF: FnMut(&'a Vertex<P>) -> VertexID<P>,
    VF: FnMut(&'a Vertex<P>) -> Option<Vertex<Q>>,
{
    move |edge| {
        let vf = edge.absolute_front();
        let vertex0 = vertex_map.entry_or_insert(vf).clone()?;
        let vb = edge.absolute_back();
        let vertex1 = vertex_map.entry_or_insert(vb).clone()?;
        let curve = curve_mapping(&*edge.curve.lock())?;
        Some(Edge::debug_new(&vertex0, &vertex1, curve))
    }
}

pub(super) fn edge_entry_map_closure<'a, P, C, Q, D, KF, VF>(
    vertex_map: &'a mut EntryMap<VertexID<P>, Vertex<Q>, KF, VF, &'a Vertex<P>>,
    curve_mapping: &'a mut impl FnMut(&C) -> D,
) -> impl FnMut(&'a Edge<P, C>) -> Edge<Q, D> + 'a
where
    KF: FnMut(&'a Vertex<P>) -> VertexID<P>,
    VF: FnMut(&'a Vertex<P>) -> Vertex<Q>,
{
    move |edge| {
        let vf = edge.absolute_front();
        let vertex0 = vertex_map.entry_or_insert(vf).clone();
        let vb = edge.absolute_back();
        let vertex1 = vertex_map.entry_or_insert(vb).clone();
        let curve = curve_mapping(&*edge.curve.lock());
        Edge::debug_new(&vertex0, &vertex1, curve)
    }
}

impl<P, C, T> From<T> for Wire<P, C>
where
    VecDeque<Edge<P, C>>: From<T>,
{
    #[inline(always)]
    fn from(edge_list: T) -> Wire<P, C> {
        Wire {
            edge_list: edge_list.into(),
        }
    }
}

impl<P, C> FromIterator<Edge<P, C>> for Wire<P, C> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = Edge<P, C>>>(iter: I) -> Wire<P, C> {
        Wire::from(VecDeque::from_iter(iter))
    }
}

impl<'a, P, C> FromIterator<&'a Edge<P, C>> for Wire<P, C> {
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = &'a Edge<P, C>>>(iter: I) -> Wire<P, C> {
        Wire::from(VecDeque::from_iter(iter.into_iter().map(Edge::clone)))
    }
}

impl<P, C> IntoIterator for Wire<P, C> {
    type Item = Edge<P, C>;
    type IntoIter = EdgeIntoIter<P, C>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.edge_list.into_iter()
    }
}

impl<'a, P, C> IntoIterator for &'a Wire<P, C> {
    type Item = &'a Edge<P, C>;
    type IntoIter = EdgeIter<'a, P, C>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.edge_list.iter()
    }
}
pub type EdgeIter<'a, P, C> = vec_deque::Iter<'a, Edge<P, C>>;
pub type EdgeIterMut<'a, P, C> = vec_deque::IterMut<'a, Edge<P, C>>;
pub type EdgeIntoIter<P, C> = vec_deque::IntoIter<Edge<P, C>>;
pub type EdgeParallelIter<'a, P, C> = <VecDeque<Edge<P, C>> as IntoParallelRefIterator<'a>>::Iter;
pub type EdgeParallelIterMut<'a, P, C> =
    <VecDeque<Edge<P, C>> as IntoParallelRefMutIterator<'a>>::Iter;
pub type EdgeParallelIntoIter<P, C> = <VecDeque<Edge<P, C>> as IntoParallelIterator>::Iter;

#[derive(Clone, Debug)]
pub struct VertexIter<'a, P, C> {
    edge_iter: Peekable<EdgeIter<'a, P, C>>,
    unconti_next: Option<Vertex<P>>,
    cyclic: bool,
}

impl<P, C> Iterator for VertexIter<'_, P, C> {
    type Item = Vertex<P>;

    fn next(&mut self) -> Option<Vertex<P>> {
        if self.unconti_next.is_some() {
            let res = self.unconti_next.clone();
            self.unconti_next = None;
            res
        } else if let Some(edge) = self.edge_iter.next() {
            if let Some(next) = self.edge_iter.peek() {
                if edge.back() != next.front() {
                    self.unconti_next = Some(edge.back().clone());
                }
            } else if !self.cyclic {
                self.unconti_next = Some(edge.back().clone());
            }
            Some(edge.front().clone())
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let min_size = self.edge_iter.len();
        let max_size = self.edge_iter.len() * 2;
        (min_size, Some(max_size))
    }

    fn last(self) -> Option<Vertex<P>> {
        let closed = self.cyclic;
        self.edge_iter.last().map(|edge| {
            if closed {
                edge.front().clone()
            } else {
                edge.back().clone()
            }
        })
    }
}

impl<P, C> std::iter::FusedIterator for VertexIter<'_, P, C> {}

impl<P, C> Extend<Edge<P, C>> for Wire<P, C> {
    fn extend<T: IntoIterator<Item = Edge<P, C>>>(&mut self, iter: T) {
        for edge in iter {
            self.push_back(edge);
        }
    }
}

impl<P, C> AsRef<VecDeque<Edge<P, C>>> for Wire<P, C> {
    #[inline(always)]
    fn as_ref(&self) -> &VecDeque<Edge<P, C>> {
        &self.edge_list
    }
}

impl<P, C> std::ops::Deref for Wire<P, C> {
    type Target = VecDeque<Edge<P, C>>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.edge_list
    }
}

impl<P, C> std::ops::DerefMut for Wire<P, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.edge_list
    }
}

impl<P, C> std::borrow::Borrow<VecDeque<Edge<P, C>>> for Wire<P, C> {
    #[inline(always)]
    fn borrow(&self) -> &VecDeque<Edge<P, C>> {
        &self.edge_list
    }
}

impl<P, C> Clone for Wire<P, C> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            edge_list: self.edge_list.clone(),
        }
    }
}

impl<P, C> PartialEq for Wire<P, C> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.edge_list == other.edge_list
    }
}

impl<P, C> Eq for Wire<P, C> {}

impl<P, C> Default for Wire<P, C> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            edge_list: Default::default(),
        }
    }
}

impl<P: Debug, C: Debug> Debug for DebugDisplay<'_, Wire<P, C>, WireDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            WireDisplayFormat::EdgesListTuple { edge_format } => f
                .debug_tuple("Wire")
                .field(&Self {
                    entity: self.entity,
                    format: WireDisplayFormat::EdgesList { edge_format },
                })
                .finish(),
            WireDisplayFormat::EdgesList { edge_format } => f
                .debug_list()
                .entries(
                    self.entity
                        .edge_iter()
                        .map(|edge| edge.display(edge_format)),
                )
                .finish(),
            WireDisplayFormat::VerticesList { vertex_format } => {
                let vertices: Vec<_> = self.entity.vertex_iter().collect();
                f.debug_list()
                    .entries(vertices.iter().map(|vertex| vertex.display(vertex_format)))
                    .finish()
            }
        }
    }
}

impl<P: Send, C: Send> FromParallelIterator<Edge<P, C>> for Wire<P, C> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = Edge<P, C>>,
    {
        Self::from(VecDeque::from_par_iter(par_iter))
    }
}

impl<P: Send, C: Send> IntoParallelIterator for Wire<P, C> {
    type Item = Edge<P, C>;
    type Iter = EdgeParallelIntoIter<P, C>;
    fn into_par_iter(self) -> Self::Iter {
        self.edge_list.into_par_iter()
    }
}

impl<'a, P: Send + 'a, C: Send + 'a> IntoParallelRefIterator<'a> for Wire<P, C> {
    type Item = &'a Edge<P, C>;
    type Iter = EdgeParallelIter<'a, P, C>;
    fn par_iter(&'a self) -> Self::Iter {
        self.edge_list.par_iter()
    }
}

impl<'a, P: Send + 'a, C: Send + 'a> IntoParallelRefMutIterator<'a> for Wire<P, C> {
    type Item = &'a mut Edge<P, C>;
    type Iter = EdgeParallelIterMut<'a, P, C>;
    fn par_iter_mut(&'a mut self) -> Self::Iter {
        self.edge_list.par_iter_mut()
    }
}

impl<P: Send, C: Send> ParallelExtend<Edge<P, C>> for Wire<P, C> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = Edge<P, C>>,
    {
        self.edge_list.par_extend(par_iter)
    }
}
