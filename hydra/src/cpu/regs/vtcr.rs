use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum VTCR_EL2Field {
    HDBSS = 45,
    HAFT = 44,
    TL0 = 41,
    GCSH = 40,
    D128 = 38,
    S2POE = 37,
    S2PIE = 36,
    TL1 = 35,
    AssuredOnly = 34,
    SL2 = 33,
    DS = 32,
    NSA = 30,
    NSW = 29,
    HWU62 = 28,
    HWU61 = 27,
    HWU60 = 26,
    HWU59 = 25,
    HD = 22,
    HA = 21,
    VS = 19,
    PS = -1,    // 18-16
    TG0 = -2,   // 15-14
    SH0 = -3,   // 13-12
    ORGN0 = -4, // 11-10
    IRGN0 = -5, // 9-8
    SL0 = -6,   // 7-6
    T0SZ = -7,  // 5-0
}

impl RegisterField<u64> for VTCR_EL2Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for VTCR_EL2Field {
    fn fetch_range(self, v: u64) -> u8 {
        (match self {
            Self::PS => (v >> 16) & 0b111,
            Self::TG0 => (v >> 14) & 0b11,
            Self::SH0 => (v >> 12) & 0b11,
            Self::ORGN0 => (v >> 10) & 0b11,
            Self::IRGN0 => (v >> 8) & 0b11,
            Self::SL0 => (v >> 6) & 0b11,
            Self::T0SZ => v & 0b11111,
            _ => unreachable!(),
        }) as u8
    }
}
