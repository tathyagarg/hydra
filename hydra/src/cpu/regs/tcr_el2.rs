use crate::cpu::{
    common::*,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum E2H0_TCR_EL2Field {
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

impl RegisterField<u64> for E2H0_TCR_EL2Field {
    fn fetch_bit(self, tcr_el2_v: u64) -> bool {
        nth_bit(tcr_el2_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for E2H0_TCR_EL2Field {
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

#[allow(non_camel_case_types)]
pub enum E2H1_TCR_EL2Field {
    MTX1 = 62,
    MTX0 = 61,
    DS = 59,
    TCMA1 = 58,
    TCMA0 = 57,
    E0PD1 = 56,
    E0PD0 = 55,
    NFD1 = 54,
    NFD0 = 53,
    TBID1 = 52,
    TBID0 = 51,
    HWU162 = 50,
    HWU161 = 49,
    HWU160 = 48,
    HWU159 = 47,
    HWU062 = 46,
    HWU061 = 45,
    HWU060 = 44,
    HWU059 = 43,
    HPD1 = 42,
    HPD0 = 41,
    HD = 40,
    HA = 39,
    TBI1 = 38,
    TBI0 = 37,
    AS = 36,
    IPS = -1,   // 34-32
    TG1 = -2,   // 31-30
    SH1 = -3,   // 29-28
    ORGN1 = -4, // 27-26
    IRGN1 = -5, // 25-24
    EPD1 = 23,
    A1 = 22,
    T1SZ = -6,   // 20-16
    TG0 = -7,    // 15-14
    SH0 = -8,    // 13-12
    ORGN0 = -9,  // 11-10
    IRGN0 = -10, // 9-8
    EPD0 = 7,
    T0SZ = -11, // 5-0
}

impl RegisterField<u64> for E2H1_TCR_EL2Field {
    fn fetch_bit(self, tcr_el2_v: u64) -> bool {
        nth_bit(tcr_el2_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for E2H1_TCR_EL2Field {
    fn fetch_range(self, tcr_el2_v: u64) -> u8 {
        (match self {
            Self::IPS => (tcr_el2_v >> 32) & 0b111,
            Self::TG1 => (tcr_el2_v >> 30) & 0b11,
            Self::SH1 => (tcr_el2_v >> 28) & 0b11,
            Self::ORGN1 => (tcr_el2_v >> 26) & 0b11,
            Self::IRGN1 => (tcr_el2_v >> 24) & 0b11,
            Self::T1SZ => (tcr_el2_v >> 16) & 0b11111,
            Self::TG0 => (tcr_el2_v >> 14) & 0b11,
            Self::SH0 => (tcr_el2_v >> 12) & 0b11,
            Self::ORGN0 => (tcr_el2_v >> 10) & 0b11,
            Self::IRGN0 => (tcr_el2_v >> 8) & 0b11,
            Self::T0SZ => tcr_el2_v & 0b111111,
            _ => unreachable!(),
        }) as u8
    }
}

#[allow(non_camel_case_types)]
pub enum TCR_EL2CommonField {
    TG0 = -2,   // 15-14
    SH0 = -3,   // 13-12
    ORGN0 = -4, // 11-10
    IRGN0 = -5, // 9-8
    T0SZ = -6,  // 5-0
}

impl RegisterFieldRange<u64, u8> for TCR_EL2CommonField {
    fn fetch_range(self, tcr_el2_v: u64) -> u8 {
        (match self {
            Self::TG0 => (tcr_el2_v >> 14) & 0b11,
            Self::SH0 => (tcr_el2_v >> 12) & 0b11,
            Self::ORGN0 => (tcr_el2_v >> 10) & 0b11,
            Self::IRGN0 => (tcr_el2_v >> 8) & 0b11,
            Self::T0SZ => tcr_el2_v & 0b111111,
        }) as u8
    }
}
