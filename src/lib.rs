// to remove
#![allow(unused_imports)]
#![allow(unused_variables)]

mod chip;
mod config;
mod ec_structs;
#[cfg(test)]
mod tests;

pub use ec_structs::AssignedECPoint;
pub use ec_structs::ECPoint;

use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::Region;
use halo2_proofs::halo2curves::FieldExt;
use halo2_proofs::plonk::Error;

pub trait NativeECOps<F: FieldExt> {
    type Config;
    type ECPoint;
    type AssignedECPoint;

    /// Loads an ecpoint (x, y) into the circuit as a private input.
    /// Constraints (x, y) is on curve.
    fn load_private(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        self.enforce_on_curve(region, config, p, offset)?;
        self.load_private_unchecked(region, config, p)
    }

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    fn load_private_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// For an input pair (x, y), checks if the point is on curve.
    /// Return boolean.
    fn is_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// For an input pair (x, y), enforces the point is on curve.
    fn enforce_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: usize,
    ) -> Result<(), Error> {
        let res = self.is_on_curve(region, config, p, offset)?;
        region.constrain_constant(res.cell(), F::zero())
    }
}
