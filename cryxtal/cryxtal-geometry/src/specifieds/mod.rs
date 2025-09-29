use crate::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Line<P>(pub P, pub P);

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SelfSameGeometry)]
pub struct UnitCircle<P>(std::marker::PhantomData<P>);

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SelfSameGeometry)]
pub struct UnitHyperbola<P>(std::marker::PhantomData<P>);

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SelfSameGeometry)]
pub struct UnitParabola<P>(std::marker::PhantomData<P>);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Plane {
    o: Point3,
    p: Point3,
    q: Point3,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Sphere {
    center: Point3,
    radius: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, SelfSameGeometry)]
pub struct Torus {
    center: Point3,
    large_radius: f64,
    small_radius: f64,
}

mod circle;
mod hyperbola;
mod line;
mod parabola;
mod plane;
mod sphere;
mod torus;

macro_rules! always_true {
    ($ty: tt) => {
        impl<P> PartialEq for $ty<P> {
            fn eq(&self, _: &Self) -> bool {
                true
            }
        }
        impl<P> Eq for $ty<P> {}
    };
}

always_true!(UnitCircle);
always_true!(UnitParabola);
always_true!(UnitHyperbola);
