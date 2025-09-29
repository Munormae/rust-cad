use crate::{errors::Error, wire::EdgeIter, *};
use crate::format::{DebugDisplay, MutexFmt};
use std::fmt::Formatter;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

impl<P, C, S> Face<P, C, S> {
    #[inline(always)]
    pub fn try_new(boundaries: Vec<Wire<P, C>>, surface: S) -> Result<Face<P, C, S>> {
        for wire in &boundaries {
            if wire.is_empty() {
                return Err(Error::EmptyWire);
            } else if !wire.is_closed() {
                return Err(Error::NotClosedWire);
            } else if !wire.is_simple() {
                return Err(Error::NotSimpleWire);
            }
        }
        if !Wire::disjoint_wires(&boundaries) {
            Err(Error::NotSimpleWire)
        } else {
            Ok(Face::new_unchecked(boundaries, surface))
        }
    }

    #[inline(always)]
    pub fn new(boundaries: Vec<Wire<P, C>>, surface: S) -> Face<P, C, S> {
        Face::try_new(boundaries, surface).remove_try()
    }

    #[inline(always)]
    pub fn new_unchecked(boundaries: Vec<Wire<P, C>>, surface: S) -> Face<P, C, S> {
        Face {
            boundaries,
            orientation: true,
            surface: Arc::new(Mutex::new(surface)),
        }
    }

    #[inline(always)]
    pub fn debug_new(boundaries: Vec<Wire<P, C>>, surface: S) -> Face<P, C, S> {
        match cfg!(debug_assertions) {
            true => Face::new(boundaries, surface),
            false => Face::new_unchecked(boundaries, surface),
        }
    }

    #[inline(always)]
    pub fn boundaries(&self) -> Vec<Wire<P, C>> {
        match self.orientation {
            true => self.boundaries.clone(),
            false => self.boundaries.iter().map(|wire| wire.inverse()).collect(),
        }
    }

    #[inline(always)]
    pub fn into_boundaries(self) -> Vec<Wire<P, C>> {
        match self.orientation {
            true => self.boundaries,
            false => self.boundaries(),
        }
    }

    #[inline(always)]
    pub const fn absolute_boundaries(&self) -> &Vec<Wire<P, C>> {
        &self.boundaries
    }

    #[inline(always)]
    pub fn absolute_clone(&self) -> Self {
        Self {
            boundaries: self.boundaries.clone(),
            surface: Arc::clone(&self.surface),
            orientation: true,
        }
    }

