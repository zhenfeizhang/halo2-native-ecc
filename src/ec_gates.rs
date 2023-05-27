use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Region;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2_proofs::halo2curves::group::Curve;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Error;

use crate::chip::ECChip;
use crate::config::ECConfig;
use crate::util::field_decompose;
use crate::util::field_decompose_u128;
use crate::util::leak;
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
    F: PrimeField,
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
            // we have already constraint that p1 == p3 and p1 itself is on curve via the custom gate
            // so we can use unchecked gate here
            self.load_private_point_unchecked(region, config, &p1_witness, offset)?
        } else {
            self.load_private_point(region, config, &p3_witness, offset)?
        };

        Ok(p3)
    }

    /// Return p2 = p1 + p1
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
        let p1 = p1.witness();
        let p2 = (p1 + p1).to_affine();
        let p2 = self.load_private_point_unchecked(region, config, &p2, offset)?;
        self.enforce_on_curve(region, config, &p2, offset)?;
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
        let mut res = C::identity();
        let bits = self.decompose_scalar(region, config, s, offset)?;

        for b in bits.iter().rev() {
            res = (res + res).into();
            if leak(&b.value()) == F::ONE {
                res = (res + *p).into();
            }
        }

        self.load_private_point(region, config, p, offset)
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
