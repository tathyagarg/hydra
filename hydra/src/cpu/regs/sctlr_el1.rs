use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum SCTLR_EL1Field {
    TIDCP = 63,
    SPINTMASK = 62,
    NMI = 61,
    EnTP2 = 60,
    TCSO = 59,
    TCSO0 = 58,
    EPAN = 57,
    EnALS = 56,
    EnAS0 = 55,
    EnASR = 54,
    TWEDEL = -1, // 49-46
    TWEDEn = 45,
    DSSBS = 44,
    ATA = 43,
    ATA0 = 42,
    TCF = -2,  // 41-40
    TCF0 = -3, // 39-38
    ITFSB = 37,
    BT1 = 36,
    BT0 = 35,
    EnFPM = 34,
    MSCEn = 33,
    CMOW = 32,
    EnIA = 31,
    EnIB = 30,
    LSMAOE = 29,
    nTLSMD = 28,
    EnDA = 27,
    UCI = 26,
    EE = 25,
    E0E = 24,
    SPAN = 23,
    EIS = 22,
    IESB = 21,
    TSCXT = 20,
    WXN = 19,
    nTWE = 18,
    nTWI = 16,
    UCT = 15,
    DZE = 14,
    EnDB = 13,
    I = 12,
    EOS = 11,
    EnRCTX = 10,
    UMA = 9,
    SED = 8,
    ITD = 7,
    nAA = 6,
    CP15BEN = 5,
    SA0 = 4,
    SA = 3,
    C = 2,
    A = 1,
    M = 0,
}

impl RegisterField<u64> for SCTLR_EL1Field {
    fn fetch_bit(self, sctlr_el1_v: u64) -> bool {
        nth_bit(sctlr_el1_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for SCTLR_EL1Field {
    fn fetch_range(self, sctlr_el1_v: u64) -> u8 {
        if let Self::TWEDEL = self {
            return ((sctlr_el1_v >> 46) & 0b1111) as u8;
        }

        if let Self::TCF = self {
            return ((sctlr_el1_v >> 40) & 0b11) as u8;
        }

        if let Self::TCF0 = self {
            return ((sctlr_el1_v >> 38) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
