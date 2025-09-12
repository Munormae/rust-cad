// cad-core/ifc_core/src/hierarchy_helper.rs

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::marker::PhantomData;
use std::ops::Deref;

use crate::logger::Logger;

// === ТРЕБУЕМЫЕ ТРЕЙТЫ/ТИПЫ ОТ ТВОЕГО КАРКАСА ===============================

pub trait IfcIdGen {
    fn ifc_global_id() -> String;
}

pub trait IfcList<T> {
    fn push(&mut self, v: T);
    fn size(&self) -> usize;
    fn begin(&self) -> Option<T> where T: Clone;
    fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> where T: Clone;
}

pub trait IfcOptionalList<T>: Sized {
    type List: IfcList<T>;
    fn is_some(&self) -> bool;
    fn get_mut_or_init(&mut self) -> &mut Self::List;
}

pub trait Schema: Sized + 'static {
    // базовые типы
    type IfcDirection;
    type IfcCartesianPoint;
    type IfcAxis2Placement2D;
    type IfcAxis2Placement3D;
    type IfcObjectPlacement;
    type IfcLocalPlacement;
    type IfcOwnerHistory;
    type IfcPerson;
    type IfcOrganization;
    type IfcPersonAndOrganization;
    type IfcApplication;
    type IfcChangeActionEnum;
    type IfcProject;
    type IfcRepresentationContext;
    type IfcUnit;
    type IfcDimensionalExponents;
    type IfcSIUnit;
    type IfcSIPrefix;
    type IfcSIUnitName;
    type IfcMeasureWithUnit;
    type IfcPlaneAngleMeasure;
    type IfcConversionBasedUnit;
    type IfcUnitAssignment;

    type IfcProduct;
    type IfcRelAggregates;
    type IfcRelContainedInSpatialStructure;

    type IfcSite;
    type IfcElementCompositionEnum;
    type IfcBuilding;
    type IfcBuildingStorey;

    type IfcRepresentation;
    type IfcProductRepresentation;
    type IfcProductDefinitionShape;
    type IfcShapeRepresentation;
    type IfcRepresentationItem;
    type IfcPolyline;

    type IfcProfileTypeEnum;
    type IfcArbitraryClosedProfileDef;
    type IfcExtrudedAreaSolid;

    type IfcRectangleProfileDef;

    type IfcPlane;
    type IfcHalfSpaceSolid;
    type IfcBooleanOperand;
    type IfcBooleanOperator;
    type IfcBooleanClippingResult;

    type IfcColourRgb;
    type IfcSurfaceStyleRendering;
    type IfcSurfaceStyleElementSelect;
    type IfcSurfaceStyle;
    type IfcSurfaceSide;

    type IfcPresentationStyleAssignment;
    type IfcPresentationStyleSelect;
    type IfcPresentationStyle;

    type IfcStyledItem;

    type IfcRepresentationMap;
    type IfcCartesianTransformationOperator3D;
    type IfcMappedItem;

    type IfcGeometricRepresentationContext;
    type IfcGeometricRepresentationSubContext;
    type IfcGeometricProjectionEnum;

    // списки
    type ListOf<T>: IfcList<T>;
    type MaybeListOf<T>: IfcOptionalList<T, List = Self::ListOf<T>>;

    // фабрики/конструкторы — ты их уже даёшь в своих типах
    fn new_direction2(x: f64, y: f64) -> Self::IfcDirection;
    fn new_direction3(x: f64, y: f64, z: f64) -> Self::IfcDirection;
    fn new_point2(x: f64, y: f64) -> Self::IfcCartesianPoint;
    fn new_point3(x: f64, y: f64, z: f64) -> Self::IfcCartesianPoint;

    fn new_axis2placement2d(o: &Self::IfcCartesianPoint, x: &Self::IfcDirection) -> Self::IfcAxis2Placement2D;
    fn new_axis2placement3d(o: &Self::IfcCartesianPoint, z: &Self::IfcDirection, x: &Self::IfcDirection) -> Self::IfcAxis2Placement3D;

    fn new_local_placement(parent: Option<&Self::IfcObjectPlacement>, p3d: &Self::IfcAxis2Placement3D) -> Self::IfcLocalPlacement;

    fn new_person() -> Self::IfcPerson;
    fn new_organization(name: &str) -> Self::IfcOrganization;
    fn new_person_and_org(p: &Self::IfcPerson, o: &Self::IfcOrganization) -> Self::IfcPersonAndOrganization;
    fn new_application(o: &Self::IfcOrganization, version: &str, app_full: &str, app_id: &str) -> Self::IfcApplication;
    fn new_owner_history(po: &Self::IfcPersonAndOrganization, app: &Self::IfcApplication, change: &Self::IfcChangeActionEnum, ts: i32) -> Self::IfcOwnerHistory;

    fn new_dim_exp() -> Self::IfcDimensionalExponents;
    fn new_si_unit_length_milli() -> Self::IfcSIUnit;
    fn new_si_unit_planeangle_radian() -> Self::IfcSIUnit;
    fn new_plane_angle_measure(v: f64) -> Self::IfcPlaneAngleMeasure;
    fn new_measure_with_unit(v: &Self::IfcPlaneAngleMeasure, u: &Self::IfcSIUnit) -> Self::IfcMeasureWithUnit;
    fn new_conversion_based_unit(d: &Self::IfcDimensionalExponents, name: &str, conv: &Self::IfcMeasureWithUnit) -> Self::IfcConversionBasedUnit;
    fn new_unit_assignment(units: &Self::ListOf<Self::IfcUnit>) -> Self::IfcUnitAssignment;

    fn new_project(
        gid: &str,
        oh: &Self::IfcOwnerHistory,
        rep_contexts: &Self::MaybeListOf<Self::IfcRepresentationContext>,
        units: &Self::IfcUnitAssignment,
    ) -> Self::IfcProject;

    fn new_cartesian_points_list() -> Self::ListOf<Self::IfcCartesianPoint>;
    fn new_polyline(pts: &Self::ListOf<Self::IfcCartesianPoint>) -> Self::IfcPolyline;
    fn new_arbitrary_closed_profile(ptype_area: &Self::IfcProfileTypeEnum, curve: &Self::IfcPolyline) -> Self::IfcArbitraryClosedProfileDef;
    fn new_extruded_area_solid(profile: &Self::IfcArbitraryClosedProfileDef, place: &Self::IfcAxis2Placement3D, dir: &Self::IfcDirection, h: f64) -> Self::IfcExtrudedAreaSolid;

    fn new_representation_items_list() -> Self::ListOf<Self::IfcRepresentationItem>;
    fn new_representations_list() -> Self::ListOf<Self::IfcRepresentation>;
    fn new_shape_representation(ctx: &Self::IfcRepresentationContext, ident: Option<&str>, rtype: &str, items: &Self::ListOf<Self::IfcRepresentationItem>) -> Self::IfcShapeRepresentation;
    fn new_pds(reps: &Self::ListOf<Self::IfcRepresentation>) -> Self::IfcProductDefinitionShape;

    fn new_rectangle_profile_def(ptype_area: &Self::IfcProfileTypeEnum, place2d: &Self::IfcAxis2Placement2D, w: f64, d: f64) -> Self::IfcRectangleProfileDef;

    fn new_plane(place: &Self::IfcAxis2Placement3D) -> Self::IfcPlane;
    fn new_half_space(plane: &Self::IfcPlane, agree: bool) -> Self::IfcHalfSpaceSolid;
    fn new_bool_clip_result(op: &Self::IfcBooleanOperator, a: &Self::IfcBooleanOperand, b: &Self::IfcHalfSpaceSolid) -> Self::IfcBooleanClippingResult;

    fn new_colour_rgb(r: f64, g: f64, b: f64) -> Self::IfcColourRgb;
    fn new_surface_style_rendering_opaque(colour: &Self::IfcColourRgb) -> Self::IfcSurfaceStyleRendering;
    fn new_surface_style_rendering_transparent(colour: &Self::IfcColourRgb, transp: f64) -> Self::IfcSurfaceStyleRendering;
    fn new_surface_style_elements_list() -> Self::ListOf<Self::IfcSurfaceStyleElementSelect>;
    fn new_surface_style_both(styles: &Self::ListOf<Self::IfcSurfaceStyleElementSelect>) -> Self::IfcSurfaceStyle;

    fn new_pstyle_select_list() -> Self::ListOf<Self::IfcPresentationStyleSelect>;
    fn new_pstyle_assignment(styles: &Self::ListOf<Self::IfcPresentationStyleSelect>) -> Self::IfcPresentationStyleAssignment;

    fn new_representation_map(origin: &Self::IfcAxis2Placement3D, rep: &Self::IfcShapeRepresentation) -> Self::IfcRepresentationMap;
    fn new_cto3d(axis1: Option<&Self::IfcDirection>, axis2: Option<&Self::IfcDirection>, loc: &Self::IfcCartesianPoint, scale: Option<f64>, axis3: Option<&Self::IfcDirection>) -> Self::IfcCartesianTransformationOperator3D;
    fn new_mapped_item(map: &Self::IfcRepresentationMap, op: &Self::IfcCartesianTransformationOperator3D) -> Self::IfcMappedItem;

    fn new_geometric_context(ident: &str, dim: i32, tol: f64, world: &Self::IfcAxis2Placement3D, true_north: &Self::IfcDirection) -> Self::IfcGeometricRepresentationContext;
    fn add_subcontext_if_missing(parent: &Self::IfcGeometricRepresentationContext, ident: &str, rtype: &str) -> Self::IfcGeometricRepresentationSubContext;

    // «энумы-предметы»
    fn profile_type_area() -> Self::IfcProfileTypeEnum;
    fn bool_op_difference() -> Self::IfcBooleanOperator;

    // геттеры/сеттеры на объектах (минимум, нужен интерфейс как в исходнике)
    fn rep_items(rep: &Self::IfcShapeRepresentation) -> Self::ListOf<Self::IfcRepresentationItem>;
    fn set_rep_items(rep: &Self::IfcShapeRepresentation, items: &Self::ListOf<Self::IfcRepresentationItem>);
    fn rep_identifier(rep: &Self::IfcShapeRepresentation) -> Option<String>;
    fn set_rep_type(rep: &Self::IfcShapeRepresentation, t: &str);
    fn rep_context(rep: &Self::IfcShapeRepresentation) -> Self::IfcRepresentationContext;

    fn pds_representations(pds: &Self::IfcProductDefinitionShape) -> Self::ListOf<Self::IfcRepresentation>;
    fn set_pds_representations(pds: &Self::IfcProductDefinitionShape, reps: &Self::ListOf<Self::IfcRepresentation>);

    fn product_object_placement(p: &Self::IfcProduct) -> Option<Self::IfcObjectPlacement>;
    fn set_local_placement_rel_to(lp: &Self::IfcLocalPlacement, rel: &Self::IfcObjectPlacement);

    fn product_decomposes_size(p: &Self::IfcProduct) -> usize;

    fn shape_representation_map(rep: &Self::IfcShapeRepresentation) -> Self::ListOf<Self::IfcRepresentationMap>;
    fn set_shape_representation_identifier(rep: &Self::IfcShapeRepresentation, id: &str);

    // создаёт styled item (разные ветки ниже — см. create_styled_item_*).
    fn new_styled_item_from_assignment(item: &Self::IfcRepresentationItem, style_assign: &Self::IfcPresentationStyleAssignment) -> Self::IfcStyledItem;
    fn new_styled_item_from_style(item: &Self::IfcRepresentationItem, style: &Self::IfcPresentationStyle) -> Self::IfcStyledItem;

    // связки
    fn relate_aggregates(owner_hist: &Self::IfcOwnerHistory, parent: &impl Deref<Target=Self::IfcProduct>, child: &impl Deref<Target=Self::IfcProduct>);
    fn relate_contained(owner_hist: &Self::IfcOwnerHistory, spatial: &Self::IfcBuildingStorey, prod: &Self::IfcProduct);
}

