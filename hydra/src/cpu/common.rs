pub fn nth_bit<T: Into<u64>>(v: T, n: usize) -> bool {
    return (v.into() >> n) & 1 == 1;
}

pub fn ones(w: u32) -> u64 {
    match w {
        0 => 0,
        64 => u64::MAX,
        w => (1u64 << w) - 1,
    }
}

pub fn n_bits<T: Into<u64>>(v: T, n: usize) -> u64 {
    v.into() & ones(n as u32)
}
