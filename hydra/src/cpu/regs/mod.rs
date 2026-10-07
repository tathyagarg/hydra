use crate::cpu::{accessors::SCTLRType, common::nth_bit};

pub mod brbcr_el1;
pub mod brbcr_el2;
pub mod brbfcr_el0;
pub mod brbfcr_el1;
pub mod edscr;
pub mod hcr_el2;
pub mod mdcr_el3;
pub mod mpamidr_el1;
pub mod scr;
pub mod scr_el3;
pub mod sctlr_el1;
pub mod sctlr_el2;
pub mod sctlr_el3;
pub mod tcr_el1;

/// Common fields in SCTLR registers
#[allow(non_camel_case_types)]
pub enum SCTLRField {
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

impl SCTLRField {
    pub fn fetch_bit(self, sctlr_v: SCTLRType) -> bool {
        nth_bit(sctlr_v, self as usize)
    }

    pub fn fetch_range(self, sctlr_v: SCTLRType) -> u8 {
        if let Self::TCF = self {
            return ((sctlr_v >> 40) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
