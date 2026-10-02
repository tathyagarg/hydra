use crate::cpu::{ExtendType, ShiftType, pstate::PState};

pub struct NZCV(bool, bool, bool, bool);

impl NZCV {
    pub(super) fn set(&self, pstate: &mut PState) {
        pstate.N = self.0;
        pstate.Z = self.1;
        pstate.C = self.2;
        pstate.V = self.3;
    }
}

/// Signed Integer function: Converts unsigned int to signed int
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

    let (unsigned_sum, carry) = {
        let (temp, c1) = x.overflowing_add(y);
        println!("temp: {}, c1: {}", temp, c1);
        let (temp2, c2) = temp.overflowing_add(carry_in as u64);
        (temp2, c1 || c2)
    };
    let signed_sum = sint::<N>(x) + sint::<N>(y) + carry_in as i64;

    let result = unsigned_sum & mask;

    let n = (result >> (N - 1)) & 1 == 1;
    let z = result == 0;
    let c = carry;
    let v = result as i64 != signed_sum;

    println!("nzcv: {} {} {} {}", n, z, c, v);
    println!("unsigned_sum: {}, signed_sum: {}", unsigned_sum, signed_sum);

    (result, NZCV(n, z, c, v))
}

fn extend(val: u64, nbits: usize, unsigned: bool) -> u64 {
    let mask = if nbits == 64 {
        u64::MAX
    } else {
        (1u64 << nbits) - 1
    };

    let value = val & mask;

    if unsigned || nbits == 64 {
        value
    } else {
        let sign_bit = 1u64 << (nbits - 1);

        if value & sign_bit != 0 {
            value | !mask
        } else {
            value
        }
    }
}

pub(super) fn extend_reg<const N: usize>(val: u64, exttype: ExtendType, shift: u8) -> u64 {
    let (unsigned, len): (bool, usize) = match exttype {
        ExtendType::SXTB => (false, 8),
        ExtendType::SXTH => (false, 16),
        ExtendType::SXTW => (false, 32),
        ExtendType::SXTX => (false, 64),
        ExtendType::UXTB => (true, 8),
        ExtendType::UXTH => (true, 16),
        ExtendType::UXTW => (true, 32),
        ExtendType::UXTX => (true, 64),
    };

    let nbits = len.min(N);
    println!("Extending {} to {} bits", val, nbits);
    let extval = extend(val, nbits, unsigned);

    return extval << shift;
}

pub(super) fn shift_reg(val: u64, shift_type: ShiftType, amount: usize) -> u64 {
    match shift_type {
        ShiftType::LSL => val << amount,
        ShiftType::LSR => val >> amount,
        ShiftType::ASR => (val as i64 >> amount) as u64,
        ShiftType::ROR => val,
    }
}
