use crate::cpu::common::nth_bit;

#[allow(non_camel_case_types)]
pub enum BRBCR_EL1Field {
    EXCEPTION = 23,
    ERTW = 22,
    FZPSS = 9,
    FZP = 8,
    TS = -1, // 6-5
    MPRED = 4,
    CC = 3,
    E1BRE = 1,
    E0BRE = 0,
}

impl BRBCR_EL1Field {
    pub fn fetch_bit(self, brbcr_el1_v: u64) -> bool {
        nth_bit(brbcr_el1_v, self as usize)
    }

    pub fn fetch_range(self, brbcr_el1_v: u64) -> u8 {
        if let Self::TS = self {
            return ((brbcr_el1_v >> 5) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
