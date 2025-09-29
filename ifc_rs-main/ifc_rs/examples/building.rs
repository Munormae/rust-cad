use bevy_math::{DVec2, DVec3};
use ifc_rs::prelude::*;
use std::fs::write;

struct VerticalWallParameter {
    pub height: f64,
    pub thickness: f64,
    pub start: f64,
    pub length: f64,
}

fn main() {
    let mut ifc = IFC::default();

    // create person and application info
    let (person_and_organization, application) = create_person_and_org_and_applicaton(
        &mut ifc,
        "Max",
        "ExampleOrganization",
        "BuildingExample",
        "BuildingExample",
    );

    // create owner_history info
    let owner_history = create_owner_history(&mut ifc, person_and_organization, application);

    // create measurement units used for this project
    let length = SiUnit::new(IfcUnitEnum::LengthUnit, None, IfcUnitName::Metre);
    let angle = SiUnit::new(IfcUnitEnum::PlaneAngleUnit, None, IfcUnitName::Radian);
    let time = SiUnit::new(IfcUnitEnum::TimeUnit, None, IfcUnitName::Second);

    // collect measurement units
    let unit_assignment = UnitAssigment::new([length.into(), angle.into(), time.into()], &mut ifc);

    // create world root coordinate
    let world_root = Axis3D::new(Point3D::from(DVec3::new(0.0, 0.0, 0.0)), &mut ifc);
    let world_root_id = ifc.data.insert_new(world_root);

    // create geometry context
    let context =
        GeometricRepresentationContext::new(DimensionCount::Three, world_root_id.id_or(), &mut ifc);
    let context_id = ifc.data.insert_new(context);

    // create project
    let project = Project::new("ExampleProject")
        .owner_history(owner_history.id(), &mut ifc)
        .unit_assignment(unit_assignment, &mut ifc)
        .add_context(context_id.id(), &mut ifc);
    let project_id = ifc.data.insert_new(project);

    // create building
    let building = Building::new("ExampleBuilding").owner_history(owner_history.id(), &mut ifc);
    let building_id = ifc.data.insert_new(building);

    // create relation between project and building
    let project_building_relation =
        RelAggregates::new("ProjectBuildingLink", project_id.id(), [building_id.id()]);
    ifc.data.insert_new(project_building_relation);

    // create subcontext for our model (wall)
    let sub_context = GeometricRepresentationSubContext::derive(
        context_id.id(),
        GeometricProjection::ModelView,
        &mut ifc,
    );
    let sub_context_id = ifc.data.insert_new(sub_context);

    // create wall with parameters
    let wall = create_wall(
        &mut ifc,
        "ExampleWall",
        VerticalWallParameter {
            height: 2.0,
            thickness: 0.02,
            start: 0.0,
            length: 4.0,
        },
        owner_history.id(),
        world_root_id.id_or(),
        sub_context_id,
    );
    let wall_id = ifc.data.insert_new(wall);

    // create wall type
    let wall_type = WallType::new("ExampleWallTypeId", WallTypeEnum::NotDefined)
        .owner_history(owner_history.id(), &mut ifc)
        .name("ExampleWallTypeName");
    let wall_type_id = ifc.data.insert_new(wall_type);

    // // relate wall type to project
    // let wall_type_project_relation = RelDeclares::new("ProjectToWallType", project_id, &mut ifc)
    //     .relate_definition(wall_type_id.id_or(), &mut ifc)
    //     .owner_history(owner_history.id(), &mut ifc);
    // ifc.data.insert_new(wall_type_project_relation);

    // relate wall type to wall
    let wall_wall_type_relation =
        RelDefinesByType::new("WallToWallType", wall_type_id.id_or(), &mut ifc)
            .relate_obj(wall_id.id_or(), &mut ifc);
    ifc.data.insert_new(wall_wall_type_relation);

    // create materials
    let material = Material::new("ExampleMaterial");
    let material_layer = MaterialLayer::new(0.02, false)
        .material(material, &mut ifc)
        .name("ExampleMaterialLayer");
    let material_layer_set = MaterialLayerSet::new()
        .name("ExampleMaterialLayerSet")
        .add_layer(material_layer, &mut ifc);
    let material_layer_set_id = ifc.data.insert_new(material_layer_set);

    let material_layer_set_usage = MaterialLayerSetUsage::new(
        material_layer_set_id.id(),
        LayerSetDirectionEnum::Axis2,
        DirectionSenseEnum::Positive,
        0.0,
        &mut ifc,
    );

    // relate material to wall
    let material_wall_association = RelAssociatesMaterial::new(
        "MaterialWallAssociation",
        material_layer_set_usage,
        &mut ifc,
    )
    .relate_obj(wall_id.id_or(), &mut ifc)
    .owner_history(owner_history.id(), &mut ifc);
    ifc.data.insert_new(material_wall_association);

    // relate material to wall type
    let wall_type_material_association = RelAssociatesMaterial::new(
        "MaterialWallTypeAssociation",
        material_layer_set_id,
        &mut ifc,
    )
    .relate_obj(wall_type_id.id_or(), &mut ifc)
    .owner_history(owner_history.id(), &mut ifc);
    ifc.data.insert_new(wall_type_material_association);

    // relate wall to building
    let spatial_relation =
        RelContainedInSpatialStructure::new("BuildingWallLink", building_id, &mut ifc)
            .relate_structure(wall_id, &mut ifc)
            .owner_history(owner_history.id(), &mut ifc);
    ifc.data.insert_new(spatial_relation);

    write("examples/building_example.ifc", ifc.to_string()).unwrap();
}

