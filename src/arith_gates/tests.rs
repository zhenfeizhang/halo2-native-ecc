use ark_std::test_rng;
use grumpkin::Fq;
use grumpkin::G1Affine;
use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::SimpleFloorPlanner;
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

use crate::arith_gates::ArithOps;
use crate::chip::ECChip;
use crate::config::ECConfig;
use crate::ec_gates::NativeECOps;

#[derive(Default, Debug, Clone, Copy)]
struct ArithTestCircuit {
    f1: Fq,
    f2: Fq,
    f3: Fq,      // f3 = f1 + f2
    f4: Fq,      // f4 = f1 * f2
    f5: [Fq; 6], // partial bit decom
}

impl Circuit<Fq> for ArithTestCircuit {
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
        let field_chip = ECChip::construct(config.clone());

        layouter.assign_region(
            || "test field circuit",
            |mut region| {
                let mut offset = 0;

                // unit test: addition
                {
                    let f3_rec =
                        field_chip.add(&mut region, &config, &self.f1, &self.f2, &mut offset)?;
                    let f3 = field_chip.load_private_field(
                        &mut region,
                        &config,
                        &self.f3,
                        &mut offset,
                    )?;
                    region.constrain_equal(f3.cell(), f3_rec.cell())?;
                }

                // unit test: multiplication
                {
                    let f4_rec =
                        field_chip.mul(&mut region, &config, &self.f1, &self.f2, &mut offset)?;
                    let f4 = field_chip.load_private_field(
                        &mut region,
                        &config,
                        &self.f4,
                        &mut offset,
                    )?;
                    region.constrain_equal(f4.cell(), f4_rec.cell())?;
                }

                // unit test: partial bit decompose
                {
                    let _cells = field_chip.partial_bit_decomp(
                        &mut region,
                        &config,
                        self.f5.as_ref(),
                        &mut offset,
                    )?;
                }

                // pad the last two rows
                field_chip.pad(&mut region, &config, &mut offset)?;

                Ok(())
            },
        )?;

        Ok(())
    }
}

#[test]
fn test_field_ops() {
    let k = 10;

    let mut rng = test_rng();

    let f1 = Fq::random(&mut rng);
    let f2 = Fq::random(&mut rng);
    let f3 = f1 + f2;
    let f4 = f1 * f2;
    {
        let f5 = [
            Fq::one(),
            Fq::zero(),
            Fq::zero(),
            Fq::one(),
            f1,
            f1 * Fq::from(16) + Fq::from(9),
        ];
        let circuit = ArithTestCircuit { f1, f2, f3, f4, f5 };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    // error case: addition fails
    {
        let f3 = f1 + f1;
        let f5 = [
            Fq::one(),
            Fq::zero(),
            Fq::zero(),
            Fq::one(),
            f1,
            f1 * Fq::from(16) + Fq::from(9),
        ];
        let circuit = ArithTestCircuit { f1, f2, f3, f4, f5 };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }
    // error case: multiplication fails
    {
        let f4 = f1 * f1;
        let f5 = [
            Fq::one(),
            Fq::zero(),
            Fq::zero(),
            Fq::one(),
            f1,
            f1 * Fq::from(16) + Fq::from(9),
        ];
        let circuit = ArithTestCircuit { f1, f2, f3, f4, f5 };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }
    // error case: not binary
    {
        let f5 = [
            Fq::from(2),
            Fq::zero(),
            Fq::zero(),
            Fq::one(),
            f1,
            f1 * Fq::from(16) + Fq::from(10),
        ];
        let circuit = ArithTestCircuit { f1, f2, f3, f4, f5 };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }
    // error case: sum not equal
    {
        let f5 = [
            Fq::zero(),
            Fq::zero(),
            Fq::zero(),
            Fq::one(),
            f1,
            f1 * Fq::from(16) + Fq::from(10),
        ];
        let circuit = ArithTestCircuit { f1, f2, f3, f4, f5 };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }
}
