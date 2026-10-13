use crate::cpu::{accessors::SCTLRType, common::nth_bit};

mod brbcr;
mod brbfcr;
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
mod sctlr;
mod sctlr2;
mod tcr;
mod tcr2;

pub use self::brbcr::{BRBCR_EL1Field, BRBCR_EL2Field};
pub use self::brbfcr::{BRBFCR_EL0Field, BRBFCR_EL1Field};
pub use self::edscr::EDSCRField;
pub use self::hcr_el2::HCR_EL2Field;
pub use self::mdcr_el3::MDCR_EL3Field;
pub use self::mpam0_el1::MPAM0_EL1Field;
pub use self::mpam1_el1::MPAM1_EL1Field;
pub use self::mpam2_el2::MPAM2_EL2Field;
pub use self::mpam3_el3::MPAM3_EL3Field;
pub use self::mpamhcr_el2::MPAMHCR_EL2Field;
pub use self::mpamidr_el1::MPAMIDR_EL1Field;
pub use self::mpamsm_el1::MPAMSM_EL1Field;
pub use self::mpamvpmv_el2::MPAMVPMV_EL2Field;
pub use self::scr::SCRField;
pub use self::scr_el3::SCR_EL3Field;
pub use self::sctlr::SCTLRField;
// pub use self::sctlr_el1::SCTLR_EL1Field;
// pub use self::sctlr_el2::SCTLR_EL2Field;
// pub use self::sctlr_el3::SCTLR_EL3Field;
// pub use self::sctlr2_el1::SCTLR2_EL1Field;
#[allow(unused_imports)]
pub use self::sctlr2::{SCTLR2_EL1Field, SCTLR2_EL2Field, SCTLR2_EL3Field};
pub use self::tcr::{
    E2H0_TCR_EL2Field, E2H1_TCR_EL2Field, TCR_EL1Field, TCR_EL2CommonField, TCR_EL3Field,
};
#[allow(unused_imports)]
pub use self::tcr2::{TCR2_EL1Field, TCR2_EL2Field};

pub trait RegisterField<T> {
    fn fetch_bit(self, v: T) -> bool;
}

pub trait RegisterFieldRange<T, S> {
    fn fetch_range(self, v: T) -> S;
}

pub type MAIRType = [u8; 8];

pub fn mairtype_from_mair(mair: u64) -> MAIRType {
    mair.to_be_bytes()
}

pub type S1PIRType = [u8; 16];

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

pub type PIRType = [u8; 16];
