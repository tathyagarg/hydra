use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum TCR_EL3Field {
    DisCH0 = 43,
    HAFT = 42,
    PTTWI = 41,
    D128 = 38,
    AIE = 37,
    POE = 36,
    PIE = 35,
    PnCH = 34,
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

impl RegisterField<u64> for TCR_EL3Field {
    fn fetch_bit(self, tcr_el3_v: u64) -> bool {
        nth_bit(tcr_el3_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for TCR_EL3Field {
    fn fetch_range(self, tcr_el3_v: u64) -> u8 {
        (match self {
            TCR_EL3Field::PS => (tcr_el3_v >> 16) & 0b111,
            TCR_EL3Field::TG0 => (tcr_el3_v >> 14) & 0b11,
            TCR_EL3Field::SH0 => (tcr_el3_v >> 12) & 0b11,
            TCR_EL3Field::ORGN0 => (tcr_el3_v >> 10) & 0b11,
            TCR_EL3Field::IRGN0 => (tcr_el3_v >> 8) & 0b11,
            TCR_EL3Field::T0SZ => tcr_el3_v & 0b11111,
            _ => unreachable!(),
        }) as u8
    }
}
