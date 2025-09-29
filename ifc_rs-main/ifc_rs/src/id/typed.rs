use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use winnow::Parser;

use crate::{parser::IFCParse, prelude::*};

use super::{Id, IdOr};

#[derive(PartialOrd, Ord)]
pub struct TypedId<T: IfcType> {
    id: Id,
    t: PhantomData<T>,
}

impl<T: IfcType> Debug for TypedId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}", id = self.id.0)
    }
}

impl<T: IfcType> Copy for TypedId<T> {}

impl<T: IfcType> Eq for TypedId<T> {}

impl<T: IfcType> std::hash::Hash for TypedId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: IfcType> PartialEq for TypedId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: IfcType> Clone for TypedId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: IfcType> TypedId<T> {
    pub fn new(id: Id) -> Self {
        Self { id, t: PhantomData }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn id_or(&self) -> IdOr<T> {
        IdOr::Id(*self)
    }
}

impl<T: IfcType> From<Id> for TypedId<T> {
    fn from(value: Id) -> Self {
        Self::new(value)
    }
}

impl<T: IfcType> IFCParse for TypedId<T> {
    fn parse<'a>() -> impl crate::parser::IFCParser<'a, Self>
    where
        Self: Sized,
    {
        Id::parse().map(TypedId::from)
    }
}

impl<T: IfcType> Display for TypedId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}", id = self.id())
    }
}

pub struct FixMeType;
impl IfcVerify for FixMeType {}
impl IfcType for FixMeType {}
impl Display for FixMeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AHHHHH")
    }
}
