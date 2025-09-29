use std::{fmt::Display, ops::Deref};

use ifc_rs_verify_derive::IfcVerify;

use crate::id::{Id, IdOr};
use crate::parser::{
    comma::Comma, list::IfcList, p_space_or_comment_surrounded, string::StringPrimitive, IFCParse,
    IFCParser,
};
use crate::prelude::*;

/// This objectified relationship, IfcRelContainedInSpatialStructure,
/// is used to assign elements to a certain level of the spatial project
/// structure. Any element can only be assigned once to a certain level
/// of the spatial structure. The question, which level is relevant
/// for which type of element, can only be answered within the context
/// of a particular project and might vary within the various regions.
///
/// https://standards.buildingsmart.org/IFC/DEV/IFC4_2/FINAL/HTML/link/ifcrelcontainedinspatialstructure.htm
#[derive(IfcVerify)]
pub struct RelContainedInSpatialStructure {
    root: Root,

    /// Set of products, which are contained within this level of the
    /// spatial structure hierarchy.
    #[ifc_types(Site, Building, Storey, OpeningElement, Slab, Roof, Wall, Window, Door)]
    pub related_elements: IfcList<Id>,

    /// Spatial structure element, within which the element is
    /// contained. Any element can only be contained within one
    /// element of the project spatial structure.
    #[ifc_types(Site, Building, Storey, OpeningElement, Slab, Roof, Wall, Window, Door)]
    pub relating_structure: Id,
}

impl RelContainedInSpatialStructure {
    pub fn new<S: Structure>(
        name: impl Into<StringPrimitive>,
        relating_structure: impl Into<IdOr<S>>,
        ifc: &mut IFC,
    ) -> Self {
        Self {
            root: Root::new(name.into()),
            related_elements: IfcList::empty(),
            relating_structure: relating_structure.into().or_insert(ifc).id(),
        }
    }

    pub fn relate_structure<S: Structure>(
        mut self,
        structure: impl Into<IdOr<S>>,
        ifc: &mut IFC,
    ) -> Self {
        self.related_elements
            .0
            .push(structure.into().or_insert(ifc).id());

        self
    }
}

impl RootBuilder for RelContainedInSpatialStructure {
    fn root_mut(&mut self) -> &mut Root {
        &mut self.root
    }
}

impl Deref for RelContainedInSpatialStructure {
    type Target = Root;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl IFCParse for RelContainedInSpatialStructure {
    fn parse<'a>() -> impl IFCParser<'a, Self> {
        winnow::seq! {
            RelContainedInSpatialStructure {
                _: p_space_or_comment_surrounded("IFCRELCONTAINEDINSPATIALSTRUCTURE("),

                root: Root::parse(),
                _: Comma::parse(),
                related_elements: IfcList::parse(),
                _: Comma::parse(),
                relating_structure: Id::parse(),

                _: p_space_or_comment_surrounded(");"),
            }
        }
    }
}

impl Display for RelContainedInSpatialStructure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({},{},{});",
            self.root, self.related_elements, self.relating_structure
        )
    }
}

impl IfcType for RelContainedInSpatialStructure {}

#[cfg(test)]
mod test {
    use winnow::Parser;

    use super::RelContainedInSpatialStructure;
    use crate::parser::IFCParse;

    #[test]
    fn rel_contained_in_spatial_structure_round_trip() {
        let example = "IFCRELCONTAINEDINSPATIALSTRUCTURE('3Sa3dTJGn0H8TQIGiuGQd5',#2,'Building','Building Container for Elements',(#11),#1);";

        let parsed: RelContainedInSpatialStructure = RelContainedInSpatialStructure::parse()
            .parse(example)
            .unwrap();
        let str = parsed.to_string();

        assert_eq!(example, str);
    }
}
