#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use truck_base::{id::ID, tolerance::*};
use truck_geotrait::*;

#[cfg(feature = "rclite")]
use rclite::Arc;
#[cfg(not(feature = "rclite"))]
use std::sync::Arc;

const SEARCH_PARAMETER_TRIALS: usize = 100;

#[derive(Debug)]
pub struct Vertex<P> {
    point: Arc<Mutex<P>>,
}

#[derive(Debug)]
pub struct Edge<P, C> {
    vertices: (Vertex<P>, Vertex<P>),
    orientation: bool,
    curve: Arc<Mutex<C>>,
}

#[derive(Debug)]
pub struct Wire<P, C> {
    edge_list: VecDeque<Edge<P, C>>,
}

#[derive(Debug)]
pub struct Face<P, C, S> {
    boundaries: Vec<Wire<P, C>>,
    orientation: bool,
    surface: Arc<Mutex<S>>,
}

#[derive(Debug)]
pub struct Shell<P, C, S> {
    face_list: Vec<Face<P, C, S>>,
}

#[derive(Clone, Debug)]
pub struct Solid<P, C, S> {
    boundaries: Vec<Shell<P, C, S>>,
}

pub type Result<T> = std::result::Result<T, errors::Error>;

trait RemoveTry<T> {
    fn remove_try(self) -> T;
}

impl<T> RemoveTry<T> for Result<T> {
    #[inline(always)]
    fn remove_try(self) -> T {
        self.unwrap_or_else(|e| panic!("{}", e))
    }
}

pub type VertexID<P> = ID<Mutex<P>>;

pub type EdgeID<C> = ID<Mutex<C>>;

pub type FaceID<S> = ID<Mutex<S>>;

#[derive(Clone, Copy, Debug)]
pub enum VertexDisplayFormat {
    Full,
    IDTuple,
    PointTuple,
    AsPoint,
}

#[derive(Clone, Copy, Debug)]
pub enum EdgeDisplayFormat {
    Full { vertex_format: VertexDisplayFormat },
    VerticesTupleAndID { vertex_format: VertexDisplayFormat },
    VerticesTupleAndCurve { vertex_format: VertexDisplayFormat },
    VerticesTupleStruct { vertex_format: VertexDisplayFormat },
    VerticesTuple { vertex_format: VertexDisplayFormat },
    AsCurve,
}

#[derive(Clone, Copy, Debug)]
pub enum WireDisplayFormat {
    EdgesListTuple { edge_format: EdgeDisplayFormat },
    EdgesList { edge_format: EdgeDisplayFormat },
    VerticesList { vertex_format: VertexDisplayFormat },
}

#[derive(Clone, Copy, Debug)]
pub enum FaceDisplayFormat {
    Full { wire_format: WireDisplayFormat },
    BoundariesAndID { wire_format: WireDisplayFormat },
    BoundariesAndSurface { wire_format: WireDisplayFormat },
    LoopsListTuple { wire_format: WireDisplayFormat },
    LoopsList { wire_format: WireDisplayFormat },
    AsSurface,
}

#[derive(Clone, Copy, Debug)]
pub enum ShellDisplayFormat {
    FacesListTuple { face_format: FaceDisplayFormat },
    FacesList { face_format: FaceDisplayFormat },
}

#[derive(Clone, Copy, Debug)]
pub enum SolidDisplayFormat {
    Struct { shell_format: ShellDisplayFormat },
    ShellsListTuple { shell_format: ShellDisplayFormat },
    ShellsList { shell_format: ShellDisplayFormat },
}

pub mod compress;
mod edge;
pub mod errors;
pub mod face;
pub mod shell;
mod solid;
mod vertex;
pub mod wire;
pub mod format {
    use crate::*;

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct DebugDisplay<'a, T, Format> {
        pub(super) entity: &'a T,
        pub(super) format: Format,
    }

    #[derive(Clone)]
    pub(super) struct MutexFmt<'a, T>(pub &'a Mutex<T>);

    impl<T: Debug> Debug for MutexFmt<'_, T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!("{:?}", self.0.lock()))
        }
    }
}
pub mod imported;

#[macro_export]
macro_rules! wire { ($($t:tt)*) => { $crate::Wire::from_iter([$($t)*]) }; }

#[macro_export]
macro_rules! shell { ($($t:tt)*) => { $crate::Shell::from_iter([$($t)*]) }; }
