use std::marker::PhantomData;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::Chip;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::poly::Rotation;

use crate::config::ECConfig;

#[derive(Clone, Debug)]
pub struct ECChip<C, F>
where
    // the embedded curve, i.e., Grumpkin
    C: CurveAffine<Base = F>,
    // the field for circuit, i.e., BN::Scalar
    F: Field,
{
    config: ECConfig<C, F>,
    _phantom: PhantomData<F>,
}

impl<C, F> Chip<F> for ECChip<C, F>
where
    C: CurveAffine<Base = F>,
    F: Field,
{
    type Config = ECConfig<C, F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<C, F> ECChip<C, F>
where
    C: CurveAffine<Base = F>,
    F: PrimeField,
{
    pub fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> <Self as Chip<F>>::Config {
        let a = meta.advice_column();
        meta.enable_equality(a);
        let b = meta.advice_column();
        meta.enable_equality(b);

        let f = meta.fixed_column();
        meta.enable_constant(f);

        let q1 = meta.complex_selector();
        let q2 = meta.complex_selector();

        let one = Expression::Constant(F::ONE);
        let two = Expression::Constant(F::from(2));
        let three = Expression::Constant(F::from(3));

        // FIXME: currently hardcoded for Grumpkin curve
        let curve_param_b = -F::from(17);
        let curve_param_b_expr = Expression::Constant(curve_param_b);

        meta.create_gate("native ec", |meta| {
            // we only care for three operations, configured with the following setup
            //  |    used for | q1 | q2 | statement
            //  | ----------- | -- | -- | -------------
            //  |      ec add | 1  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
            //  |   ec double | 1  | 1  | (x1, y1) and (x3, -y3) are on a tangential line of the curve
            //  | is on curve | 0  | 1  | y1^2 = x1^3 - 17

            let q1 = meta.query_selector(q1);
            let q2 = meta.query_selector(q2);

            let a0 = meta.query_advice(a, Rotation::cur());
            let b0 = meta.query_advice(b, Rotation::cur());
            let a1 = meta.query_advice(a, Rotation::next());
            let b1 = meta.query_advice(b, Rotation::next());
            let a2 = meta.query_advice(a, Rotation(2));
            let b2 = meta.query_advice(b, Rotation(2));

            // ==========================================
            // case 1: (x1, y1), (x2, y2) and (x3, -y3) are on a same line
            // ==========================================
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
            // | x3 | y3 |

            #[rustfmt::skip]
            let case_1_express = q1.clone()
                * (one.clone() - q2.clone())
                * ( (a2 - a0.clone())   // (x3-x1)
                  * (b1.clone() - b0.clone())   // (y2-y1)
                  + (a1.clone() - a0.clone())   // (x2-x1)
                  * (b2 + b0.clone())   // (y3+y1)
                );

            // ==========================================
            // case 2:
            // - (x1, y1) and (x3, -y3) are on a tangential line of the curve
            // ==========================================
            // the slope: 3^x1^2 / 2y^1
            // therefore: 2y1 * (y3 + y1) + 3x1^2 * (x3 - x1) = 0

            // | a  | b  |
            // -----------
            // | x1 | y1 |
            // | x3 | y3 |

            #[rustfmt::skip]
            let case_2_express = q1.clone()
                * q2.clone()
                * ( two * b0.clone()                // 2  * y1
                  * (b1 + b0.clone())       // y3 + y1
                  + (three*a0.clone() * a0.clone()) // 3  * x1^2
                  * (a1.clone() - a0.clone())       // x3 - x1
                );

            // ==========================================
            // case 3: (a, b) is on curve
            // ==========================================
            // (1 - q1) * q2 * (a^3 - b^2 - 17) == c
            let case_3_express = (one - q1)
                * q2
                * (a0.clone() * a0.clone() * a0 - b0.clone() * b0 - a1 + curve_param_b_expr);

            vec![case_1_express + case_2_express + case_3_express]
        });

        ECConfig {
            a,
            b,
            q1,
            q2,
            _phantom: PhantomData::default(),
        }
    }
}