    #[inline(always)]
    pub fn boundary_iters(&self) -> Vec<BoundaryIter<'_, P, C>> {
        self.boundaries
            .iter()
            .map(|wire| BoundaryIter {
                edge_iter: wire.edge_iter(),
                orientation: self.orientation,
            })
            .collect()
    }

    #[inline(always)]
    fn renew_pointer(&mut self)
    where
        S: Clone,
    {
        let surface = self.surface();
        self.surface = Arc::new(Mutex::new(surface));
    }

    #[inline(always)]
    pub fn edge_iter(&self) -> impl Iterator<Item = Edge<P, C>> + '_ {
        self.boundary_iters().into_iter().flatten()
    }

    #[inline(always)]
    pub fn vertex_iter(&self) -> impl Iterator<Item = Vertex<P>> + '_ {
        self.edge_iter().map(|e| e.front().clone())
    }

    #[inline(always)]
    pub fn try_add_boundary(&mut self, mut wire: Wire<P, C>) -> Result<()>
    where
        S: Clone,
    {
        if wire.is_empty() {
            return Err(Error::EmptyWire);
        } else if !wire.is_closed() {
            return Err(Error::NotClosedWire);
        } else if !wire.is_simple() {
            return Err(Error::NotSimpleWire);
        }
        if !self.orientation {
            wire.invert();
        }
        self.boundaries.push(wire);
        self.renew_pointer();
        if !Wire::disjoint_wires(&self.boundaries) {
            self.boundaries.pop();
            return Err(Error::NotDisjointWires);
        }
        Ok(())
    }

    #[inline(always)]
    pub fn add_boundary(&mut self, wire: Wire<P, C>)
    where
        S: Clone,
    {
        self.try_add_boundary(wire).remove_try()
    }

    #[doc(hidden)]
    pub fn try_mapped<Q, D, T>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
        mut curve_mapping: impl FnMut(&C) -> Option<D>,
        mut surface_mapping: impl FnMut(&S) -> Option<T>,
    ) -> Option<Face<Q, D, T>> {
        let wires = self
            .absolute_boundaries()
            .iter()
            .map(|wire| wire.try_mapped(&mut point_mapping, &mut curve_mapping))
            .collect::<Option<Vec<_>>>()?;
        let surface = surface_mapping(&*self.surface.lock())?;
        let mut face = Face::debug_new(wires, surface);
        if !self.orientation() {
            face.invert();
        }
        Some(face)
    }

    #[doc(hidden)]
    pub fn mapped<Q, D, T>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Q,
        mut curve_mapping: impl FnMut(&C) -> D,
        mut surface_mapping: impl FnMut(&S) -> T,
    ) -> Face<Q, D, T> {
        let wires: Vec<_> = self
            .absolute_boundaries()
            .iter()
            .map(|wire| wire.mapped(&mut point_mapping, &mut curve_mapping))
            .collect();
        let surface = surface_mapping(&*self.surface.lock());
        let mut face = Face::debug_new(wires, surface);
        if !self.orientation() {
            face.invert();
        }
        face
    }

    #[inline(always)]
    pub fn orientation(&self) -> bool {
        self.orientation
    }

    #[inline(always)]
    pub fn surface(&self) -> S
    where
        S: Clone,
    {
        self.surface.lock().clone()
    }

    #[inline(always)]
    pub fn set_surface(&self, surface: S) {
        *self.surface.lock() = surface;
    }

    #[inline(always)]
    pub fn invert(&mut self) -> &mut Self {
        self.orientation = !self.orientation;
        self
    }

    #[inline(always)]
    pub fn is_same(&self, other: &Self) -> bool {
        std::ptr::eq(Arc::as_ptr(&self.surface), Arc::as_ptr(&other.surface))
    }

    #[inline(always)]
    pub fn id(&self) -> FaceID<S> {
        ID::new(Arc::as_ptr(&self.surface))
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.surface)
    }

    #[inline(always)]
    pub fn inverse(&self) -> Face<P, C, S> {
        let mut face = self.clone();
        face.invert();
        face
    }

    pub fn border_on(&self, other: &Face<P, C, S>) -> bool {
        let edge_iter = self.boundary_iters().into_iter().flatten();
        let hashset: HashSet<_> = edge_iter.map(|edge| edge.id()).collect();
        let mut edge_iter = other.boundary_iters().into_iter().flatten();
        edge_iter.any(|edge| hashset.contains(&edge.id()))
    }

    pub fn border_wires(&self, other: &Face<P, C, S>) -> Vec<Wire<P, C>> {
        let edge_iter = other.boundary_iters().into_iter().flatten();
        let hashset: HashSet<_> = edge_iter.map(|edge| edge.id()).collect();
        let closure = move |boundary: BoundaryIter<'_, P, C>| {
            let mut conti = false;
            let mut border_wires = Vec::<Wire<P, C>>::new();
            for edge in boundary {
                if hashset.contains(&edge.id()) {
                    if conti {
                        let wire = border_wires.last_mut().unwrap();
                        wire.push_back(edge);
                    } else {
                        border_wires.push(wire![edge]);
                        conti = true;
                    }
                } else {
                    conti = false;
                }
            }
            if conti && border_wires.len() > 1 {
                let first = border_wires.first().unwrap().front_vertex().unwrap();
                let back = border_wires.last().unwrap().back_vertex().unwrap();
                if first == back {
                    let first = border_wires.remove(0);
                    border_wires.last_mut().unwrap().extend(first);
                }
            }
            border_wires
        };
        self.boundary_iters()
            .into_iter()
            .flat_map(closure)
            .collect()
    }

    pub fn cut_by_edge(&self, edge: Edge<P, C>) -> Option<(Self, Self)>
    where
        S: Clone,
    {
        self.cut_by_wire([edge].into())
    }

    pub fn cut_by_wire(&self, wire: Wire<P, C>) -> Option<(Self, Self)>
    where
        S: Clone,
    {
        if self.boundaries.len() != 1 {
            return None;
        }
        let mut face0 = Face {
            boundaries: self.boundaries.clone(),
            orientation: self.orientation,
            surface: Arc::new(Mutex::new(self.surface())),
        };
        let boundary = &mut face0.boundaries[0];
        let i = boundary
            .edge_iter()
            .enumerate()
            .find(|(_, e)| Some(e.front()) == wire.back_vertex())
            .map(|(i, _)| i)?;
        let j = boundary
            .edge_iter()
            .enumerate()
            .find(|(_, e)| Some(e.back()) == wire.front_vertex())
            .map(|(i, _)| i)?;
        boundary.rotate_left(i);
        let j = (j + boundary.len() - i) % boundary.len();
        let mut new_wire = boundary.split_off(j + 1);
        new_wire.extend(wire.iter().rev().map(|e| e.inverse()));
        boundary.extend(wire);
        debug_assert!(Face::try_new(self.boundaries.clone(), ()).is_ok());
        debug_assert!(Face::try_new(vec![new_wire.clone()], ()).is_ok());
        let face1 = Face {
            boundaries: vec![new_wire],
            orientation: self.orientation,
            surface: Arc::new(Mutex::new(self.surface())),
        };
        Some((face0, face1))
    }

    pub fn glue_at_boundaries(&self, other: &Self) -> Option<Self>
    where
        S: Clone + PartialEq,
        Wire<P, C>: Debug,
    {
        let surface = self.surface();
        if surface != other.surface() || self.orientation() != other.orientation() {
            return None;
        }
        let mut vemap: HashMap<VertexID<P>, &Edge<P, C>> = self
            .absolute_boundaries()
            .iter()
            .flatten()
            .map(|edge| (edge.front().id(), edge))
            .collect();
        other
            .absolute_boundaries()
            .iter()
            .flatten()
            .try_for_each(|edge| {
                if let Some(edge0) = vemap.get(&edge.back().id()) {
                    if edge.front() == edge0.back() {
                        return if edge.is_same(edge0) {
                            vemap.remove(&edge.back().id());
                            Some(())
                        } else {
                            None
                        }
                    }
                }
                vemap.insert(edge.front().id(), edge);
                Some(())
            })?;
        if vemap.is_empty() {
            return None;
        }
        let mut boundaries = Vec::new();
        while !vemap.is_empty() {
            let mut wire = Wire::new();
            let v = *vemap.iter().next().unwrap().0;
            let mut edge = vemap.remove(&v).unwrap();
            wire.push_back(edge.clone());
            while let Some(edge0) = vemap.remove(&edge.back().id()) {
                wire.push_back(edge0.clone());
                edge = edge0;
            }
            boundaries.push(wire);
        }
        debug_assert!(Face::try_new(boundaries.clone(), ()).is_ok());
        Some(Face {
            boundaries,
            orientation: self.orientation(),
            surface: Arc::new(Mutex::new(surface)),
        })
    }

    #[inline(always)]
    pub fn display(&self, format: FaceDisplayFormat) -> DebugDisplay<'_, Self, FaceDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