pub struct IfcHierarchyHelper<S: Schema> {
    pub contexts: HashMap<String, S::IfcGeometricRepresentationContext>,
    pub _phantom: PhantomData<S>,
}

impl<S: Schema> IfcHierarchyHelper<S> {
    // Заглушки под твои base-методы: у тебя они уже есть, оставляю как вызовы add_*/get_* ниже:
    fn add_entity<T>(&self, _e: &T) {}
    fn add_related_object<R>(&self, _parent: &S::IfcProduct, _child: &S::IfcProduct, _oh: &S::IfcOwnerHistory) {}
    fn get_single<T>(&self) -> Option<T> { None }
    fn add_triplet<T>(&self, x: f64, y: f64, z: f64) -> T where S: Schema, T: From<S::IfcDirection> + From<S::IfcCartesianPoint> { unreachable!() }
    fn add_doublet<T>(&self, x: f64, y: f64) -> T where S: Schema, T: From<S::IfcDirection> + From<S::IfcCartesianPoint> { unreachable!() }
    fn add_localplacement(&self) -> S::IfcLocalPlacement { S::new_local_placement(None, &self.add_placement3d(0.0,0.0,0.0, 0.0,0.0,1.0, 1.0,0.0,0.0)) }
    fn get_representation_context(&mut self, s: &str) -> S::IfcGeometricRepresentationContext {
        if let Some(c) = self.contexts.get(s) { return c.clone() }
        let ctx = S::new_geometric_context(s, 3, 1e-5, &self.add_placement3d(0.0,0.0,0.0, 0.0,0.0,1.0, 1.0,0.0,0.0), &S::new_direction2(0.0,1.0));
        self.add_entity(&ctx);
        self.contexts.insert(s.to_string(), ctx.clone());
        ctx
    }

