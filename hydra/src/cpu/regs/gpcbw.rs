use crate::cpu::{common::ones, regs::RegisterFieldRange};

#[allow(non_camel_case_types)]
pub enum GPCBW_EL3Field {
    BWSIZE = -1,   // 39-37
    BWSTRIDE = -2, // 36-32
    BWADDR = -3,   // 25-0
}

impl RegisterFieldRange<u64, u8> for GPCBW_EL3Field {
    fn fetch_range(self, v: u64) -> u8 {
        if let Self::BWSIZE = self {
            return ((v >> 37) & 0b111) as u8;
        }

        if let Self::BWSTRIDE = self {
            return ((v >> 32) & 0b11111) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u32> for GPCBW_EL3Field {
    fn fetch_range(self, v: u64) -> u32 {
        if let Self::BWADDR = self {
            return (v & ones(26)) as u32;
        }

        unreachable!()
    }
}
