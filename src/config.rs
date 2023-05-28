use std::marker::PhantomData;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Selector;
use halo2_proofs::plonk::VirtualCells;
use halo2_proofs::poly::Rotation;

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
    pub(crate) q_ec_enable: Selector, // ec is enabled
    pub(crate) q1: Selector,          // ec conditional add
    pub(crate) q2: Selector,          // ec double
    pub(crate) q3: Selector,          // ec on curve

    pub(crate) _phantom: PhantomData<C>,
}

impl<C, F> ECConfig<C, F>
where
    C: CurveAffine<Base = F>,
    F: PrimeField,
{
    pub(crate) fn conditional_ec_add_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let one = Expression::Constant(F::ONE);
        // FIXME: currently hardcoded for Grumpkin curve
        let curve_param_b = -F::from(17);
        let curve_param_b_expr = Expression::Constant(curve_param_b);

        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        let a1 = meta.query_advice(self.a, Rotation::next());
        let b1 = meta.query_advice(self.b, Rotation::next());
        let condition = meta.query_advice(self.a, Rotation(2));
        let a2 = meta.query_advice(self.a, Rotation(3));
        let b2 = meta.query_advice(self.b, Rotation(3));

        //      (x2-x1)/(y2-y1) = (x3-x1)/(-y3-y1)
        // =>   (x3-x1)(y2-y1) + (x2-x1)(y3+y1) = 0
        //
        // we do not want to open up the above equations
        // a fully expanded one will require 6 muls while the current
        // one only requires 2 muls

        // | a  | b  |
        // -----------
        // | x1 | y1 |
        // | x2 | y2 |
        // | c  |    |
        // | x3 | y3 |
        let add = (a2.clone() - a0.clone()) * (b1 - b0.clone())
            + (a1 - a0.clone()) * (b2.clone() + b0.clone());

        // Given (x1, y1), (x2, y2)
        // if condition is true, we return (x1, y1) + (x2, y2)
        // else we return (x1, y1)
        condition.clone() * add
            + (one.clone() - condition.clone()) * (a2.clone() - a0)
            + (one - condition) * (b2.clone() - b0)
            // enforce the result is on curve
            + a2.clone() * a2.clone() * a2
            - b2.clone() * b2
            + curve_param_b_expr
    }

    /// (x1, y1) and (x3, -y3) are on a tangential line of the curve
    pub(crate) fn ec_double_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let two = Expression::Constant(F::from(2));
        let three = Expression::Constant(F::from(3));
        // FIXME: currently hardcoded for Grumpkin curve
        let curve_param_b = -F::from(17);
        let curve_param_b_expr = Expression::Constant(curve_param_b);

        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        let a1 = meta.query_advice(self.a, Rotation::next());
        let b1 = meta.query_advice(self.b, Rotation::next());

        // the slope: 3^x1^2 / 2y^1
        // therefore: 2y1 * (y3 + y1) + 3x1^2 * (x3 - x1) = 0

        // | a  | b  |
        // -----------
        // | x1 | y1 |
        // | x3 | y3 |

        two * b0.clone() * (b1.clone() + b0) + (three * a0.clone() * a0.clone()) * (a1.clone() - a0)
        // enforce the result is on curve
        + a1.clone() * a1.clone() * a1
            - b1.clone() * b1
            + curve_param_b_expr
    }

    /// (x1, y1) is on curve
    pub(crate) fn on_curve_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        // FIXME: currently hardcoded for Grumpkin curve
        let curve_param_b = -F::from(17);
        let curve_param_b_expr = Expression::Constant(curve_param_b);

        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        // (1 - q1) * q2 * (a^3 - b^2 - 17) == c
        a0.clone() * a0.clone() * a0 - b0.clone() * b0 + curve_param_b_expr
    }

    /// partial bit decom
    /// - y3 = x1 + 2y1 + 4x2 + 8y2 + 16x3
    /// - x1, y1, x2, y2 are all binary
    pub(crate) fn partial_bit_decom_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let one = Expression::Constant(F::ONE);
        let two = Expression::Constant(F::from(2));
        let four = Expression::Constant(F::from(4));
        let eight = Expression::Constant(F::from(8));
        let sixteen = Expression::Constant(F::from(16));

        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        let a1 = meta.query_advice(self.a, Rotation::next());
        let b1 = meta.query_advice(self.b, Rotation::next());
        let a2 = meta.query_advice(self.a, Rotation(2));
        let b2 = meta.query_advice(self.b, Rotation(2));

        // y3 = x1 + 2y1 + 4x2 + 8y2 + 16x3
        a0.clone() + two * b0.clone() + four * a1.clone() + eight * b1.clone() + sixteen * a2 - b2
        // x1, y1, x2, y2 are all binary
            + a0.clone() * (one.clone() - a0)
            + b0.clone() * (one.clone() - b0)
            + a1.clone() * (one.clone() - a1)
            + b1.clone() * (one - b1)
    }

    /// additional gate
    pub(crate) fn add_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        let a1 = meta.query_advice(self.a, Rotation::next());

        a0 + b0 - a1
    }

    /// additional gate
    pub(crate) fn mul_gate(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let a0 = meta.query_advice(self.a, Rotation::cur());
        let b0 = meta.query_advice(self.b, Rotation::cur());
        let a1 = meta.query_advice(self.a, Rotation::next());

        a0 * b0 - a1
    }
}