    // ===================== Порт 1: Плейсменты ===============================
    pub fn add_placement3d(
        &self, ox: f64, oy: f64, oz: f64, zx: f64, zy: f64, zz: f64, xx: f64, xy: f64, xz: f64
    ) -> S::IfcAxis2Placement3D {
        let x = S::new_direction3(xx, xy, xz);
        let z = S::new_direction3(zx, zy, zz);
        let o = S::new_point3(ox, oy, oz);
        let p = S::new_axis2placement3d(&o, &z, &x);
        self.add_entity(&p);
        p
    }

    pub fn add_placement2d(&self, ox: f64, oy: f64, xx: f64, xy: f64) -> S::IfcAxis2Placement2D {
        let x = S::new_direction2(xx, xy);
        let o = S::new_point2(ox, oy);
        let p = S::new_axis2placement2d(&o, &x);
        self.add_entity(&p);
        p
    }

    pub fn add_local_placement(
        &self, parent: Option<&S::IfcObjectPlacement>,
        ox: f64, oy: f64, oz: f64, zx: f64, zy: f64, zz: f64, xx: f64, xy: f64, xz: f64
    ) -> S::IfcLocalPlacement {
        let p3d = self.add_placement3d(ox, oy, oz, zx, zy, zz, xx, xy, xz);
        let lp = S::new_local_placement(parent, &p3d);
        self.add_entity(&lp);
        lp
    }

