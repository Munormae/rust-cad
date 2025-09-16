use crate::{errors::Error, *};
use thiserror::Error;

impl<P, C> Edge<P, C> {
    #[inline(always)]
    pub fn try_new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Result<Edge<P, C>> {
        if front == back {
            Err(Error::SameVertex)
        } else {
            Ok(Edge::new_unchecked(front, back, curve))
        }
    }

    #[inline(always)]
    pub fn new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        Edge::try_new(front, back, curve).remove_try()
    }

    #[inline(always)]
    pub fn new_unchecked(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        Edge {
            vertices: (front.clone(), back.clone()),
            orientation: true,
            curve: Arc::new(Mutex::new(curve)),
        }
    }

    #[inline(always)]
    pub fn debug_new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        match cfg!(debug_assertions) {
            true => Edge::new(front, back, curve),
            false => Edge::new_unchecked(front, back, curve),
        }
    }

    #[inline(always)]
    pub const fn orientation(&self) -> bool {
        self.orientation
    }

    #[inline(always)]
    pub fn invert(&mut self) -> &mut Self {
        self.orientation = !self.orientation;
        self
    }

    #[inline(always)]
    pub fn inverse(&self) -> Edge<P, C> {
        Edge {
            vertices: self.vertices.clone(),
            orientation: !self.orientation,
            curve: Arc::clone(&self.curve),
        }
    }

    #[inline(always)]
    pub fn front(&self) -> &Vertex<P> {
        match self.orientation {
            true => &self.vertices.0,
            false => &self.vertices.1,
        }
    }

    #[inline(always)]
    pub fn back(&self) -> &Vertex<P> {
        match self.orientation {
            true => &self.vertices.1,
            false => &self.vertices.0,
        }
    }

    #[inline(always)]
    pub fn ends(&self) -> (&Vertex<P>, &Vertex<P>) {
        match self.orientation {
            true => (&self.vertices.0, &self.vertices.1),
            false => (&self.vertices.1, &self.vertices.0),
        }
    }

    #[inline(always)]
    pub const fn absolute_front(&self) -> &Vertex<P> {
        &self.vertices.0
    }

    #[inline(always)]
    pub const fn absolute_back(&self) -> &Vertex<P> {
        &self.vertices.1
    }

    #[inline(always)]
    pub const fn absolute_ends(&self) -> (&Vertex<P>, &Vertex<P>) {
        (&self.vertices.0, &self.vertices.1)
    }

    #[inline(always)]
    pub fn absolute_clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            curve: Arc::clone(&self.curve),
            orientation: true,
        }
    }

    #[inline(always)]
    pub fn is_same(&self, other: &Edge<P, C>) -> bool {
        self.id() == other.id()
    }

    #[inline(always)]
    pub fn curve(&self) -> C
    where
        C: Clone,
    {
        self.curve.lock().clone()
    }

    #[inline(always)]
    pub fn set_curve(&self, curve: C) {
        *self.curve.lock() = curve;
    }

    #[inline(always)]
    pub fn id(&self) -> EdgeID<C> {
        ID::new(Arc::as_ptr(&self.curve))
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.curve)
    }

    #[inline(always)]
    pub fn oriented_curve(&self) -> C
    where
        C: Clone + Invertible,
    {
        match self.orientation {
            true => self.curve.lock().clone(),
            false => self.curve.lock().inverse(),
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn try_mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
        mut curve_mapping: impl FnMut(&C) -> Option<D>,
    ) -> Option<Edge<Q, D>> {
        let v0 = self.absolute_front().try_mapped(&mut point_mapping)?;
        let v1 = self.absolute_back().try_mapped(&mut point_mapping)?;
        let curve = curve_mapping(&*self.curve.lock())?;
        let mut edge = Edge::debug_new(&v0, &v1, curve);
        if !self.orientation() {
            edge.invert();
        }
        Some(edge)
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Q,
        mut curve_mapping: impl FnMut(&C) -> D,
    ) -> Edge<Q, D> {
        let v0 = self.absolute_front().mapped(&mut point_mapping);
        let v1 = self.absolute_back().mapped(&mut point_mapping);
        let curve = curve_mapping(&*self.curve.lock());
        let mut edge = Edge::debug_new(&v0, &v1, curve);
        if edge.orientation() != self.orientation() {
            edge.invert();
        }
        edge
    }

    #[inline(always)]
    pub fn is_geometric_consistent(&self) -> bool
    where
        P: Tolerance,
        C: BoundedCurve<Point = P>,
    {
        let curve = self.curve.lock();
        let geom_front = curve.front();
        let geom_back = curve.back();
        let top_front = self.absolute_front().point.lock();
        let top_back = self.absolute_back().point.lock();
        geom_front.near(&*top_front) && geom_back.near(&*top_back)
    }

    #[inline(always)]
    fn pre_cut(&self, vertex: &Vertex<P>, mut curve0: C, t: f64) -> (Self, Self)
    where
        C: Cut<Point = P>,
    {
        let curve1 = curve0.cut(t);
        let edge0 = Edge {
            vertices: (self.absolute_front().clone(), vertex.clone()),
            orientation: self.orientation,
            curve: Arc::new(Mutex::new(curve0)),
        };
        let edge1 = Edge {
            vertices: (vertex.clone(), self.absolute_back().clone()),
            orientation: self.orientation,
            curve: Arc::new(Mutex::new(curve1)),
        };
        match self.orientation {
            true => (edge0, edge1),
            false => (edge1, edge0),
        }
    }

    pub fn cut(&self, vertex: &Vertex<P>) -> Option<(Self, Self)>
    where
        P: Clone,
        C: Cut<Point = P> + SearchParameter<D1, Point = P>,
    {
        let curve0 = self.curve();
        let t = curve0.search_parameter(vertex.point(), None, SEARCH_PARAMETER_TRIALS)?;
        let (t0, t1) = curve0.range_tuple();
        if t < t0 + TOLERANCE || t1 - TOLERANCE < t {
            return None;
        }
        Some(self.pre_cut(vertex, curve0, t))
    }

    pub fn cut_with_parameter(&self, vertex: &Vertex<P>, t: f64) -> Option<(Self, Self)>
    where
        P: Clone + Tolerance,
        C: Cut<Point = P>,
    {
        let curve0 = self.curve();
        if !curve0.subs(t).near(&vertex.point()) {
            return None;
        }
        let (t0, t1) = curve0.range_tuple();
        if t < t0 + TOLERANCE || t1 - TOLERANCE < t {
            return None;
        }
        Some(self.pre_cut(vertex, curve0, t))
    }

    pub fn concat(&self, rhs: &Self) -> std::result::Result<Self, ConcatError<P>>
    where
        P: Debug,
        C: Concat<C, Point = P, Output = C> + Invertible + ParameterTransform,
    {
        if self.back() != rhs.front() {
            return Err(ConcatError::DisconnectedVertex(
                self.back().clone(),
                rhs.front().clone(),
            ));
        }
        if self.front() == rhs.back() {
            return Err(ConcatError::SameVertex(self.front().clone()));
        }
        let curve0 = self.oriented_curve();
        let mut curve1 = rhs.oriented_curve();
        let t0 = curve0.range_tuple().1;
        let t1 = curve1.range_tuple().0;
        curve1.parameter_transform(1.0, t0 - t1);
        let curve = curve0.try_concat(&curve1)?;
        Ok(Edge::debug_new(self.front(), rhs.back(), curve))
    }

    #[inline(always)]
    pub fn display(&self, format: EdgeDisplayFormat) -> DebugDisplay<'_, Self, EdgeDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum ConcatError<P: Debug> {
    #[error("The end point {0:?} of the first curve is different from the start point {1:?} of the second curve.")]
    DisconnectedVertex(Vertex<P>, Vertex<P>),
    #[error("The end vertices are the same vertex {0:?}.")]
    SameVertex(Vertex<P>),
    #[error("{0}")]
    FromGeometry(truck_geotrait::ConcatError<P>),
}

