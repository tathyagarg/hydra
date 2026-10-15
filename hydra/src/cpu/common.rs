use crate::cpu::{
    fault::Fault,
    memory_acc::{AddressDescriptor, PhysMemRetStatus},
};

pub const FINAL_LEVEL: u8 = 3;
pub const NUM_PABITS: u8 = 56;

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

pub fn nth_bit<T: Into<u128>>(v: T, n: usize) -> bool {
    return (v.into() >> n) & 1 == 1;
}

pub fn get_bits<T: Into<u128>>(v: T, x: usize, y: usize) -> u128 {
    let mask = if y - x + 1 == 128 {
        u128::MAX
    } else {
        (1u128 << (y - x + 1)) - 1
    };

    (v.into() >> x) & mask
}

pub trait BitInteger:
    Copy + std::ops::Shl<u32, Output = Self> + std::ops::Sub<Output = Self>
{
    const BITS: u32;
    const ZERO: Self;
    const MAX: Self;
    fn one() -> Self;
}

impl BitInteger for u64 {
    const BITS: u32 = 64;
    const ZERO: Self = 0;
    const MAX: Self = u64::MAX;
    fn one() -> Self {
        1
    }
}

impl BitInteger for u128 {
    const BITS: u32 = 128;
    const ZERO: Self = 0;
    const MAX: Self = u128::MAX;
    fn one() -> Self {
        1
    }
}

pub fn ones_as<T: BitInteger>(w: u32) -> T {
    match w {
        0 => T::ZERO,
        w if w >= T::BITS => T::MAX,
        w => (T::one() << w) - T::one(),
    }
}

pub fn ones(w: u32) -> u64 {
    ones_as::<u64>(w)
}

pub fn n_bits<T: Into<u64>>(v: T, n: usize) -> u64 {
    v.into() & ones(n as u32)
}

pub fn count_leading_sign_bits<const N: usize>(value: u64) -> u32 {
    let differing = (value ^ (value >> 1)) & (ones(N as u32) >> 1);

    differing.leading_zeros() as u32 - (64 - (N - 1) as u32)
}

pub fn count_leading_zero_bits<const N: usize>(value: u64) -> u32 {
    value.leading_zeros() as u32 - (64 - N as u32)
}

pub fn is_aligned_size(address: u64, size: u64) -> bool {
    address % size == 0
}

pub fn is_fault_addrdesc(addrdesc: &AddressDescriptor) -> bool {
    addrdesc.fault.statuscode != Fault::None
}

pub fn is_fault_fault(fault: &Fault) -> bool {
    *fault != Fault::None
}

pub fn is_fault_retstatus(retstatus: &PhysMemRetStatus) -> bool {
    retstatus.statuscode != Fault::None
}

pub fn align_down_p2<const N: u32>(x: u64, p2: u32) -> u64 {
    assert!(N <= 64 && p2 <= N);
    x & !ones_as::<u64>(p2) & ones_as::<u64>(N)
}

pub fn align_down_p2_as<T>(x: T, p2: u32) -> T
where
    T: BitInteger + std::ops::BitAnd<Output = T> + std::ops::Not<Output = T>,
{
    assert!(p2 <= T::BITS);

    if p2 == 0 {
        return x;
    }

    if p2 == T::BITS {
        return T::ZERO;
    }

    x & !(ones_as::<T>(p2))
}
