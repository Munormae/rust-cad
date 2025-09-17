use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    FromTopology(#[from] truck_topology::errors::Error),
    #[error("cannot attach a plane to a wire that is not on one plane.")]
    WireNotInOnePlane,
    #[error("The wires must contain the same number of edges to create a homotopy.")]
    NotSameNumberOfEdges,
}

#[test]
fn print_messages() {
    use std::io::Write;
    writeln!(
        &mut std::io::stderr(),
        "****** test of the expressions of error messages ******\n"
    )
    .unwrap();
    writeln!(
        &mut std::io::stderr(),
        "{}\n",
        Error::FromTopology(truck_topology::errors::Error::SameVertex)
    )
    .unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::WireNotInOnePlane).unwrap();
    writeln!(
        &mut std::io::stderr(),
        "*******************************************************"
    )
    .unwrap();
}
