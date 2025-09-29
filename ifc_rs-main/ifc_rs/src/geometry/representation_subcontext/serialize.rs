use std::fmt::Display;

use crate::geometry::representation_subcontext::GeometricRepresentationSubContext;

impl Display for GeometricRepresentationSubContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT({context_id},{context_type},{coord_dims},{precision},{world_coord_system},{true_north},{parent_context},{target_scale},{target_view},{user_defined_target_view});",
            context_id = self.context_identifier,
            context_type = self.context_type,
            coord_dims = self.coord_space_dimension,
            precision = self.precision,
            world_coord_system = self.world_coord_system,
            true_north = self.true_north,
            parent_context = self.parent_context,
            target_scale = self.target_scale,
            target_view = self.target_view,
            user_defined_target_view = self.user_defined_target_view,
        )
    }
}
