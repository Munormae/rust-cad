use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Error)]
pub enum Error {
    #[error("This knot vector consists single value.")]
    ZeroRange,
    #[error("Cannot concat two knot vectors whose the back of the first and the front of the second are different.
the back of the first knot vector: {0}
the front of the second knot vector: {1}")]
    DifferentBackFront(f64, f64),

    #[error("This knot vector is not clamped.")]
    NotClampedKnotVector,

    #[error("This knot vector is not sorted.")]
    NotSortedVector,

    #[error(
        "This knot vector is too short compared to the degree.
the length of knot_vec: {0}
the degree: {1}"
    )]
    TooLargeDegree(usize, usize),

    #[error("The {0}th knot in this knot vector cannot be removed.")]
    CannotRemoveKnot(usize),

    #[error("The control point must not be empty.")]
    EmptyControlPoints,

    #[error(
        "The knot vector must be more than the control points.
the length of knot_vec: {0}
the number of control points: {1}"
    )]
    TooShortKnotVector(usize, usize),

    #[error("The number of control points is irregular")]
    IrregularControlPoints,

    #[error("The vector of control points and the one of weights have different length.")]
    DifferentLength,

    #[error("Gaussian elimination is failed.")]
    GaussianEliminationFailure,
}

#[test]
#[rustfmt::skip]
fn print_messages() {
    use std::io::Write;
    let stderr = &mut std::io::stderr();
    writeln!(stderr, "****** test of the expressions of error messages ******\n").unwrap();
    writeln!(stderr, "{}\n", Error::ZeroRange).unwrap();
    writeln!(stderr, "{}\n", Error::DifferentBackFront(0.0, 1.0)).unwrap();
    writeln!(stderr, "{}\n", Error::NotClampedKnotVector).unwrap();
    writeln!(stderr, "{}\n", Error::NotSortedVector).unwrap();
    writeln!(stderr, "{}\n", Error::TooLargeDegree(1, 2)).unwrap();
    writeln!(stderr, "{}\n", Error::CannotRemoveKnot(7)).unwrap();
    writeln!(stderr, "{}\n", Error::EmptyControlPoints).unwrap();
    writeln!(stderr, "{}\n", Error::TooShortKnotVector(1, 2)).unwrap();
    writeln!(stderr, "{}\n", Error::IrregularControlPoints).unwrap();
    writeln!(stderr, "*******************************************************").unwrap();
}
