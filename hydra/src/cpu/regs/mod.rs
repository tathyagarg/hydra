use crate::cpu::{accessors::SCTLRType, common::nth_bit};

mod brbcr_el1;
mod brbcr_el2;
mod brbfcr_el0;
mod brbfcr_el1;
mod edscr;
mod hcr_el2;
mod mdcr_el3;
mod mpam0_el1;
mod mpam1_el1;
mod mpam2_el2;
mod mpam3_el3;
mod mpamhcr_el2;
mod mpamidr_el1;
mod mpamsm_el1;
mod mpamvpmv_el2;
mod scr;
mod scr_el3;
mod sctlr_el1;
mod sctlr_el2;
mod sctlr_el3;
mod tcr_el1;

pub use brbcr_el1::BRBCR_EL1Field;
pub use brbcr_el2::BRBCR_EL2Field;
pub use brbfcr_el0::BRBFCR_EL0Field;
pub use brbfcr_el1::BRBFCR_EL1Field;
pub use edscr::EDSCRField;
pub use hcr_el2::HCR_EL2Field;
pub use mdcr_el3::MDCR_EL3Field;
pub use mpam0_el1::MPAM0_EL1Field;
pub use mpam1_el1::MPAM1_EL1Field;
pub use mpam2_el2::MPAM2_EL2Field;
pub use mpam3_el3::MPAM3_EL3Field;
pub use mpamhcr_el2::MPAMHCR_EL2Field;
pub use mpamidr_el1::MPAMIDR_EL1Field;
pub use mpamsm_el1::MPAMSM_EL1Field;
pub use mpamvpmv_el2::MPAMVPMV_EL2Field;
pub use scr::SCRField;
pub use scr_el3::SCR_EL3Field;
// pub use sctlr_el1::SCTLR_EL1Field;
// pub use sctlr_el2::SCTLR_EL2Field;
// pub use sctlr_el3::SCTLR_EL3Field;
pub use tcr_el1::TCR_EL1Field;

pub trait RegisterField<T> {
    fn fetch_bit(self, v: T) -> bool;
}

pub trait RegisterFieldRange<T, S> {
    fn fetch_range(self, v: T) -> S;
}

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

impl RegisterField<u64> for SCTLRField {
    fn fetch_bit(self, sctlr_v: SCTLRType) -> bool {
        nth_bit(sctlr_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for SCTLRField {
    fn fetch_range(self, sctlr_v: SCTLRType) -> u8 {
        if let Self::TCF = self {
            return ((sctlr_v >> 40) & 0b11) as u8;
        }

        unreachable!("??")
    }
}

pub type MPAMVPMnEL2Type = u64;
