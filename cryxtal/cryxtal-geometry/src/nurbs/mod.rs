use cryxtal_base::cgmath64::control_point::ControlPoint;
use cryxtal_geotrait::ToSameGeometry;
use cryxtal_base::tolerance::TOLERANCE;
use crate::base::SelfSameGeometry;
use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use cryxtal_base::cgmath64::cgmath::BaseFloat;
}

#[derive(Clone, PartialEq, Debug, Default, Serialize)]
pub struct KnotVec(Vec<f64>);

#[derive(Clone, PartialEq, Debug, Serialize, SelfSameGeometry)]
pub struct BSplineCurve<P> {
    knot_vec: KnotVec,      // the knot vector
    control_points: Vec<P>, // the indices of control points
}

#[derive(Clone, PartialEq, Debug, Serialize, SelfSameGeometry)]
pub struct BSplineSurface<P> {
    knot_vecs: (KnotVec, KnotVec),
    control_points: Vec<Vec<P>>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, SelfSameGeometry)]
pub struct NurbsCurve<V>(BSplineCurve<V>);

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, SelfSameGeometry)]
pub struct NurbsSurface<V>(BSplineSurface<V>);

mod bspcurve;
mod bspsurface;
mod knot_vec;
mod nurbscurve;
mod nurbssurface;

#[doc(hidden)]
#[inline(always)]
pub const fn inv_or_zero(delta: f64) -> f64 {
    if delta.abs() <= TOLERANCE {
        0.0
    } else {
        1.0 / delta
    }
}

mod gaussian_elimination {
    use cryxtal_base::cgmath64::cgmath::BaseFloat;

    pub fn gaussian_elimination<S: BaseFloat>(matrix: &mut [Vec<S>]) -> Option<Vec<S>> {
        let size = matrix.len();
        if size != matrix[0].len() - 1 {
            return None;
        }

        for i in 0..size - 1 {
            for j in i..size - 1 {
                echelon(matrix, i, j);
            }
        }

        for i in (1..size).rev() {
            eliminate(matrix, i);
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..size {
            if matrix[i][i].is_zero() {
                return None;
            }
        }

        Some((0..size).map(|i| matrix[i][size] / matrix[i][i]).collect())
    }

    fn echelon<S: BaseFloat>(matrix: &mut [Vec<S>], i: usize, j: usize) {
        let size = matrix.len();
        if matrix[i][i] != S::zero() {
            let factor = matrix[j + 1][i] / matrix[i][i];
            (i..size + 1).for_each(|k| {
                matrix[j + 1][k] = matrix[j + 1][k] - factor * matrix[i][k];
            });
        }
    }

    fn eliminate<S: BaseFloat>(matrix: &mut [Vec<S>], i: usize) {
        let size = matrix.len();
        if matrix[i][i] != S::zero() {
            for j in (1..i + 1).rev() {
                let factor = matrix[j - 1][i] / matrix[i][i];
                for k in (0..size + 1).rev() {
                    matrix[j - 1][k] = matrix[j - 1][k] - factor * matrix[i][k];
                }
            }
        }
    }
}
