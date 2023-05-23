use halo2_proofs::circuit::Value;
use halo2_proofs::halo2curves::ff::PrimeField;

pub(crate) fn leak<T: Copy + Default>(a: &Value<&T>) -> T {
    let mut t = T::default();
    a.map(|x| t = *x);
    t
}

// pub(crate) fn to_bits<F: PrimeField<Repr = [u8;32]>>(e: &F)->Vec<bool>{
//     let mut res = vec![];
//     let repr = e.to_repr();
//     for e in repr.iter(){
//         res.extend_from_slice(e.t)

//     }

// }
