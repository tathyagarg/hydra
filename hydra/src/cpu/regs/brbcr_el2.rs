use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum BRBCR_EL2Field {
    EXCEPTION = 23,
    ERTN = 22,
    FZPSS = 9,
    FZP = 8,
    TS = -1, // 6-5
    MPRED = 4,
    CC = 3,
    E2BRE = 1,
    E0HBRE = 0,
}

impl RegisterField<u64> for BRBCR_EL2Field {
    fn fetch_bit(self, brbcr_el2_v: u64) -> bool {
        nth_bit(brbcr_el2_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for BRBCR_EL2Field {
    fn fetch_range(self, brbcr_el2_v: u64) -> u8 {
        if let Self::TS = self {
            return ((brbcr_el2_v >> 5) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
