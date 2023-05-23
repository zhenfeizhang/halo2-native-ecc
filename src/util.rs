use halo2_proofs::circuit::Value;

pub(crate) fn leak<T: Copy + Default>(a: &Value<&T>) -> T {
    let mut t = T::default();
    a.map(|x| t = *x);
    t
}
