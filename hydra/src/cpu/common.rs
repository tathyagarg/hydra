#[derive(PartialEq, Debug)]
pub struct NZCV(bool, bool, bool, bool);

impl NZCV {
    pub(super) fn N(&self) -> bool {
        self.0
    }

    pub(super) fn Z(&self) -> bool {
        self.1
    }

    pub(super) fn C(&self) -> bool {
        self.2
    }

    pub(super) fn V(&self) -> bool {
        self.3
    }

    pub fn init(n: bool, z: bool, c: bool, v: bool) -> Self {
        Self(n, z, c, v)
    }
}

impl From<u8> for NZCV {
    fn from(value: u8) -> Self {
        Self(
            (value >> 3) & 1 == 1,
            (value >> 2) & 1 == 1,
            (value >> 1) & 1 == 1,
            (value >> 0) & 1 == 1,
        )
    }
}

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