fn create_person_and_org_and_applicaton(
    ifc: &mut IFC,
    person_name: &str,
    organization_name: &str,
    application_name: &str,
    application_id: &str,
) -> (TypedId<PersonAndOrganization>, TypedId<Application>) {
    let person = Person::empty().id(person_name).given_name(person_name);
    let person_id = ifc.data.insert_new(person);

    let organization = Organization::new(None, organization_name, None);
    let organization_id = ifc.data.insert_new(organization);

    let person_and_organization = ifc
        .data
        .insert_new(PersonAndOrganization::new(person_id, organization_id));

    let application = Application::new(organization_id, "0.0.1", application_name, application_id);
    let application_id = ifc.data.insert_new(application);

    (person_and_organization, application_id)
}

fn create_owner_history(
    ifc: &mut IFC,
    person_and_organization: TypedId<PersonAndOrganization>,
    application: TypedId<Application>,
) -> TypedId<OwnerHistory> {
    let owner_history = OwnerHistory::new(ChangeAction::Added, IfcTimestamp::now())
        .owning_user(person_and_organization, ifc)
        .owning_application(application.id_or(), ifc)
        .last_modified_date(IfcTimestamp::now());

    ifc.data.insert_new(owner_history)
}

fn create_wall(
    ifc: &mut IFC,
    id: &str,
    wall_parameter: VerticalWallParameter,
    owner_history: impl Into<IdOr<OwnerHistory>>,
    placement: IdOr<Axis3D>,
    sub_context: TypedId<GeometricRepresentationSubContext>,
) -> Wall {
    let shape_repr = ShapeRepresentation::new(
        sub_context,
        RepresentationIdentifier::Body,
        RepresentationType::SweptSolid,
        ifc,
    )
    .add_item(
        ExtrudedAreaSolid::new(
            RectangleProfileDef::new(
                ProfileType::Area,
                wall_parameter.length,
                wall_parameter.thickness,
            )
            // center of the rectangle
            .position(
                Axis2D::new(
                    Point2D::from(DVec2::new(
                        wall_parameter.start + wall_parameter.length * 0.5,
                        wall_parameter.thickness * 0.5,
                    )),
                    ifc,
                ),
                ifc,
            ),
            // vertical wall (z-up)
            Direction3D::from(DVec3::new(0.0, 0.0, 1.0)),
            wall_parameter.height,
            ifc,
        ),
        ifc,
    );

    let shape_repr_id = ifc.data.insert_new(shape_repr);

    let x = DVec3::new(1.0, 1.0, 0.0);
    let y = DVec3::new(-1.0, 1.0, 0.0);
    let z = DVec3::new(0.0, 0.0, 1.0);
    let origin = DVec3::new(0.0, 0.0, 0.0);
    let scale = 1.0;
    let scale_y = 1.0;
    let scale_z = 1.0;

    let transformation = CartesianTransformationOperator3DnonUniform::new(
        Point3D::from(origin),
        (
            Direction3D::from(x),
            Direction3D::from(y),
            Direction3D::from(z),
        ),
        (scale, scale_y, scale_z),
        ifc,
    );

    let representation_map = RepresentationMap::new(
        Axis3D::new(Point3D::from(DVec3::new(0.0, 0.0, 0.0)), ifc),
        shape_repr_id,
        ifc,
    );

    let transformation_shape = ShapeRepresentation::new(
        sub_context,
        RepresentationIdentifier::Body,
        RepresentationType::MappedRepresentation,
        ifc,
    )
    .add_item(
        MappedItem::new(representation_map, transformation, ifc),
        ifc,
    );

    let product_shape = ProductDefinitionShape::new()
        .add_representation(shape_repr_id, ifc)
        .add_representation(transformation_shape, ifc);

    let local_placement = LocalPlacement::new(placement, ifc);

    Wall::new(id)
        .owner_history(owner_history, ifc)
        .object_placement(local_placement, ifc)
        .representation(product_shape, ifc)
}
