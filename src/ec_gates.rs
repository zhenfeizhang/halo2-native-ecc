use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Region;
use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::group::Curve;
use halo2_proofs::halo2curves::CurveAffine;
use halo2_proofs::plonk::Error;

use crate::chip::ECChip;
use crate::config::ECConfig;
use crate::util::leak;
use crate::AssignedECPoint;

#[cfg(test)]
mod tests;

pub trait NativeECOps<F: Field> {
    type Config;
    type ECPoint: CurveAffine;
    type AssignedECPoint;

    /// Loads an ecpoint (x, y) into the circuit as a private input.
    /// Constraints (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b); and use column c to enforce point is on curve
    fn load_private_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
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
        p: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// For an input pair (x, y), checks if the point is on curve.
    /// Return boolean.
    fn is_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error>;

    /// For an input pair (x, y), enforces the point is on curve.
    fn enforce_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<(), Error> {
        let res = self.is_on_curve(region, config, p, offset)?;
        region.constrain_constant(res.cell(), F::ZERO)
    }

    /// Return p3 = p1 + p2.
    /// Also enforces p1 and p2 are on curve.
    fn add_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        p2: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p3 = p1 + p2
    fn add_assigned_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        p2: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p2 = p1 + p1
    /// Also enforces p1 is on curve.
    fn double_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Return p2 = p1 + p1
    fn double_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// Decompose a scalar into a vector of boolean Cells
    fn decompose_scalar(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error>;

    fn mul_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error>;

    /// summation
    fn summation(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<
        (
            Vec<AssignedCell<F, F>>, // cells allocated for inputs
            AssignedCell<F, F>,      // cells allocated for sum
        ),
        Error,
    >;

    /// Pad the row with empty cells.
    fn pad(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        offset: &mut usize,
    ) -> Result<(), Error>;
}

impl<C, F> NativeECOps<F> for ECChip<C, F>
where
    C: CurveAffine<Base = F>,
    F: Field,
{
    type Config = ECConfig<C, F>;
    type ECPoint = C;
    type AssignedECPoint = AssignedECPoint<C, F>;

    /// Loads a pair (x, y) into the circuit as a private input.
    /// Do not constraint (x, y) is on curve.
    ///
    /// Will allocate the (x, y) to columns (a, b)
    fn load_private_point_unchecked(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        let p = p.coordinates().unwrap();
        let x = region.assign_advice(|| "x", config.a, *offset, || Value::known(*p.x()))?;
        let y = region.assign_advice(|| "y", config.b, *offset, || Value::known(*p.y()))?;
        *offset += 1;
        Ok(Self::AssignedECPoint::new(x, y))
    }

    /// For an input pair (x, y), checks if the point is on curve.
    /// Return boolean.
    fn is_on_curve(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        //  | is on curve | 0  | 1  | y1^2 = x1^3 - 17
        let x = leak(&p.x.value());
        let y = leak(&p.y.value());
        let r = x * x * x - y * y + C::b();

        // use column a to store the result; padd column b with 0
        let r = region.assign_advice(|| "is_on_curve", config.a, *offset, || Value::known(r))?;
        region.assign_advice(
            || "is_on_curve",
            config.b,
            *offset,
            || Value::known(F::ZERO),
        )?;
        config.q2.enable(region, *offset - 1)?;
        *offset += 1;
        Ok(r)
    }

    /// Return p3 = p1 + p2.
    /// Also enforces p1 and p2 are on curve.
    fn add_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        p2: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        // checking if a point is on curve takes 2 continuous rows, therefore we need
        // to copy the cells for additions
        let p1_checked = self.load_private_point(region, config, p1, offset)?;
        let p2_checked = self.load_private_point(region, config, p2, offset)?;
        let p1_unchecked = self.load_private_point_unchecked(region, config, p1, offset)?;
        let p2_unchecked = self.load_private_point_unchecked(region, config, p2, offset)?;

        region.constrain_equal(p1_checked.x.cell(), p1_unchecked.x.cell())?;
        region.constrain_equal(p1_checked.y.cell(), p1_unchecked.y.cell())?;
        region.constrain_equal(p2_checked.x.cell(), p2_unchecked.x.cell())?;
        region.constrain_equal(p2_checked.y.cell(), p2_unchecked.y.cell())?;

        self.add_assigned_points(region, config, &p1_unchecked, &p2_unchecked, offset)
    }

    /// Return cells for p3 = p1 + p2
    ///
    /// Required layout:
    ///
    ///  index  |  a   |  b
    ///  -------|------|------
    ///         | p1.x | p1.y
    ///         | p2.x | p2.y
    ///  offset | p3.x | p3.y
    fn add_assigned_points(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        p2: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        //                  q1   q2
        //  |      ec add | 1  | 0  |
        config.q1.enable(region, *offset - 2)?;

        let p1 = p1.witness();
        let p2 = p2.witness();
        let p3 = (p1 + p2).to_affine();

        let p3 = self.load_private_point_unchecked(region, config, &p3, offset)?;
        self.enforce_on_curve(region, config, &p3, offset)?;
        Ok(p3)
    }

    /// Return p2 = p1 + p1
    /// Also enforces p1 is on curve.
    fn double_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::ECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        // checking if a point is on curve takes 2 continuous rows, therefore we need
        // to copy the cells for doubling
        let p1_checked = self.load_private_point(region, config, p1, offset)?;
        let p1_unchecked = self.load_private_point_unchecked(region, config, p1, offset)?;

        region.constrain_equal(p1_checked.x.cell(), p1_unchecked.x.cell())?;
        region.constrain_equal(p1_checked.y.cell(), p1_unchecked.y.cell())?;

        self.double_assigned_point(region, config, &p1_unchecked, offset)
    }

