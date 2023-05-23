use std::marker::PhantomData;

use halo2_proofs::circuit::Chip;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::halo2curves::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Selector;
use halo2_proofs::poly::Rotation;

/// Three advices and two additions
#[derive(Clone, Debug)]
pub struct ECConfig<F> {
    pub(crate) a: Column<Advice>,
    pub(crate) b: Column<Advice>,
    pub(crate) c: Column<Advice>,
    pub(crate) q1: Selector,
    pub(crate) q2: Selector,

    /// the parameter b in the curve equation x^3 + b = y^2
    pub(crate) curve_param_b: F,
}
