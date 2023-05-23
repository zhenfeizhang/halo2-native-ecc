use std::marker::PhantomData;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Selector;

/// Three advices and two additions
#[derive(Clone, Debug)]
pub struct ECConfig<C, F>
where
    // the embedded curve, i.e., Grumpkin
    C: CurveAffine<Base = F>,
    // the field for circuit, i.e., BN::Scalar
    F: Field,
{
    // witnesses
    pub(crate) a: Column<Advice>,
    pub(crate) b: Column<Advice>,

    // selectors
    pub(crate) q1: Selector,
    pub(crate) q2: Selector,

    pub(crate) _phantom: PhantomData<C>,
}
