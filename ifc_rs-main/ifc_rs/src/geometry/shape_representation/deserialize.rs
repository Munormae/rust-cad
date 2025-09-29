use crate::{
    parser::{
        comma::Comma, list::IfcList, optional::OptionalParameter, p_space_or_comment_surrounded,
        IFCParse,
    },
    prelude::*,
};

use super::ShapeRepresentation;

impl IFCParse for ShapeRepresentation {
    fn parse<'a>() -> impl crate::parser::IFCParser<'a, Self>
    where
        Self: Sized,
    {
        winnow::seq! {
            ShapeRepresentation {
                _: p_space_or_comment_surrounded("IFCSHAPEREPRESENTATION("),
                context_of_items: Id::parse(),
                _: Comma::parse(),
                representation_identifier: OptionalParameter::parse(),
                _: Comma::parse(),
                representation_type: OptionalParameter::parse(),
                _: Comma::parse(),
                items: IfcList::parse(),
                _: p_space_or_comment_surrounded(");"),
            }
        }
    }
}

#[test]
fn parse_shape_representation_works() {
    use winnow::prelude::*;

    let data = "IFCSHAPEREPRESENTATION(#107,'Body','MappedRepresentation',(#2921786));";
    let parsed = ShapeRepresentation::parse().parse(data).unwrap();
    assert_eq!(format!("{data}"), format!("{parsed}"));
}
