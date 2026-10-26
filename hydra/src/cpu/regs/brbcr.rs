use crate::cpu::regs::RegisterFieldRange;

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

impl RegisterFieldRange<u64, u8> for BRBCR_EL1Field {
    fn fetch_range(self, brbcr_el1_v: u64) -> u8 {
        if let Self::TS = self {
            return ((brbcr_el1_v >> 5) & 0b11) as u8;
        }

        unreachable!("??")
    }
}

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

impl RegisterFieldRange<u64, u8> for BRBCR_EL2Field {
    fn fetch_range(self, brbcr_el2_v: u64) -> u8 {
        if let Self::TS = self {
            return ((brbcr_el2_v >> 5) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
