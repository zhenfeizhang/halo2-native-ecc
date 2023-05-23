// // to remove
// #![allow(unused_imports)]
// #![allow(unused_variables)]

mod chip;
mod config;
mod ec_gates;
mod ec_structs;
#[cfg(test)]
mod tests;
mod util;

pub use chip::ECChip;
pub use config::ECConfig;
pub use ec_structs::AssignedECPoint;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Region;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Error;

pub trait NativeECOps<F: Field> {
    type Config;
    type ECPoint: CurveAffine;
    type AssignedECPoint;

    /// Loads an ecpoint (x, y) into the circuit as a private input.
    /// Constraints (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b); and use column c to enforce point is on curve
    fn load_private(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        let p = self.load_private_unchecked(region, config, p, offset)?;
        self.enforce_on_curve(region, config, &p, offset)?;
        Ok(p)
    }

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b)
    fn load_private_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// For an input pair (x, y), checks if the point is on curve.
    /// Return boolean.
    fn is_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// For an input pair (x, y), enforces the point is on curve.
    fn enforce_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<(), Error> {
        let res = self.is_on_curve(region, config, p, offset)?;
        region.constrain_constant(res.cell(), F::ZERO)
    }

    /// Return p3 = p1 + p2.
    /// Also enforces p1 and p2 are on curve.
    fn add_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        p2: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p3 = p1 + p2
    fn add_assigned_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        p2: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p2 = p1 + p1
    /// Also enforces p1 is on curve.
    fn double_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p2 = p1 + p1
    fn double_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Decompose a scalar into a vector of boolean Cells
    fn decompose_scalar(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>;

    fn mul_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// summation
    fn summation(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<
        (
            Vec<AssignedCell<F, F>>, // cells allocated for inputs
            AssignedCell<F, F>,      // cells allocated for sum
        ),
        Error,
    >;

    /// Input x1, y1, x2, y2, x3, y3
    /// Assert that
    /// - x3 = x1 + y1 + x2 + y2 + y3
    /// - x1, y1, x2, y2 are all binary
    fn partial_bit_decomp(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>;

    /// Pad the row with empty cells.
    fn pad(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        offset: &mut usize,
    ) -> Result<(), Error>;
}
