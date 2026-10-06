use crate::cpu::common::nth_bit;

#[allow(non_camel_case_types)]
pub enum SCTLR_EL3Field {
    SPINTMASK = 62,
    NMI = 61,
    TCSO = 59,
    DSSBS = 44,
    ATA = 43,
    TCF = -2, // 41-40
    ITFSB = 37,
    BT = 36,
    EnIA = 31,
    EnIB = 30,
    EnDA = 27,
    EE = 25,
    EIS = 22,
    IESB = 21,
    WXN = 19,
    EnDB = 13,
    I = 12,
    EOS = 11,
    nAA = 6,
    SA = 3,
    C = 2,
    A = 1,
    M = 0,
}

impl SCTLR_EL3Field {
    pub fn fetch_bit(self, sctlr_el3_v: u64) -> bool {
        nth_bit(sctlr_el3_v, self as usize)
    }

    pub fn fetch_range(self, sctlr_el3_v: u64) -> u8 {
        if let Self::TCF = self {
            return ((sctlr_el3_v >> 40) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