impl<P, C, S: Clone + Invertible> Face<P, C, S> {
    #[inline(always)]
    pub fn oriented_surface(&self) -> S {
        match self.orientation {
            true => self.surface.lock().clone(),
            false => self.surface.lock().inverse(),
        }
    }
}

impl<P, C, S> Face<P, C, S>
where
    P: Tolerance,
    C: BoundedCurve<Point = P>,
    S: IncludeCurve<C>,
{
    #[inline(always)]
    pub fn is_geometric_consistent(&self) -> bool {
        let surface = &*self.surface.lock();
        self.boundary_iters().into_iter().flatten().all(|edge| {
            let edge_consist = edge.is_geometric_consistent();
            let curve = &*edge.curve.lock();
            let curve_consist = surface.include(curve);
            edge_consist && curve_consist
        })
    }
}

impl<P, C, S> Clone for Face<P, C, S> {
    #[inline(always)]
    fn clone(&self) -> Face<P, C, S> {
        Face {
            boundaries: self.boundaries.clone(),
            orientation: self.orientation,
            surface: Arc::clone(&self.surface),
        }
    }
}

impl<P, C, S> PartialEq for Face<P, C, S> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(Arc::as_ptr(&self.surface), Arc::as_ptr(&other.surface))
            && self.orientation == other.orientation
    }
}

impl<P, C, S> Eq for Face<P, C, S> {}

impl<P, C, S> Hash for Face<P, C, S> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.surface), state);
        self.orientation.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct BoundaryIter<'a, P, C> {
    edge_iter: EdgeIter<'a, P, C>,
    orientation: bool,
}

