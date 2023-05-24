use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Region;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Error;

use crate::ECChip;
use crate::ECConfig;

#[cfg(test)]
mod tests;

pub trait ArithOps<F: Field> {
    type Config;

    /// Load a private field element
    fn load_private_field(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        f: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// Load two private field elements
    fn load_two_private_fields(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        f1: &F,
        f2: &F,
        offset: &mut usize,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error>;

    /// Add two cells and return the sum
    fn add(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        a: &F,
        b: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// Multiply two cells and return the product
    fn mul(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        a: &F,
        b: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// Input x1, y1, x2, y2, x3, y3
    /// Assert that
    /// - x3 = x1 + 2y1 + 4x2 + 8y2 + 16y3
    /// - x1, y1, x2, y2 are all binary
    fn partial_bit_decomp(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>;

    /// Input a u128,
    /// Output
    /// - its bit decomposition cells in little endian
    /// - the cell that contains u128
    fn decompose_u128(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        input: &u128,
        offset: &mut usize,
    ) -> Result<(Vec<AssignedCell<F, F>>, AssignedCell<F, F>), Error>;
}

impl<C, F> ArithOps<F> for ECChip<C, F>
where
    C: CurveAffine<Base = F>,
    F: PrimeField,
{
    type Config = ECConfig<C, F>;

    // Load a private field element
    fn load_private_field(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        f: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        let res = region.assign_advice(|| "field element", config.a, *offset, || Value::known(*f));
        let _ = region.assign_advice(
            || "field element",
            config.b,
            *offset,
            || Value::known(F::ZERO),
        );

        *offset += 1;
        res
    }

    /// Load two private field elements
    fn load_two_private_fields(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        f1: &F,
        f2: &F,
        offset: &mut usize,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let a =
            region.assign_advice(|| "field element", config.a, *offset, || Value::known(*f1))?;
        let b =
            region.assign_advice(|| "field element", config.b, *offset, || Value::known(*f2))?;

        *offset += 1;
        Ok((a, b))
    }

    /// Add two cells and return the sum
    fn add(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        a: &F,
        b: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        //  |         add |       1       | 1  | 0  |
        config.q_ec_disabled.enable(region, *offset)?;
        config.q1.enable(region, *offset)?;
        region.assign_advice(|| "field element", config.a, *offset, || Value::known(*a))?;
        region.assign_advice(|| "field element", config.b, *offset, || Value::known(*b))?;

        let c = *a + *b;
        let res = region.assign_advice(
            || "field element",
            config.a,
            *offset + 1,
            || Value::known(c),
        );
        let _ = region.assign_advice(
            || "field element",
            config.b,
            *offset + 1,
            || Value::known(F::ZERO),
        );

        *offset += 2;
        res
    }

    // Multiply two cells and return the product
    fn mul(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        a: &F,
        b: &F,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        //  |         mul |       1       | 1  | 1  |
        config.q_ec_disabled.enable(region, *offset)?;
        config.q1.enable(region, *offset)?;
        config.q2.enable(region, *offset)?;
        region.assign_advice(|| "field element", config.a, *offset, || Value::known(*a))?;
        region.assign_advice(|| "field element", config.b, *offset, || Value::known(*b))?;

        let c = *a * *b;
        let res = region.assign_advice(
            || "field element",
            config.a,
            *offset + 1,
            || Value::known(c),
        );
        let _ = region.assign_advice(
            || "field element",
            config.b,
            *offset + 1,
            || Value::known(F::ZERO),
        );

        *offset += 2;
        res
    }

    /// Input x1, y1, x2, y2, x3, y3
    /// Assert that
    /// - x3 = x1 + 2y1 + 4x2 + 8y2 + 16y3
    /// - x1, y1, x2, y2 are all binary
    fn partial_bit_decomp(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        // |     partial |      1       | 0  | 1  |
        assert_eq!(inputs.len(), 6, "input length is not 6");

        let mut res = vec![];
        config.q_ec_disabled.enable(region, *offset)?;
        config.q2.enable(region, *offset)?;
        res.push(region.assign_advice(|| "x0", config.a, *offset, || Value::known(inputs[0]))?);
        res.push(region.assign_advice(|| "y0", config.b, *offset, || Value::known(inputs[1]))?);
        res.push(region.assign_advice(
            || "x1",
            config.a,
            *offset + 1,
            || Value::known(inputs[2]),
        )?);
        res.push(region.assign_advice(
            || "y1",
            config.b,
            *offset + 1,
            || Value::known(inputs[3]),
        )?);
        res.push(region.assign_advice(
            || "x2",
            config.a,
            *offset + 2,
            || Value::known(inputs[4]),
        )?);
        res.push(region.assign_advice(
            || "y2",
            config.b,
            *offset + 2,
            || Value::known(inputs[5]),
        )?);

        *offset += 3;
        Ok(res)
    }

    /// Input a u128,
    /// Output
    /// - its bit decomposition cells in little endian
    /// - the cell that contains u128
    fn decompose_u128(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        input: &u128,
        offset: &mut usize,
    ) -> Result<(Vec<AssignedCell<F, F>>, AssignedCell<F, F>), Error> {
        let input_le_vec = crate::util::decompose_u128(input);
        let input_field_vec = input_le_vec
            .iter()
            .rev()
            .map(|&x| F::from(x))
            .collect::<Vec<_>>();

        let two = F::from(2);
        let four = F::from(4);
        let eight = F::from(8);
        let sixteen = F::from(16);

        let mut acc;
        let mut prev_acc = F::ZERO;

        let mut res = vec![];
        let mut acc_cells = vec![];
        // we assert the decomposition via 32 calls of partial decomp
        // each call we absorb 4 bits
        for i in 0..32 {
            config.q_ec_disabled.enable(region, *offset)?;
            config.q2.enable(region, *offset)?;

            // allocate the four bits to be absorbed
            res.push(region.assign_advice(
                || "b2",
                config.b,
                *offset + 1,
                || Value::known(input_field_vec[4 * i]),
            )?);
            res.push(region.assign_advice(
                || "a2",
                config.a,
                *offset + 1,
                || Value::known(input_field_vec[4 * i + 1]),
            )?);
            res.push(region.assign_advice(
                || "b1",
                config.b,
                *offset,
                || Value::known(input_field_vec[4 * i + 2]),
            )?);
            res.push(region.assign_advice(
                || "a1",
                config.a,
                *offset,
                || Value::known(input_field_vec[4 * i + 3]),
            )?);

            // compute the accumulated value
            acc = input_field_vec[4 * i + 3]
                + input_field_vec[4 * i + 2] * two
                + input_field_vec[4 * i + 1] * four
                + input_field_vec[4 * i] * eight
                + prev_acc * sixteen;

            // assign accumulator
            acc_cells.push(region.assign_advice(
                || "a3",
                config.a,
                *offset + 2,
                || Value::known(prev_acc),
            )?);
            acc_cells.push(region.assign_advice(
                || "b3",
                config.b,
                *offset + 2,
                || Value::known(acc),
            )?);
            prev_acc = acc;
            *offset += 3;
        }

        // sanity check
        assert_eq!(prev_acc, F::from_u128(*input));

        // constrain the accumulators are well-formed
        for i in 0..31 {
            region.constrain_equal(
                // acc in the previous round
                acc_cells[i * 2 + 1].cell(),
                // prev_acc in the current round
                acc_cells[(i + 1) * 2].cell(),
            )?;
        }

        // format the result in little endian format
        res.reverse();

        // for (i, e) in res.iter().enumerate() {
        //     println!("{} {:?}", i, e.value());
        //     if i % 8 == 7 {
        //         println!()
        //     }
        // }
        Ok((res, acc_cells.last().unwrap().clone()))
    }
}
