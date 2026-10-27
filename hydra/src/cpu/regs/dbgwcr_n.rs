use crate::cpu::regs::RegisterFieldRange;

#[allow(non_camel_case_types)]
pub enum DBGWCR_N_EL1Field {
    LBNX = -1, // 31-30
    SSCE = 29,
    MASK = -2, // 28-24
    WT2 = 22,
    WT = 20,
    LBN = -3, // 19-16
    SSC = -4, // 15-14
    HMC = 13,
    BAS = -5, // 12-5
    LSC = -6, // 4-3
    PAC = -7, // 2-1
    E = 0,
}

impl RegisterFieldRange<u64, u8> for DBGWCR_N_EL1Field {
    fn fetch_range(self, v: u64) -> u8 {
        (match self {
            Self::LBNX => ((v >> 30) & 0b11) as u8,
            Self::MASK => ((v >> 24) & 0b11111) as u8,
            Self::LBN => ((v >> 16) & 0b1111) as u8,
            Self::SSC => ((v >> 14) & 0b11) as u8,
            Self::BAS => ((v >> 5) & 0xff) as u8,
            Self::LSC => ((v >> 3) & 0b11) as u8,
            Self::PAC => ((v >> 1) & 0b11) as u8,
            _ => unreachable!(),
        }) as u8
    }
}

