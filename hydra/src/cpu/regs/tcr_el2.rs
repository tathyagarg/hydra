use crate::cpu::{
    common::*,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum TCR_EL2Field {
    MTX = 33,
    DS = 32,
    TCMA = 30,
    TBID = 29,
    HWU62 = 28,
    HWU61 = 27,
    HWU60 = 26,
    HWU59 = 25,
    HPD = 24,
    HD = 22,
    HA = 21,
    TBI = 20,
    PS = -1,    // 18-16
    TG0 = -2,   // 15-14
    SH0 = -3,   // 13-12
    ORGN0 = -4, // 11-10
    IRGN0 = -5, // 9-8
    T0SZ = -6,  // 5-0
}

impl RegisterField<u64> for TCR_EL2Field {
    fn fetch_bit(self, tcr_el2_v: u64) -> bool {
        nth_bit(tcr_el2_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for TCR_EL2Field {
    fn fetch_range(self, tcr_el2_v: u64) -> u8 {
        (match self {
            Self::PS => (tcr_el2_v >> 16) & 0b111,
            Self::TG0 => (tcr_el2_v >> 14) & 0b11,
            Self::SH0 => (tcr_el2_v >> 12) & 0b11,
            Self::ORGN0 => (tcr_el2_v >> 10) & 0b11,
            Self::IRGN0 => (tcr_el2_v >> 8) & 0b11,
            Self::T0SZ => tcr_el2_v & 0b111111,
            _ => unreachable!(),
        }) as u8
    }
}
