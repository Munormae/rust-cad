use crate::geometry::arbitrary_closed_profile_def::ArbitraryClosedProfileDef;
use crate::geometry::profile_type::ProfileType;
use crate::id::Id;
use crate::parser::comma::Comma;
use crate::parser::optional::OptionalParameter;
use crate::parser::*;

impl IFCParse for ArbitraryClosedProfileDef {
    fn parse<'a>() -> impl IFCParser<'a, Self> {
        winnow::seq! {
            ArbitraryClosedProfileDef {
                _: p_space_or_comment_surrounded("IFCARBITRARYCLOSEDPROFILEDEF("),

                profile_type: ProfileType::parse(),
                _: Comma::parse(),
                profile_name: OptionalParameter::parse(),
                _: Comma::parse(),
                outer_curve: Id::parse(),

                _: p_space_or_comment_surrounded(");"),
            }
        }
    }
}
