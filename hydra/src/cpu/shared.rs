pub struct NZCV(bool, bool, bool, bool);

fn sint<const N: usize>(n: u64) -> i64 {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };
    let sign_bit = 1u64 << (N - 1);
    (if n & sign_bit != 0 {
        n | !mask
    } else {
        n & mask
    }) as i64
}

pub(super) fn add_with_carry<const N: usize>(x: u64, y: u64, carry_in: bool) -> (u64, NZCV) {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };

    let unsigned_sum = x + y + carry_in as u64;
    let signed_sum = sint::<N>(x) + sint::<N>(y) + carry_in as i64;

    let result = unsigned_sum & mask;

    let n = (result >> (N - 1)) & 1 == 1;
    let z = result == 0;
    let c = result != unsigned_sum;
    let v = result as i64 != signed_sum;

    (result, NZCV(n, z, c, v))
}
