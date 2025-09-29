use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error("Two same vertices cannot construct an edge.")]
    SameVertex,
    #[error("This wire is empty.")]
    EmptyWire,
    #[error("This wire is not closed.")]
    NotClosedWire,
    #[error("This wire is not simple.")]
    NotSimpleWire,
    #[error("Some wires has a shared vertex.")]
    NotDisjointWires,
    #[error("This shell is empty.")]
    EmptyShell,
    #[error("This shell is not connected.")]
    NotConnected,
    #[error("This shell is not oriented and closed.")]
    NotClosedShell,
    #[error("This shell is not a manifold.")]
    NotManifold,
}

#[test]
fn print_messages() {
    use std::io::Write;
    writeln!(
        &mut std::io::stderr(),
        "****** test of the expressions of error messages ******\n"
    )
    .unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::SameVertex).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::EmptyWire).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotClosedWire).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotSimpleWire).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotDisjointWires).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::EmptyShell).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotConnected).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotClosedShell).unwrap();
    writeln!(&mut std::io::stderr(), "{}\n", Error::NotManifold).unwrap();
    writeln!(
        &mut std::io::stderr(),
        "*******************************************************"
    )
    .unwrap();
}
