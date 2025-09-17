use crate::topo_impls::*;
use crate::topo_traits::*;
use truck_topology::*;

impl<P, C, T, Pc, Cc> Sweep<T, Pc, Cc, Edge<P, C>> for Vertex<P>
where
    P: Clone,
    C: Clone,
    T: GeometricMapping<P> + Copy,
    Pc: Connector<P, C>,
{
    fn sweep(&self, trans: T, connect_points: Pc, _: Cc) -> Edge<P, C> {
        let v = self.mapped(trans.mapping());
        connect_vertices(self, &v, connect_points.connector())
    }
}

impl<P, C, S, T, Pc, Cc> Sweep<T, Pc, Cc, Face<P, C, S>> for Edge<P, C>
where
    P: Clone,
    C: Clone,
    S: Clone,
    T: GeometricMapping<P> + GeometricMapping<C> + Copy,
    Pc: Connector<P, C>,
    Cc: Connector<C, S>,
{
    fn sweep(&self, trans: T, point_connector: Pc, curve_connector: Cc) -> Face<P, C, S> {
        let point_mapping = GeometricMapping::<P>::mapping(trans);
        let curve_mapping = GeometricMapping::<C>::mapping(trans);
        let connect_points = point_connector.connector();
        let connect_curves = curve_connector.connector();
        let edge = self.mapped(point_mapping, curve_mapping);
        connect_edges(self, &edge, connect_points, connect_curves)
    }
}

impl<P, C, S, T, Pc, Cc> Sweep<T, Pc, Cc, Shell<P, C, S>> for Wire<P, C>
where
    P: Clone,
    C: Clone,
    S: Clone,
    T: GeometricMapping<P> + GeometricMapping<C> + Copy,
    Pc: Connector<P, C>,
    Cc: Connector<C, S>,
{
    fn sweep(&self, trans: T, point_connector: Pc, curve_connector: Cc) -> Shell<P, C, S> {
        let point_mapping = GeometricMapping::<P>::mapping(trans);
        let curve_mapping = GeometricMapping::<C>::mapping(trans);
        let connect_points = point_connector.connector();
        let connect_curves = curve_connector.connector();
        let wire = self.mapped(point_mapping, curve_mapping);
        connect_wires(self, &wire, connect_points, connect_curves).collect()
    }
}

impl<P, C, S, T, Pc, Cc> Sweep<T, Pc, Cc, Solid<P, C, S>> for Face<P, C, S>
where
    P: Clone,
    C: Clone,
    S: Clone,
    T: GeometricMapping<P> + GeometricMapping<C> + GeometricMapping<S> + Copy,
    Pc: Connector<P, C>,
    Cc: Connector<C, S>,
{
    fn sweep(&self, trans: T, point_connector: Pc, curve_connector: Cc) -> Solid<P, C, S> {
        let point_mapping = GeometricMapping::<P>::mapping(trans);
        let curve_mapping = GeometricMapping::<C>::mapping(trans);
        let surface_mapping = GeometricMapping::<S>::mapping(trans);
        let connect_points = point_connector.connector();
        let connect_curves = curve_connector.connector();
        let mut shell = shell![self.inverse()];
        let seiling = self.mapped(point_mapping, curve_mapping, surface_mapping);
        let biter0 = self.boundary_iters().into_iter().flatten();
        let biter1 = seiling.boundary_iters().into_iter().flatten();
        shell.extend(connect_raw_wires(
            biter0,
            biter1,
            connect_points,
            connect_curves,
        ));
        shell.push(seiling);
        Solid::debug_new(vec![shell])
    }
}

impl<P, C, S, T, Pc, Cc> Sweep<T, Pc, Cc, Vec<Result<Solid<P, C, S>>>> for Shell<P, C, S>
where
    P: Clone,
    C: Clone,
    S: Clone,
    T: GeometricMapping<P> + GeometricMapping<C> + GeometricMapping<S> + Copy,
    Pc: Connector<P, C>,
    Cc: Connector<C, S>,
{
    fn sweep(
        &self,
        trans: T,
        point_connector: Pc,
        curve_connector: Cc,
    ) -> Vec<Result<Solid<P, C, S>>> {
        let point_mapping = GeometricMapping::<P>::mapping(trans);
        let curve_mapping = GeometricMapping::<C>::mapping(trans);
        let surface_mapping = GeometricMapping::<S>::mapping(trans);
        let connect_points = point_connector.connector();
        let connect_curves = curve_connector.connector();
        self.connected_components()
            .into_iter()
            .map(move |shell| {
                let mut bdry = Shell::new();
                let mut seiling = shell.mapped(&point_mapping, &curve_mapping, &surface_mapping);
                bdry.extend(shell.face_iter().map(|face| face.inverse()));
                let bdries0 = shell.extract_boundaries();
                let bdries1 = seiling.extract_boundaries();
                let biter0 = bdries0.iter().flat_map(Wire::edge_iter);
                let biter1 = bdries1.iter().flat_map(Wire::edge_iter);
                bdry.extend(connect_wires(
                    biter0,
                    biter1,
                    &connect_points,
                    &connect_curves,
                ));
                bdry.append(&mut seiling);
                Solid::try_new(vec![bdry])
            })
            .collect()
    }
}
