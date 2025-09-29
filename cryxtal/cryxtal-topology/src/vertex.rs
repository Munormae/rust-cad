use crate::*;
use crate::format::{DebugDisplay, MutexFmt};
use std::fmt::Formatter;

impl<P> Vertex<P> {
    #[inline(always)]
    pub fn new(point: P) -> Vertex<P> {
        Vertex {
            point: Arc::new(Mutex::new(point)),
        }
    }

    #[inline(always)]
    pub fn news(points: impl AsRef<[P]>) -> Vec<Vertex<P>>
    where
        P: Copy,
    {
        points.as_ref().iter().map(|p| Vertex::new(*p)).collect()
    }

    #[inline(always)]
    pub fn point(&self) -> P
    where
        P: Clone,
    {
        self.point.lock().clone()
    }

    #[inline(always)]
    pub fn set_point(&self, point: P) {
        *self.point.lock() = point;
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn try_mapped<Q>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
    ) -> Option<Vertex<Q>> {
        Some(Vertex::new(point_mapping(&*self.point.lock())?))
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn mapped<Q>(&self, mut point_mapping: impl FnMut(&P) -> Q) -> Vertex<Q> {
        Vertex::new(point_mapping(&*self.point.lock()))
    }

    #[inline(always)]
    pub fn id(&self) -> VertexID<P> {
        ID::new(Arc::as_ptr(&self.point))
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.point)
    }

    #[inline(always)]
    pub fn display(
        &self,
        format: VertexDisplayFormat,
    ) -> DebugDisplay<'_, Self, VertexDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

impl<P> Clone for Vertex<P> {
    #[inline(always)]
    fn clone(&self) -> Vertex<P> {
        Vertex {
            point: Arc::clone(&self.point),
        }
    }
}

impl<P> PartialEq for Vertex<P> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl<P> Eq for Vertex<P> {}

impl<P> Hash for Vertex<P> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.point), state);
    }
}

impl<P: Debug> Debug for DebugDisplay<'_, Vertex<P>, VertexDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            VertexDisplayFormat::Full => f
                .debug_struct("Vertex")
                .field("id", &Arc::as_ptr(&self.entity.point))
                .field("entity", &MutexFmt(&self.entity.point))
                .finish(),
            VertexDisplayFormat::IDTuple => {
                f.debug_tuple("Vertex").field(&self.entity.id()).finish()
            }
            VertexDisplayFormat::PointTuple => f
                .debug_tuple("Vertex")
                .field(&MutexFmt(&self.entity.point))
                .finish(),
            VertexDisplayFormat::AsPoint => {
                f.write_fmt(format_args!("{:?}", &MutexFmt(&self.entity.point)))
            }
        }
    }
}
