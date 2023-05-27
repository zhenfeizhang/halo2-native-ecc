use std::marker::PhantomData;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::halo2curves::CurveAffine;

use crate::util::leak;

#[derive(Debug, Clone)]
pub struct AssignedECPoint<C, F>
where
    C: CurveAffine<Base = F>,
    F: Field,
{
    pub(crate) x: AssignedCell<F, F>,
    pub(crate) y: AssignedCell<F, F>,
    // the index of the ec point: the two cells is always stored in a same row
    pub(crate) offset: usize,
    _phantom: PhantomData<C>,
}

impl<C, F> AssignedECPoint<C, F>
where
    C: CurveAffine<Base = F>,
    F: Field,
{
    pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>, offset: usize) -> Self {
        Self {
            x,
            y,
            offset,
            _phantom: PhantomData::default(),
        }
    }

    pub fn witness(&self) -> C {
        C::from_xy(leak(&self.x.value()), leak(&self.y.value())).unwrap()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}