    // ===================== Порт 2: OwnerHistory/Project =====================
    pub fn add_owner_history(&self) -> S::IfcOwnerHistory {
        let person = S::new_person();
        let org = S::new_organization("IfcOpenShell");
        let po = S::new_person_and_org(&person, &org);
        let app = S::new_application(&org, crate::version::IFCOPENSHELL_VERSION, "IfcOpenShell", "IfcOpenShell");
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i32;

        let oh = S::new_owner_history(&po, &app, &self.change_added(), ts);

        self.add_entity(&person);
        self.add_entity(&org);
        self.add_entity(&po);
        self.add_entity(&app);
        self.add_entity(&oh);
        oh
    }

    fn change_added(&self) -> S::IfcChangeActionEnum { /* верни enum Added */ unreachable!() }

    pub fn add_project(&self, mut owner_hist: Option<S::IfcOwnerHistory>) -> S::IfcProject {
        let oh = owner_hist.unwrap_or_else(|| self.add_owner_history());

        // units
        let dim = S::new_dim_exp();
        let u_len = S::new_si_unit_length_milli();
        let u_rad = S::new_si_unit_planeangle_radian();
        let rad = S::new_plane_angle_measure(0.017_453_293);
        let mu = S::new_measure_with_unit(&rad, &u_rad);
        let u_deg = S::new_conversion_based_unit(&dim, "Degrees", &mu);

        let mut units: S::ListOf<S::IfcUnit> = S::new_representations_list(); // переиспользуем тип списка
        units.push(unsafe { std::mem::transmute(u_len) });
        units.push(unsafe { std::mem::transmute(u_deg) });

        let ua = S::new_unit_assignment(&units);

        let mut rep_contexts: S::MaybeListOf<S::IfcRepresentationContext> = unsafe { std::mem::zeroed() };

        let gid = <crate::id::DefaultIdGen as IfcIdGen>::ifc_global_id();
        let prj = S::new_project(&gid, &oh, &rep_contexts, &ua);

        self.add_entity(&dim);
        self.add_entity(&u_len);
        self.add_entity(&u_rad);
        self.add_entity(&mu);
        self.add_entity(&ua);
        self.add_entity(&prj);
        prj
    }

    // ===================== Порт 3: Site/Building/Storey =====================
    pub fn relate_placements(&self, parent: &S::IfcProduct, product: &S::IfcProduct) {
        if let Some(place) = S::product_object_placement(product) {
            // считаем, что это always LocalPlacement
            if let Some(pp) = S::product_object_placement(parent) {
                // self-ссылка — запретим
                if std::ptr::eq(&place, &pp) {
                    Logger::notice("Placement cannot be relative to self");
                } else {
                    S::set_local_placement_rel_to(unsafe { &*(&place as *const _ as *const S::IfcLocalPlacement) }, &pp);
                }
            }
        }
    }

