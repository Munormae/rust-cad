use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<V: std::fmt::Debug = crate::StandardVertex> {
    #[error("The index {0:?} is out of range.")]
    OutOfRange(V),

    #[error("This mesh has no normal vectors.")]
    NoNormal,

    #[error("The lengths of point vector, uvdivisions, normal vector are incompatible.")]
    DifferentLengthArrays,

    #[error("This 2-dim array is irregular.")]
    IrregularArray,

    #[error("This division vector is unsorted.")]
    UnsortedDivision,

    #[error(transparent)]
    FromIO(#[from] std::io::Error),
}

impl From<std::num::ParseFloatError> for Error {
    fn from(error: std::num::ParseFloatError) -> Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error).into()
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error).into()
    }
}
