use std::marker::PhantomData;

use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Chip;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::Region;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::group::Curve;
use halo2_proofs::halo2curves::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Selector;
use halo2_proofs::poly::Rotation;

use crate::config::ECConfig;
use crate::AssignedECPoint;
use crate::ECPoint;
use crate::NativeECOps;

#[derive(Clone, Debug)]
struct ECChip<F: FieldExt> {
    config: ECConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for ECChip<F> {
    type Config = ECConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> ECChip<F> {
    fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    fn configure<C: Curve>(meta: &mut ConstraintSystem<F>) -> <Self as Chip<F>>::Config {
        let a = meta.advice_column();
        meta.enable_equality(a);
        let b = meta.advice_column();
        meta.enable_equality(b);
        let c = meta.advice_column();
        meta.enable_equality(c);
        let q1 = meta.selector();
        let q2 = meta.selector();

        let one = Expression::Constant(F::one());

        // FIXME: currently hardcoded for grumpkin curve
        let curve_param_b = -F::from(17);
        let curve_param_b_expr = Expression::Constant(curve_param_b);

        meta.create_gate("native ec", |meta| {
            // we only care for three operations, configured with the following setup
            //  |    used for | q1 | q2 | statement
            //  | ----------- | -- | -- | -------------
            //  |      ec add | 1  | 0  | (x1, y1), (x2, y2) and (x3, -y3) are on a same line
            //  |   ec double | 1  | 1  | tbd
            //  | is on curve | 0  | 1  | y1^2 = x1^3 - 17

            let q1 = meta.query_selector(q1);
            let q2 = meta.query_selector(q2);

            let a0 = meta.query_advice(a, Rotation::cur());
            let b0 = meta.query_advice(b, Rotation::cur());
            let c0 = meta.query_advice(c, Rotation::cur());
            let _a1 = meta.query_advice(a, Rotation::next());
            let _b1 = meta.query_advice(b, Rotation::next());
            let _c1 = meta.query_advice(c, Rotation::next());

            // ==========================================
            // case 1: (x1, y1), (x2, y2) and (x3, -y3) are on a same line
            // ==========================================

            // ==========================================
            // case 3: (a, b) is on curve
            // ==========================================
            // (1 - q1) * q2 * (a^3 - b^2 - 17) == c
            let case_3_express = (one - q1)
                * q2
                * (a0.clone() * a0.clone() * a0 - b0.clone() * b0 - c0 + curve_param_b_expr);

            vec![case_3_express]
        });

        ECConfig {
            a,
            b,
            c,
            q1,
            q2,
            curve_param_b,
        }
    }
}

impl<F: FieldExt> NativeECOps<F> for ECChip<F> {
    type Config = ECConfig<F>;
    type ECPoint = ECPoint<F>;
    type AssignedECPoint = AssignedECPoint<F>;

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    fn load_private_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
    ) -> Result<Self::AssignedECPoint, Error> {
        todo!()
    }

    /// For an input pair (x, y), checks if the point is on curve.
    /// Return boolean.
    fn is_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        //  | is on curve | 0  | 1  | y1^2 = x1^3 - 17

        let x = region.assign_advice(|| "x", config.a, offset, || p.x)?;
        let y = region.assign_advice(|| "y", config.b, offset, || p.y)?;
        let res = p.x * p.x * p.x - p.y * p.y + Value::known(config.curve_param_b);
        let res = region.assign_advice(|| "is_on_curve", config.c, offset, || res)?;
        config.q2.enable(region, offset)?;

        Ok(res)
    }
}