impl<P, C> Iterator for BoundaryIter<'_, P, C> {
    type Item = Edge<P, C>;
    #[inline(always)]
    fn next(&mut self) -> Option<Edge<P, C>> {
        match self.orientation {
            true => self.edge_iter.next().cloned(),
            false => self.edge_iter.next_back().map(|edge| edge.inverse()),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }

    #[inline(always)]
    fn last(mut self) -> Option<Edge<P, C>> {
        self.next_back()
    }
}

impl<P, C> DoubleEndedIterator for BoundaryIter<'_, P, C> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Edge<P, C>> {
        match self.orientation {
            true => self.edge_iter.next_back().cloned(),
            false => self.edge_iter.next().map(|edge| edge.inverse()),
        }
    }
}

impl<P, C> ExactSizeIterator for BoundaryIter<'_, P, C> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.edge_iter.len()
    }
}

impl<P, C> std::iter::FusedIterator for BoundaryIter<'_, P, C> {}

impl<P: Debug, C: Debug, S: Debug> Debug for DebugDisplay<'_, Face<P, C, S>, FaceDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            FaceDisplayFormat::Full { wire_format } => f
                .debug_struct("Face")
                .field("id", &self.entity.id())
                .field(
                    "boundaries",
                    &self
                        .entity
                        .boundaries()
                        .iter()
                        .map(|wire| wire.display(wire_format))
                        .collect::<Vec<_>>(),
                )
                .field("entity", &MutexFmt(&self.entity.surface))
                .finish(),
            FaceDisplayFormat::BoundariesAndID { wire_format } => f
                .debug_struct("Face")
                .field("id", &self.entity.id())
                .field(
                    "boundaries",
                    &self
                        .entity
                        .boundaries()
                        .iter()
                        .map(|wire| wire.display(wire_format))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            FaceDisplayFormat::BoundariesAndSurface { wire_format } => f
                .debug_struct("Face")
                .field(
                    "boundaries",
                    &self
                        .entity
                        .boundaries()
                        .iter()
                        .map(|wire| wire.display(wire_format))
                        .collect::<Vec<_>>(),
                )
                .field("entity", &MutexFmt(&self.entity.surface))
                .finish(),
            FaceDisplayFormat::LoopsListTuple { wire_format } => f
                .debug_tuple("Face")
                .field(
                    &self
                        .entity
                        .boundaries()
                        .iter()
                        .map(|wire| wire.display(wire_format))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            FaceDisplayFormat::LoopsList { wire_format } => f
                .debug_list()
                .entries(
                    self.entity
                        .boundaries()
                        .iter()
                        .map(|wire| wire.display(wire_format)),
                )
                .finish(),
            FaceDisplayFormat::AsSurface => {
                f.write_fmt(format_args!("{:?}", &MutexFmt(&self.entity.surface)))
            }
        }
    }
}

#[test]
fn invert_mapped_face() {
    let v = Vertex::news([0, 1, 2, 3, 4, 5, 6]);
    let wire0 = Wire::from(vec![
        Edge::new(&v[0], &v[1], 100),
        Edge::new(&v[1], &v[2], 200),
        Edge::new(&v[2], &v[3], 300),
        Edge::new(&v[3], &v[0], 400),
    ]);
    let wire1 = Wire::from(vec![
        Edge::new(&v[4], &v[5], 500),
        Edge::new(&v[6], &v[5], 600).inverse(),
        Edge::new(&v[6], &v[4], 700),
    ]);
    let face0 = Face::new(vec![wire0, wire1], 10000).inverse();
    let face1 = face0.mapped(
        &move |i: &usize| *i + 10,
        &move |j: &usize| *j + 1000,
        &move |k: &usize| *k + 100000,
    );

    assert_eq!(face0.surface() + 100000, face1.surface(),);
    assert_eq!(face0.orientation(), face1.orientation());
    let biters0 = face0.boundary_iters();
    let biters1 = face1.boundary_iters();
    for (biter0, biter1) in biters0.into_iter().zip(biters1) {
        for (edge0, edge1) in biter0.zip(biter1) {
            assert_eq!(edge0.front().point() + 10, edge1.front().point(),);
            assert_eq!(edge0.back().point() + 10, edge1.back().point(),);
            assert_eq!(edge0.curve() + 1000, edge1.curve(),);
        }
    }
}
