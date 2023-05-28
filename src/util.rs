use std::u128;

use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::ff::PrimeField;
use halo2curves::CurveAffine;

pub(crate) fn leak<T: Copy + Default>(a: &Value<&T>) -> T {
    let mut t = T::default();
    a.map(|x| t = *x);
    t
}

/// Split a scalar field elements into high and low and
/// store the high and low in base field.
pub(crate) fn field_decompose_u128<S>(e: &S) -> (u128, u128)
where
    S: PrimeField<Repr = [u8; 32]>,
{
    let repr = e.to_repr();
    let high = u128::from_le_bytes(repr[16..].try_into().unwrap());
    let low = u128::from_le_bytes(repr[..16].try_into().unwrap());
    (high, low)
}

/// Split a scalar field elements into high and low and
/// store the high and low in base field.
#[allow(dead_code)]
pub(crate) fn field_decompose<F, S>(e: &S) -> (F, F)
where
    F: PrimeField,
    S: PrimeField<Repr = [u8; 32]>,
{
    let repr = e.to_repr();
    let high = F::from_u128(u128::from_le_bytes(repr[16..].try_into().unwrap()));
    let low = F::from_u128(u128::from_le_bytes(repr[..16].try_into().unwrap()));
    (high, low)
}

#[allow(dead_code)]
pub(crate) fn to_le_bits<F: PrimeField<Repr = [u8; 32]>>(e: &F) -> Vec<bool> {
    let mut res = vec![];
    let repr = e.to_repr();
    for e in repr.iter() {
        res.extend_from_slice(byte_to_le_bits(e).as_slice())
    }
    res
}

#[inline]
fn byte_to_le_bits(b: &u8) -> Vec<bool> {
    let mut t = *b;
    let mut res = vec![];
    for _ in 0..8 {
        res.push(t & 1 == 1);
        t >>= 1;
    }
    res
}

#[inline]
pub(crate) fn decompose_u128(a: &u128) -> Vec<u64> {
    a.to_le_bytes()
        .iter()
        .flat_map(|x| {
            byte_to_le_bits(x)
                .iter()
                .map(|&x| x as u64)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[inline]
// hardcoded value for `-2^256 * generator` for Grumpkin curve
pub(crate) fn neg_generator_times_2_to_256<C, F>() -> C
where
    F: PrimeField<Repr = [u8; 32]>,
    C: CurveAffine<Base = F>,
{
    let x = F::from_str_vartime(
        "18292374296067206172215749431916515128228165256807037435601971767767562625877",
    )
    .unwrap();
    let y = F::from_str_vartime(
        "8411761026004062292626067694055242675827541323706122037355419552115320964415",
    )
    .unwrap();
    C::from_xy(x, y).unwrap()
}

#[cfg(test)]
mod test {
    use halo2_proofs::arithmetic::Field;
    use halo2curves::grumpkin::Fq;
    use halo2curves::grumpkin::Fr;

    use crate::util::byte_to_le_bits;
    use crate::util::to_le_bits;

    use super::decompose_u128;
    use super::field_decompose;

    #[test]
    fn test_to_bites() {
        assert_eq!(
            byte_to_le_bits(&4),
            vec![false, false, true, false, false, false, false, false]
        );

        {
            let f = Fr::from(4);
            let sequence = to_le_bits(&f);

            for (i, v) in sequence.iter().enumerate() {
                if i == 2 {
                    assert_eq!(*v, true)
                } else {
                    assert_eq!(*v, false)
                }
            }
        }

        {
            let f = Fr::from(4 + (1 << 13));
            let sequence = to_le_bits(&f);

            for (i, v) in sequence.iter().enumerate() {
                if i == 2 || i == 13 {
                    assert_eq!(*v, true, "{}-th coefficient failed", i)
                } else {
                    assert_eq!(*v, false, "{}-th coefficient failed", i)
                }
            }
        }
    }

    #[test]
    fn test_field_decom() {
        let mut rng = ark_std::test_rng();
        let a = Fr::random(&mut rng);
        let (_high, _low) = field_decompose::<Fq, Fr>(&a);

        // println!("{:?}", a);
        // println!("{:?}", high);
        // println!("{:?}", low);

        let a = u128::from_le_bytes([1; 16]);
        let _bits = decompose_u128(&a);
        // println!("{0:x?}", a);
        // println!("{:?}", bits);
        // panic!()
    }
}