impl<P: Debug> From<truck_geotrait::ConcatError<P>> for ConcatError<P> {
    fn from(err: truck_geotrait::ConcatError<P>) -> ConcatError<P> {
        ConcatError::FromGeometry(err)
    }
}

impl<P, C> Clone for Edge<P, C> {
    #[inline(always)]
    fn clone(&self) -> Edge<P, C> {
        Edge {
            vertices: self.vertices.clone(),
            orientation: self.orientation,
            curve: Arc::clone(&self.curve),
        }
    }
}

impl<P, C> PartialEq for Edge<P, C> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(Arc::as_ptr(&self.curve), Arc::as_ptr(&other.curve))
            && self.orientation == other.orientation
    }
}

impl<P, C> Eq for Edge<P, C> {}

impl<P, C> Hash for Edge<P, C> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.curve), state);
        self.orientation.hash(state);
    }
}

impl<P: Debug, C: Debug> Debug for DebugDisplay<'_, Edge<P, C>, EdgeDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            EdgeDisplayFormat::Full { vertex_format } => f
                .debug_struct("Edge")
                .field("id", &Arc::as_ptr(&self.entity.curve))
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .field("entity", &MutexFmt(&self.entity.curve))
                .finish(),
            EdgeDisplayFormat::VerticesTupleAndID { vertex_format } => f
                .debug_struct("Edge")
                .field("id", &self.entity.id())
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .finish(),
            EdgeDisplayFormat::VerticesTupleAndCurve { vertex_format } => f
                .debug_struct("Edge")
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .field("entity", &MutexFmt(&self.entity.curve))
                .finish(),
            EdgeDisplayFormat::VerticesTupleStruct { vertex_format } => f
                .debug_tuple("Edge")
                .field(&self.entity.front().display(vertex_format))
                .field(&self.entity.back().display(vertex_format))
                .finish(),
            EdgeDisplayFormat::VerticesTuple { vertex_format } => f.write_fmt(format_args!(
                "({:?}, {:?})",
                self.entity.front().display(vertex_format),
                self.entity.back().display(vertex_format),
            )),
            EdgeDisplayFormat::AsCurve => {
                f.write_fmt(format_args!("{:?}", &MutexFmt(&self.entity.curve)))
            }
        }
    }
}
