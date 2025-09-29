use std::fmt::Display;

use super::ProductDefinitionShape;

impl Display for ProductDefinitionShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IFCPRODUCTDEFINITIONSHAPE({name},{desc},{items});",
            name = self.name,
            desc = self.description,
            items = self.representations
        )
    }
}
