use ark_std::test_rng;
use grumpkin::Fq;
use grumpkin::G1Affine;
use grumpkin::G1;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::SimpleFloorPlanner;
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::group::Curve;
use halo2_proofs::halo2curves::group::Group;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

use crate::chip::ECChip;
use crate::config::ECConfig;
use crate::NativeECOps;

#[derive(Default, Debug, Clone, Copy)]
struct ECTestCircuit {
    p1: G1Affine,
    p2: G1Affine,
    p3: G1Affine, // p1 + p2
    p4: G1Affine, // 2p1
}

impl Circuit<Fq> for ECTestCircuit {
    type Config = ECConfig<G1Affine, Fq>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fq>) -> Self::Config {
        ECChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fq>,
    ) -> Result<(), Error> {
        let ec_chip = ECChip::construct(config.clone());

        layouter.assign_region(
            || "test ec circuit",
            |mut region| {
                let mut offset = 0;
                // unit testing: `load private unchecked`, then `enforce is on curve`
                let _p1 = {
                    let p1 = ec_chip.load_private_unchecked(
                        &mut region,
                        &config,
                        &self.p1,
                        &mut offset,
                    )?;
                    ec_chip.enforce_on_curve(&mut region, &config, &p1, &mut offset)?;
                    p1
                };
                // unit testing: load private
                let _p2 = ec_chip.load_private(&mut region, &config, &self.p2, &mut offset)?;
                let p3 = ec_chip.load_private(&mut region, &config, &self.p3, &mut offset)?;
                let p4 = ec_chip.load_private(&mut region, &config, &self.p4, &mut offset)?;

                // unit testing: point addition
                {
                    let p1 = ec_chip.load_private_unchecked(
                        &mut region,
                        &config,
                        &self.p1,
                        &mut offset,
                    )?;
                    let p2 = ec_chip.load_private_unchecked(
                        &mut region,
                        &config,
                        &self.p2,
                        &mut offset,
                    )?;
                    let p3_rec =
                        ec_chip.add_assigned_points(&mut region, &config, &p1, &p2, &mut offset)?;

                    region.constrain_equal(p3.x.cell(), p3_rec.x.cell())?;
                    region.constrain_equal(p3.y.cell(), p3_rec.y.cell())?;
                }

                // unit testing: point addition from witnesses
                {
                    let p3_rec = ec_chip.add_points(
                        &mut region,
                        &config,
                        &self.p1,
                        &self.p2,
                        &mut offset,
                    )?;

                    region.constrain_equal(p3.x.cell(), p3_rec.x.cell())?;
                    region.constrain_equal(p3.y.cell(), p3_rec.y.cell())?;
                }

                // unit testing: point doubling
                {
                    let p1 = ec_chip.load_private_unchecked(
                        &mut region,
                        &config,
                        &self.p1,
                        &mut offset,
                    )?;
                    let p4_rec =
                        ec_chip.double_assigned_point(&mut region, &config, &p1, &mut offset)?;

                    region.constrain_equal(p4.x.cell(), p4_rec.x.cell())?;
                    region.constrain_equal(p4.y.cell(), p4_rec.y.cell())?;
                }

                // pad the last two rows
                ec_chip.pad(&mut region, &config, &mut offset)?;

                Ok(())
            },
        )?;

        Ok(())
    }
}

#[test]
fn test_ec_ops() {
    let k = 10;

    let mut rng = test_rng();
    let p1 = G1::random(&mut rng).to_affine();
    let p2 = G1::random(&mut rng).to_affine();
    let p3 = (p1 + p2).to_affine();
    let p4 = (p1 + p1).to_affine();

    let circuit = ECTestCircuit { p1, p2, p3, p4 };

    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
