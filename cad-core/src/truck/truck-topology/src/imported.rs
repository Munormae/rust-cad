#[macro_export]
macro_rules! prelude {
    ($point: ty, $curve: ty, $surface: ty $(, $pub: tt $($super: tt)?)?) => {
        #[allow(unused)]
        $($pub$($super)?)? use $crate::{
            compress::CompressedEdgeIndex,
            shell::ShellCondition,
            VertexDisplayFormat,
            EdgeDisplayFormat,
            WireDisplayFormat,
            FaceDisplayFormat,
            ShellDisplayFormat,
            SolidDisplayFormat,
            wire,
            shell,
        };

        #[allow(unused)]
        $($pub$($super)?)? type Vertex = $crate::Vertex<$point>;
        #[allow(unused)]
        $($pub$($super)?)? type Edge = $crate::Edge<$point, $curve>;
        #[allow(unused)]
        $($pub$($super)?)? type Wire = $crate::Wire<$point, $curve>;
        #[allow(unused)]
        $($pub$($super)?)? type Face = $crate::Face<$point, $curve, $surface>;
        #[allow(unused)]
        $($pub$($super)?)? type Shell = $crate::Shell<$point, $curve, $surface>;
        #[allow(unused)]
        $($pub$($super)?)? type Solid = $crate::Solid<$point, $curve, $surface>;
        #[allow(unused)]
        $($pub$($super)?)? type VertexID = $crate::VertexID<$point>;
        #[allow(unused)]
        $($pub$($super)?)? type EdgeID = $crate::EdgeID<$curve>;
        #[allow(unused)]
        $($pub$($super)?)? type FaceID = $crate::FaceID<$surface>;
        #[allow(unused)]
        $($pub$($super)?)? type CompressedEdge = $crate::compress::CompressedEdge<$curve>;
        #[allow(unused)]
        $($pub$($super)?)? type CompressedFace = $crate::compress::CompressedFace<$surface>;
        #[allow(unused)]
        $($pub$($super)?)? type CompressedShell = $crate::compress::CompressedShell<$point, $curve, $surface>;
        #[allow(unused)]
        $($pub$($super)?)? type CompressedSolid = $crate::compress::CompressedSolid<$point, $curve, $surface>;
    };
}

#[doc(hidden)]
pub mod empty_geometries {
    #![allow(missing_debug_implementations)]
    pub struct Point;
    pub struct Curve;
    pub struct Surface;
}
#[doc(hidden)]
pub use empty_geometries::*;
prelude!(Point, Curve, Surface, pub);
