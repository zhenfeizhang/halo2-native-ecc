use std::marker::PhantomData;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::Chip;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;

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

        // ec is enabled
        let q_ec_enable = meta.complex_selector();
        // ec conditional add
        let q1 = meta.complex_selector();
        // ec double
        let q2 = meta.complex_selector();
        // ec on curve
        let q3 = meta.complex_selector();

        let config = ECConfig {
            a,
            b,
            q_ec_enable,
            q1,
            q2,
            q3,
            _phantom: PhantomData::default(),
        };

        let one = Expression::Constant(F::ONE);

        meta.create_gate("native ec chip", |meta| {
            // |   op codes  | cost | q_ec_enabled | q1 | q2 | q3 | statement
            // | ----------- |:----:|:------------:| -- | -- | -- | -------------
            // |      ec add |   4  |       1      | 1  | 0  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
            // |   ec double |   2  |       1      | 0  | 1  | 0  | (x1, y1) and (x3, -y3) are on a tangential line of the curve
            // | is on curve |   1  |       1      | 0  | 0  | 1  | y1^2 = x1^3 - C::b()
            //
            // |     partial |   3  |       0      | 1  | 0  | 0  | y3 = x1 + y1 + x2 + y2 + x3 and
            // |   decompose |      |              |    |    |    | x1, y1, x2, y2 are all binary
            // |         add |   2  |       0      | 0  | 1  | 0  | a1 = a0 + b0
            // |         mul |   2  |       0      | 0  | 0  | 1  | a1 = a0 * b0

            let q1 = meta.query_selector(config.q1);
            let q2 = meta.query_selector(config.q2);
            let q3 = meta.query_selector(config.q3);
            let q_ec_enable = meta.query_selector(config.q_ec_enable);

            let ec_add_gate = config.conditional_ec_add_gate(meta);
            let ec_double_gate = config.ec_double_gate(meta);
            let on_curve_gate = config.on_curve_gate(meta);
            let partial_bit_decom_gate = config.partial_bit_decom_gate(meta);
            let add_gate = config.add_gate(meta);
            let mul_gate = config.mul_gate(meta);

            vec![
                // |      ec add |   4  |       1       | 1  | 0  | 0  |
                ec_add_gate * q_ec_enable.clone() * q1.clone()
                // |   ec double |   2  |       1       | 0  | 1  | 0  |
                    + ec_double_gate * q_ec_enable.clone() * q2.clone()
                // | is on curve |   1  |       1       | 0  | 0  | 1  |
                    + on_curve_gate * q_ec_enable.clone() * q3.clone()
                // |     partial |   3  |       0       | 1  | 0  | 0  | 
                // |   decompose |      |               |    |    |    |
                    + partial_bit_decom_gate * (one.clone() - q_ec_enable.clone()) * q1
                // |         add |   2  |       0       | 0  | 1  | 0  |  
                    + add_gate * (one.clone() - q_ec_enable.clone()) * q2
                // |         mul |   2  |       0       | 0  | 0  | 1  | 
                    + mul_gate * (one - q_ec_enable) * q3,
            ]
        });
        #[cfg(feature = "verbose")]
        println!("custom gate's degree {}", meta.degree());
        config
    }
}