    /// Return p2 = p1 + p1
    fn double_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p1: &Self::AssignedECPoint,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
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
    fn decompose_scalar(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<F, F>>, Error> {
        let mut res = vec![];

        // let bits =

        Ok(res)
    }

    fn mul_assigned_point(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        p: &Self::AssignedECPoint,
        s: &<Self::ECPoint as CurveAffine>::ScalarExt,
        offset: &mut usize,
    ) -> Result<Self::AssignedECPoint, Error> {
        todo!();
    }

    /// summation
    fn summation(
        &self,
        region: &mut Region<F>,
        config: &Self::Config,
        inputs: &[F],
        offset: &mut usize,
    ) -> Result<
        (
            Vec<AssignedCell<F, F>>, // cells allocated for inputs
            AssignedCell<F, F>,      // cells allocated for sum
        ),
        Error,
    > {
        // let mut res = vec![];

        // for chunk in inputs.chunks(5) {
        //     config.q_ec_disabled.enable(region, *offset)?;

        //     let chunk_sum: F = chunk.iter().sum();
        //     res.push(region.assign_advice(
        //         || "x0",
        //         config.a,
        //         *offset,
        //         || Value::known(chunk[0]),
        //     )?);
        //     res.push(region.assign_advice(
        //         || "x1",
        //         config.b,
        //         *offset,
        //         || Value::known(chunk[1]),
        //     )?);
        //     res.push(region.assign_advice(
        //         || "x2",
        //         config.a,
        //         *offset + 1,
        //         || Value::known(chunk[2]),
        //     )?);
        //     res.push(region.assign_advice(
        //         || "x3",
        //         config.b,
        //         *offset + 1,
        //         || Value::known(chunk[3]),
        //     )?);
        //     res.push(region.assign_advice(
        //         || "x4",
        //         config.a,
        //         *offset + 2,
        //         || Value::known(chunk[4]),
        //     )?);
        //     region.assign_advice(|| "sum", config.b, *offset + 2, || Value::known(chunk_sum))?;
        //     *offset += 3;
        // }

        todo!()

        // Ok(res)
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

        *offset += 2;
        Ok(())
    }
}