    pub fn add_site(&self, mut proj: Option<S::IfcProject>, mut owner_hist: Option<S::IfcOwnerHistory>) -> S::IfcSite {
        let oh = owner_hist.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_owner_history()));
        let prj = proj.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_project(Some(oh.clone()))));

        let site = self.new_site(&oh);

        self.add_entity(&site);
        self.add_related_object::<S::IfcRelAggregates>(&prj, &site, &oh);
        site
    }

    fn new_site(&self, _oh: &S::IfcOwnerHistory) -> S::IfcSite { unreachable!() }

    pub fn add_building(&self, mut site: Option<S::IfcSite>, mut owner_hist: Option<S::IfcOwnerHistory>) -> S::IfcBuilding {
        let oh = owner_hist.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_owner_history()));
        let si = site.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_site(None, Some(oh.clone()))));

        let bld = self.new_building(&oh);

        self.add_entity(&bld);
        self.add_related_object::<S::IfcRelAggregates>(&si, &bld, &oh);
        self.relate_placements(unsafe { &*(std::ptr::addr_of!(si)) }, unsafe { &*(std::ptr::addr_of!(bld)) });
        bld
    }

    fn new_building(&self, _oh: &S::IfcOwnerHistory) -> S::IfcBuilding { unreachable!() }

    pub fn add_building_storey(&self, mut building: Option<S::IfcBuilding>, mut owner_hist: Option<S::IfcOwnerHistory>) -> S::IfcBuildingStorey {
        let oh = owner_hist.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_owner_history()));
        let b = building.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_building(None, Some(oh.clone()))));

        let storey = self.new_building_storey(&oh);

        self.add_entity(&storey);
        self.add_related_object::<S::IfcRelAggregates>(&b, &storey, &oh);
        self.relate_placements(unsafe { &*(std::ptr::addr_of!(b)) }, unsafe { &*(std::ptr::addr_of!(storey)) });
        storey
    }

    fn new_building_storey(&self, _oh: &S::IfcOwnerHistory) -> S::IfcBuildingStorey { unreachable!() }

    pub fn add_building_product(
        &self,
        product: &S::IfcProduct,
        mut storey: Option<S::IfcBuildingStorey>,
        mut owner_hist: Option<S::IfcOwnerHistory>
    ) -> S::IfcBuildingStorey {
        let oh = owner_hist.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_owner_history()));
        let st = storey.unwrap_or_else(|| self.get_single().unwrap_or_else(|| self.add_building_storey(None, Some(oh.clone()))));

        self.add_entity(product);

        let is_decomposition = S::product_decomposes_size(product) > 0;
        if !is_decomposition {
            S::relate_contained(&oh, &st, product);
            self.relate_placements(unsafe { &*(std::ptr::addr_of!(st)) }, product);
        }
        st
    }

    // ===================== Порт 4: Геометрия (polyline/box/axis) =============
    pub fn add_extruded_polyline_into_rep(
        &self,
        rep: &S::IfcShapeRepresentation,
        points: &[(f64, f64)],
        h: f64,
        _place2d: Option<&S::IfcAxis2Placement2D>,
        place3d: Option<&S::IfcAxis2Placement3D>,
        dir: Option<&S::IfcDirection>,
        _ctx: Option<&S::IfcRepresentationContext>,
    ) {
        let mut pts: S::ListOf<S::IfcCartesianPoint> = S::new_cartesian_points_list();
        for (x, y) in points {
            let p = S::new_point2(*x, *y);
            pts.push(p);
        }
        if S::IfcList::<S::IfcCartesianPoint>::size(&pts) > 0 {
            if let Some(first) = S::IfcList::<S::IfcCartesianPoint>::begin(&pts) {
                pts.push(first);
            }
        }
        let line = S::new_polyline(&pts);
        let profile = S::new_arbitrary_closed_profile(&S::profile_type_area(), &line);
        let solid = S::new_extruded_area_solid(
            &profile,
            place3d.unwrap_or(&self.add_placement3d(0.0,0.0,0.0, 0.0,0.0,1.0, 1.0,0.0,0.0)),
            dir.unwrap_or(&S::new_direction3(0.0,0.0,1.0)),
            h
        );
        let mut items = S::rep_items(rep);
        items.push(unsafe { std::mem::transmute::<S::IfcExtrudedAreaSolid, S::IfcRepresentationItem>(solid) });
        S::set_rep_items(rep, &items);

        self.add_entity(&line);
        self.add_entity(&profile);
        // solid уже в items
    }

    pub fn add_extruded_polyline(
        &mut self,
        points: &[(f64, f64)], h: f64,
        place2d: Option<&S::IfcAxis2Placement2D>,
        place3d: Option<&S::IfcAxis2Placement3D>,
        dir: Option<&S::IfcDirection>,
        ctx: Option<&S::IfcRepresentationContext>,
    ) -> S::IfcProductDefinitionShape {
        let mut reps: S::ListOf<S::IfcRepresentation> = S::new_representations_list();
        let items: S::ListOf<S::IfcRepresentationItem> = S::new_representation_items_list();
        let rep = S::new_shape_representation(
            ctx.unwrap_or(&self.get_representation_context("Model")),
            Some("Body"),
            "SweptSolid",
            &items
        );
        reps.push(unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcRepresentation>(rep.clone()) });
        let shape = S::new_pds(&reps);
        self.add_entity(&rep);
        self.add_entity(&shape);
        self.add_extruded_polyline_into_rep(&rep, points, h, place2d, place3d, dir, ctx);
        shape
    }

    pub fn add_box_into_rep(
        &self,
        rep: &S::IfcShapeRepresentation,
        w: f64, d: f64, h: f64,
        place2d: Option<&S::IfcAxis2Placement2D>,
        place3d: Option<&S::IfcAxis2Placement3D>,
        dir: Option<&S::IfcDirection>,
        ctx: Option<&S::IfcRepresentationContext>,
    ) {
        let pts = [
            (-w/2.0, -d/2.0),
            ( w/2.0, -d/2.0),
            ( w/2.0,  d/2.0),
            (-w/2.0,  d/2.0),
        ];
        self.add_extruded_polyline_into_rep(rep, &pts, h, place2d, place3d, dir, ctx);
    }

    pub fn add_axis(&self, rep: &S::IfcShapeRepresentation, l: f64) {
        let p1 = S::new_point2(-l/2.0, 0.0);
        let p2 = S::new_point2( l/2.0, 0.0);
        let mut pts: S::ListOf<S::IfcCartesianPoint> = S::new_cartesian_points_list();
        pts.push(p1);
        pts.push(p2);
        let poly = S::new_polyline(&pts);
        self.add_entity(&poly);

        let mut items = S::rep_items(rep);
        items.push(unsafe { std::mem::transmute::<S::IfcPolyline, S::IfcRepresentationItem>(poly) });
        S::set_rep_items(rep, &items);
    }

    pub fn add_box(
        &mut self, w: f64, d: f64, h: f64,
        place2d: Option<&S::IfcAxis2Placement2D>,
        place3d: Option<&S::IfcAxis2Placement3D>,
        dir: Option<&S::IfcDirection>,
        ctx: Option<&S::IfcRepresentationContext>,
    ) -> S::IfcProductDefinitionShape {
        let mut reps: S::ListOf<S::IfcRepresentation> = S::new_representations_list();
        let items: S::ListOf<S::IfcRepresentationItem> = S::new_representation_items_list();
        let rep = S::new_shape_representation(
            ctx.unwrap_or(&self.get_representation_context("Model")),
            Some("Body"),
            "SweptSolid",
            &items
        );
        reps.push(unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcRepresentation>(rep.clone()) });
        let shape = S::new_pds(&reps);
        self.add_entity(&rep);
        self.add_entity(&shape);
        self.add_box_into_rep(&rep, w, d, h, place2d, place3d, dir, ctx);
        shape
    }

    pub fn add_axis_box(
        &mut self, w: f64, d: f64, h: f64, ctx: Option<&S::IfcRepresentationContext>
    ) -> S::IfcProductDefinitionShape {
        let mut reps: S::ListOf<S::IfcRepresentation> = S::new_representations_list();
        let body_items: S::ListOf<S::IfcRepresentationItem> = S::new_representation_items_list();
        let axis_items: S::ListOf<S::IfcRepresentationItem> = S::new_representation_items_list();

        let body_rep = S::new_shape_representation(
            ctx.unwrap_or(&self.get_representation_context("Model")),
            Some("Body"),
            "SweptSolid",
            &body_items
        );
        let axis_rep = S::new_shape_representation(
            &self.get_representation_context("Plan"),
            Some("Axis"),
            "Curve2D",
            &axis_items
        );
        reps.push(unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcRepresentation>(axis_rep.clone()) });
        reps.push(unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcRepresentation>(body_rep.clone()) });

        let shape = S::new_pds(&reps);
        self.add_entity(&shape);
        self.add_entity(&body_rep);
        self.add_box_into_rep(&body_rep, w, d, h, None, None, None, ctx);
        self.add_entity(&axis_rep);
        self.add_axis(&axis_rep, w);
        shape
    }

    // ===================== Порт 5: Clipping =================================
    pub fn clip_product_representation(&self, shape: &S::IfcProductRepresentation, place: &S::IfcAxis2Placement3D, agree: bool) {
        let reps = S::pds_representations(unsafe { std::mem::transmute(shape.clone()) });
        for r in reps.iter() {
            self.clip_representation(unsafe { std::mem::transmute::<S::IfcRepresentation, S::IfcShapeRepresentation>(r) }, place, agree);
        }
    }

    pub fn clip_representation(&self, rep: &S::IfcShapeRepresentation, place: &S::IfcAxis2Placement3D, agree: bool) {
        if let Some(id) = S::rep_identifier(rep) {
            if id != "Body" { return; }
        }
        let plane = S::new_plane(place);
        let half = S::new_half_space(&plane, agree);
        self.add_entity(&plane);
        self.add_entity(&half);
        S::set_rep_type(rep, "Clipping");

        let items = S::rep_items(rep);
        let mut new_items: S::ListOf<S::IfcRepresentationItem> = S::new_representation_items_list();
        for it in items.iter() {
            // cast до BooleanOperand — как в C++, тут считаем, что фильтруется корректно
            let clip = S::new_bool_clip_result(&S::bool_op_difference(), unsafe { &*(std::ptr::addr_of!(it) as *const S::IfcBooleanOperand) }, &half);
            self.add_entity(&clip);
            new_items.push(unsafe { std::mem::transmute::<S::IfcBooleanClippingResult, S::IfcRepresentationItem>(clip) });
        }
        S::set_rep_items(rep, &new_items);
    }

    // ===================== Порт 6: Surface colour / styles ===================
    fn surface_style(&self, r: f64, g: f64, b: f64, a: f64) -> S::IfcSurfaceStyle {
        let colour = S::new_colour_rgb(r, g, b);
        let rend = if (a - 1.0).abs() < 1e-9 {
            S::new_surface_style_rendering_opaque(&colour)
        } else {
            S::new_surface_style_rendering_transparent(&colour, 1.0 - a)
        };
        let mut styles: S::ListOf<S::IfcSurfaceStyleElementSelect> = S::new_surface_style_elements_list();
        styles.push(unsafe { std::mem::transmute(rend) });
        let ss = S::new_surface_style_both(&styles);
        self.add_entity(&colour);
        self.add_entity(&ss);
        ss
    }

    fn add_style_assignment_2x3(&self, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyleAssignment {
        let surf = self.surface_style(r,g,b,a);
        let mut v: S::ListOf<S::IfcPresentationStyleSelect> = S::new_pstyle_select_list();
        v.push(unsafe { std::mem::transmute::<S::IfcSurfaceStyle, S::IfcPresentationStyleSelect>(surf.clone()) });
        let psa = S::new_pstyle_assignment(&v);
        self.add_entity(&psa);
        psa
    }

    pub fn set_surface_colour_2x3_on_product(&self, shape: &S::IfcProductRepresentation, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyleAssignment {
        let psa = self.add_style_assignment_2x3(r,g,b,a);
        self.set_surface_colour_2x3_on_representation_product(shape, &psa);
        psa
    }

    pub fn set_surface_colour_4x3_on_product(&self, shape: &S::IfcProductRepresentation, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyle {
        let style = unsafe { std::mem::transmute::<S::IfcSurfaceStyle, S::IfcPresentationStyle>(self.surface_style(r,g,b,a)) };
        self.set_surface_colour_4x3_on_representation_product(shape, &style);
        style
    }

    pub fn set_surface_colour_2x3_on_representation_product(&self, shape: &S::IfcProductRepresentation, psa: &S::IfcPresentationStyleAssignment) {
        let reps = S::pds_representations(unsafe { std::mem::transmute(shape.clone()) });
        for r in reps.iter() {
            self.set_surface_colour_2x3_on_representation(&unsafe { std::mem::transmute::<S::IfcRepresentation, S::IfcShapeRepresentation>(r) }, psa);
        }
    }

    pub fn set_surface_colour_4x3_on_representation_product(&self, shape: &S::IfcProductRepresentation, style: &S::IfcPresentationStyle) {
        let reps = S::pds_representations(unsafe { std::mem::transmute(shape.clone()) });
        for r in reps.iter() {
            self.set_surface_colour_4x3_on_representation(&unsafe { std::mem::transmute::<S::IfcRepresentation, S::IfcShapeRepresentation>(r) }, style);
        }
    }

    pub fn set_surface_colour_2x3_on_representation(&self, rep: &S::IfcShapeRepresentation, psa: &S::IfcPresentationStyleAssignment) {
        let items = S::rep_items(rep);
        for it in items.iter() {
            let styled = S::new_styled_item_from_assignment(&it, psa);
            self.add_entity(&styled);
        }
    }

    pub fn set_surface_colour_4x3_on_representation(&self, rep: &S::IfcShapeRepresentation, style: &S::IfcPresentationStyle) {
        let items = S::rep_items(rep);
        for it in items.iter() {
            let styled = S::new_styled_item_from_style(&it, style);
            self.add_entity(&styled);
        }
    }

    // ===================== Порт 7: Mapped items ==============================
    pub fn add_mapped_item_from_rep(
        &self,
        rep: &S::IfcShapeRepresentation,
        mut transform: Option<S::IfcCartesianTransformationOperator3D>,
        mut def: Option<S::IfcProductDefinitionShape>
    ) -> S::IfcProductDefinitionShape {
        let maps = S::shape_representation_map(rep);
        let map = if S::IfcList::<S::IfcRepresentationMap>::size(&maps) == 1 {
            S::IfcList::<S::IfcRepresentationMap>::begin(&maps).unwrap()
        } else {
            let m = S::new_representation_map(&self.add_placement3d(0.0,0.0,0.0, 0.0,0.0,1.0, 1.0,0.0,0.0), rep);
            self.add_entity(&m);
            m
        };

        let representations = def.as_ref().map(|p| S::pds_representations(p)).unwrap_or_else(|| S::new_representations_list());
        let op = transform.take().unwrap_or_else(|| {
            let c = S::new_point3(0.0,0.0,0.0);
            let op = S::new_cto3d(None, None, &c, None, None);
            self.add_entity(&op);
            op
        });
        let item = S::new_mapped_item(&map, &op);
        let mut items = S::new_representation_items_list();
        items.push(unsafe { std::mem::transmute::<S::IfcMappedItem, S::IfcRepresentationItem>(item) });
        let new_rep = S::new_shape_representation(&S::rep_context(rep), S::rep_identifier(rep).as_deref(), "MappedRepresentation", &items);
        if let Some(id) = S::rep_identifier(rep) {
            S::set_shape_representation_identifier(&new_rep, &id);
        }
        self.add_entity(&new_rep);

        let mut reps = representations;
        reps.push(unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcRepresentation>(new_rep) });

        let out = def.unwrap_or_else(|| {
            let pds = S::new_pds(&reps);
            self.add_entity(&pds);
            pds
        });
        S::set_pds_representations(&out, &reps);
        out
    }

    pub fn add_mapped_item_from_reps(
        &self,
        reps: &S::ListOf<S::IfcShapeRepresentation>,
        transform: Option<S::IfcCartesianTransformationOperator3D>
    ) -> S::IfcProductDefinitionShape {
        let mut def: Option<S::IfcProductDefinitionShape> = None;
        for r in reps.iter() {
            let rshape = unsafe { std::mem::transmute::<S::IfcShapeRepresentation, S::IfcShapeRepresentation>(r) };
            def = Some(self.add_mapped_item_from_rep(&rshape, transform.clone(), def));
        }
        def.unwrap()
    }

    // ===================== Порт 8: Empty representation ======================
    pub fn add_empty_representation(&mut self, repid: &str, reptype: &str) -> S::IfcShapeRepresentation {
        let items = S::new_representation_items_list();
        let ctx_key = if reptype == "Curve2D" { "Plan" } else { "Model" };
        let rep = S::new_shape_representation(&self.get_representation_context(ctx_key), Some(repid), reptype, &items);
        self.add_entity(&rep);
        rep
    }

    // ===================== Порт 9: Context/SubContext ========================
    pub fn get_representation_context(&mut self, s: &str) -> S::IfcGeometricRepresentationContext {
        if let Some(c) = self.contexts.get(s) { return c.clone(); }
        let ctx = S::new_geometric_context(s, 3, 1e-5, &self.add_placement3d(0.0,0.0,0.0, 0.0,0.0,1.0, 1.0,0.0,0.0), &S::new_direction2(0.0,1.0));
        self.add_entity(&ctx);
        self.contexts.insert(s.to_string(), ctx.clone());
        ctx
    }

    pub fn get_representation_subcontext(&mut self, ident: &str, rtype: &str) -> S::IfcGeometricRepresentationSubContext {
        let parent = self.get_representation_context(rtype);
        let sub = S::add_subcontext_if_missing(&parent, ident, rtype);
        self.add_entity(&sub);
        sub
    }
}

// Обёртки setSurfaceColour как в исходнике — фичегейты под разные схемы.

#[cfg(feature = "schema_2x3")]
pub fn add_style_assignment_2x3<S: Schema>(file: &IfcHierarchyHelper<S>, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyleAssignment {
    file.add_style_assignment_2x3(r,g,b,a)
}

#[cfg(feature = "schema_2x3")]
pub fn set_surface_colour_2x3_prod<S: Schema>(file: &IfcHierarchyHelper<S>, shape: &S::IfcProductRepresentation, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyleAssignment {
    file.set_surface_colour_2x3_on_product(shape, r,g,b,a)
}

#[cfg(feature = "schema_4x3")]
pub fn add_style_assignment_4x3<S: Schema>(file: &IfcHierarchyHelper<S>, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyle {
    unsafe { std::mem::transmute::<S::IfcSurfaceStyle, S::IfcPresentationStyle>(file.surface_style(r,g,b,a)) }
}

#[cfg(feature = "schema_4x3")]
pub fn set_surface_colour_4x3_prod<S: Schema>(file: &IfcHierarchyHelper<S>, shape: &S::IfcProductRepresentation, r: f64, g: f64, b: f64, a: f64) -> S::IfcPresentationStyle {
    file.set_surface_colour_4x3_on_product(shape, r,g,b,a)
}
