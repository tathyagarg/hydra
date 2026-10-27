use crate::cpu::regs::RegisterFieldRange;

#[allow(non_camel_case_types)]
pub enum DBGBCR_N_EL1Field {
    LBNX = -1, // 31-30
    SSCE = 29,
    MASK = -2, // 28-24
    BT = -3,   // 23-20
    LBN = -4,  // 19-16
    SSC = -5,  // 15-14
    HMC = 13,
    BAS = -6, // 8-5
    BT2 = 3,
    PMC = -7, // 2-1
    E = 0,
}

impl RegisterFieldRange<u64, u8> for DBGBCR_N_EL1Field {
    fn fetch_range(self, v: u64) -> u8 {
        match self {
            Self::LBNX => ((v >> 30) & 0b11) as u8,
            Self::MASK => ((v >> 24) & 0b11111) as u8,
            Self::BT => ((v >> 20) & 0b1111) as u8,
            Self::LBN => ((v >> 16) & 0b1111) as u8,
            Self::SSC => ((v >> 14) & 0b11) as u8,
            Self::BAS => ((v >> 5) & 0b1111) as u8,
            Self::PMC => ((v >> 1) & 0b11) as u8,
            _ => unreachable!(),
        }
    }
}
