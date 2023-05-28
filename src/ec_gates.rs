use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Region;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::group::Curve;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Error;

use crate::chip::ECChip;
use crate::config::ECConfig;
use crate::util::field_decompose_u128;
use crate::util::leak;
use crate::util::neg_generator_times_2_to_256;
use crate::ArithOps;
use crate::AssignedECPoint;

#[cfg(test)]
mod tests;

pub trait NativeECOps<C, F>
where
    // the embedded curve, i.e., Grumpkin
    C: CurveAffine<Base = F>,
    // the field for circuit, i.e., BN::Scalar
    F: PrimeField,
{
    type Config;
    type AssignedECPoint;

    /// Loads an ecpoint (x, y) into the circuit as a private input.
    /// Constraints (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b); and use column c to enforce point is on curve
    fn load_private_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &C,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        let p = self.load_private_point_unchecked(region, config, p, offset)?;
        self.enforce_on_curve(region, config, &p, offset)?;
        Ok(p)
    }

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b)
    fn load_private_point_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &C,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// For an input pair (x, y), enforces the point is on curve.
    fn enforce_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<(), Error>;

    /// Input p1 and p2 that are on the curve.
    /// Input an additional bit b.
    ///
    /// Returns
    /// - p3 = p1 + p2 if b == 1.
    /// - p3 = p1 if b == 0.
    ///
    /// Caller must check p1 and p2 are on curve and b is a bit.
    fn conditional_point_add(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        p2: &Self::AssignedECPoint,
        b: &AssignedCell<F, F>,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p2 = p1 + p1
    fn point_double(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Decompose a scalar into a vector of boolean Cells
    fn decompose_scalar<S>(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        s: &C::ScalarExt,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>
    where
        S: PrimeField<Repr = [u8; 32]>,
        C: CurveAffine<ScalarExt = S>;

    /// Point mul via double-then-add method
    fn point_mul<S>(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &C,
        s: &C::ScalarExt,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>
    where
        S: PrimeField<Repr = [u8; 32]>,
        C: CurveAffine<ScalarExt = S>;

    /// Pad the row with empty cells.
    fn pad(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        offset: &mut usize,
    ) -> Result<(), Error>;
}

impl<C, F> NativeECOps<C, F> for ECChip<C, F>
where
    C: CurveAffine<Base = F>,
    F: PrimeField<Repr = [u8; 32]>,
{
    type Config = ECConfig<C, F>;
    type AssignedECPoint = AssignedECPoint<C, F>;

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b)
    fn load_private_point_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &C,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        let p = p.coordinates().unwrap();
        let x = region.assign_advice(|| "x", config.a, *offset, || Value::known(*p.x()))?;
        let y = region.assign_advice(|| "y", config.b, *offset, || Value::known(*p.y()))?;
        let res = Self::AssignedECPoint::new(x, y, *offset);
        *offset += 1;
        Ok(res)
    }

    /// For an input pair (x, y), enforces the point is on curve.
    /// The point must locate at (offset - 1) row
    fn enforce_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<(), Error> {
        assert_eq!(
            p.offset,
            *offset - 1,
            "on curve: p is not the latest assigned cells"
        );

        #[cfg(feature = "verbose")]
        {
            println!(
                "[on curve check]           selector: {}, point: {}",
                *offset - 1,
                p.offset
            );
        }

        //  | is on curve | 0  | 1  | y1^2 = x1^3 - 17
        config.q2.enable(region, *offset - 1)?;
        Ok(())
    }

    /// Input p1 and p2 that are on the curve.
    /// Input an additional bit b.
    ///
    /// Returns
    /// - p3 = p1 + p2 if b == 1.
    /// - p3 = p1 if b == 0.
    ///
    /// Ensures
    /// - p3 is on curve
    ///
    /// Caller must check p1 and p2 are on curve and b is a bit.
    fn conditional_point_add(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        p2: &Self::AssignedECPoint,
        b: &AssignedCell<F, F>,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        //  index  |  a   |  b
        //  -------|------|------
        //         | p1.x | p1.y
        //         | p2.x | p2.y
        //         | cond |
        //  offset | p3.x | p3.y

        //                  q1   q2
        //  | cond ec add | 1  | 0  |
        config.q1.enable(region, *offset - 3)?;

        let p1_witness = p1.witness();
        let p2_witness = p2.witness();
        let p3_witness = (p1_witness + p2_witness).to_affine();
        let bit = leak(&b.value());

        let p3 = if bit == F::ZERO {
            self.load_private_point_unchecked(region, config, &p1_witness, offset)?
        } else {
            self.load_private_point_unchecked(region, config, &p3_witness, offset)?
        };

        #[cfg(feature = "verbose")]
        {
            println!(
                "[conditional point add]    selector: {}, points: {} {} {}",
                *offset - 3,
                p1.offset,
                p2.offset,
                p3.offset
            );
        }

        Ok(p3)
    }

    /// Return p2 = p1 + p1
    ///
    /// Ensures
    /// - p2 is on curve
    ///
    /// Caller must check p1 is on curve.
    fn point_double(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        assert_eq!(
            p1.offset,
            *offset - 1,
            "point double: p is not the latest assigned cells"
        );

        //                  q1   q2
        //  |   ec double | 1  | 1  |
        config.q1.enable(region, *offset - 1)?;
        config.q2.enable(region, *offset - 1)?;
        let p1_witness = p1.witness();
        let p2 = (p1_witness + p1_witness).to_affine();
        let p2 = self.load_private_point_unchecked(region, config, &p2, offset)?;

        #[cfg(feature = "verbose")]
        {
            println!(
                "[point double]             selector: {}, points: {} {}",
                *offset - 1,
                p1.offset,
                p2.offset,
            );
        }

        Ok(p2)
    }

    /// Decompose a scalar into a vector of boolean Cells
    fn decompose_scalar<S>(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        s: &C::ScalarExt,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>
    where
        S: PrimeField<Repr = [u8; 32]>,
        C: CurveAffine<ScalarExt = S>,
    {
        let (high, low) = field_decompose_u128(s);
        let (low_cells, _res) = self.decompose_u128(region, config, &low, offset)?;
        let (high_cells, _res) = self.decompose_u128(region, config, &high, offset)?;
        let res = [low_cells.as_slice(), high_cells.as_slice()].concat();
        // println!("s: {:?}", s);
        // for (i, e) in res.iter().enumerate(){
        //     println!("{} {:?}", i, e.value());
        // }
        Ok(res)
    }

    /// Point mul via double-then-add method
    // todo: assigned point -> point
    fn point_mul<S>(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &C,
        s: &C::ScalarExt,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>
    where
        S: PrimeField<Repr = [u8; 32]>,
        C: CurveAffine<ScalarExt = S>,
    {
        let gen = C::generator();
        let bits = self.decompose_scalar(region, config, s, offset)?;

        let p_assigned = self.load_private_point(region, config, &p, offset)?;
        let gen_assigned = self.load_private_point(region, config, &gen, offset)?;

        // we do not have a cell representation for infinity point
        // therefore we first compute
        //  res = 2^256 * generator + p *s
        // ans then subtract 2^256 * generator from res
        let mut res: AssignedECPoint<C, F> = gen_assigned;

        // begin the `double-then-add` loop
        for b in bits.iter().rev() {
            // double
            let res_double = self.point_double(region, config, &res, offset)?;

            // conditional add depending on the bit b
            res = {
                let p_copied = if leak(&b.value()) == F::ONE {
                    // copy the base point cells
                    let p_copied: AssignedECPoint<C, F> =
                        self.load_private_point_unchecked(region, config, p, offset)?;
                    region.constrain_equal(p_copied.x.cell(), p_assigned.x.cell())?;
                    region.constrain_equal(p_copied.y.cell(), p_assigned.y.cell())?;
                    p_copied
                } else {
                    // the point here doesn't matter but we do need to fill in the cells
                    self.load_private_point_unchecked(region, config, &gen, offset)?
                };

                // copy the bit cell; already constraint `bit` is either 0 or 1
                let (bit, _) = self.load_two_private_fields(
                    region,
                    config,
                    &leak(&b.value()),
                    &F::ZERO,
                    offset,
                )?;
                region.constrain_equal(bit.cell(), b.cell())?;

                // conditional add
                self.conditional_point_add(region, config, &res_double, &p_copied, &bit, offset)?
            };
        }

        // now we  subtract 2^256 * generator from res
        let offset_generator = neg_generator_times_2_to_256::<C, C::Base>();
        let offset_generator_assigned =
            self.load_private_point_unchecked(region, config, &offset_generator, offset)?;
        let (bit, _) = self.load_two_private_fields(region, config, &F::ONE, &F::ZERO, offset)?;
        res = self.conditional_point_add(
            region,
            config,
            &res,
            &offset_generator_assigned,
            &bit,
            offset,
        )?;

        Ok(res)
    }

    /// Pad the row with empty cells.
    fn pad(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        offset: &mut usize,
    ) -> Result<(), Error> {
        region.assign_advice(|| "pad", config.a, *offset, || Value::known(F::ZERO))?;
        region.assign_advice(|| "pad", config.b, *offset, || Value::known(F::ZERO))?;
        region.assign_advice(|| "pad", config.a, *offset + 1, || Value::known(F::ZERO))?;
        region.assign_advice(|| "pad", config.b, *offset + 1, || Value::known(F::ZERO))?;
        region.assign_advice(|| "pad", config.a, *offset + 2, || Value::known(F::ZERO))?;
        region.assign_advice(|| "pad", config.b, *offset + 2, || Value::known(F::ZERO))?;
        *offset += 3;
        Ok(())
    }
}
