use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::FieldExt;

#[derive(Debug, Copy, Clone)]
pub struct ECPoint<F: FieldExt> {
    pub(crate) x: Value<F>,
    pub(crate) y: Value<F>,
}

#[derive(Debug, Clone)]
pub struct AssignedECPoint<F: FieldExt> {
    pub(crate) x: AssignedCell<F, F>,
    pub(crate) y: AssignedCell<F, F>,
}
