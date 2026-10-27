mod brbcr;
mod brbfcr;
mod dbgbcr_n;
mod dbgwcr_n;
mod edscr;
mod gpcbw;
mod gpccr;
mod gptbr;
mod hcr;
mod hcr2;
mod hcr_el2;
mod hcrx;
mod mdcr;
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
mod ttbr0;
mod ttbr1;
mod vtcr;

use crate::cpu::common::nth_bit;

pub use self::brbcr::{BRBCR_EL1Field, BRBCR_EL2Field};
pub use self::brbfcr::{BRBFCR_EL0Field, BRBFCR_EL1Field};
pub use self::dbgbcr_n::DBGBCR_N_EL1Field;
pub use self::dbgwcr_n::DBGWCR_N_EL1Field;
pub use self::edscr::EDSCRField;
pub use self::gpcbw::GPCBW_EL3Field;
pub use self::gpccr::GPCCR_EL3Field;
pub use self::gptbr::GPTBR_EL3Field;
pub use self::hcr::HCRField;
pub use self::hcr_el2::HCR_EL2Field;
pub use self::hcr2::HCR2Field;
pub use self::hcrx::HCRX_EL2Field;
pub use self::mdcr::{MDCR_EL2Field, MDCR_EL3Field};
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
#[allow(unused_imports)]
pub use self::sctlr::{SCTLR_EL1Field, SCTLR_EL2Field, SCTLR_EL3Field, SCTLRField};
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
pub use self::tcr2::{E2H_TCR2_EL2Field, TCR2_EL1Field, TCR2_EL2Field};
#[allow(unused_imports)]
pub use self::ttbr0::{
    TTBR0_64_EL1Field, TTBR0_64_EL2Field, TTBR0_64_EL3Field, TTBR0_128_EL1Field,
    TTBR0_128_EL2Field, TTBR0_128_EL3Field,
};
#[allow(unused_imports)]
pub use self::ttbr1::{
    TTBR1_64_EL1Field, TTBR1_64_EL2Field, TTBR1_128_EL1Field, TTBR1_128_EL2Field,
};
pub use self::vtcr::VTCR_EL2Field;

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
pub type S1PORType = [u8; 16];

pub type MPAMVPMnEL2Type = u64;

#[allow(non_camel_case_types)]
pub enum MECID_RL_A_EL3Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for MECID_RL_A_EL3Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum MECID_A0_EL2Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for MECID_A0_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum MECID_A1_EL2Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for MECID_A1_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum MECID_P0_EL2Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for MECID_P0_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum MECID_P1_EL2Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for MECID_P1_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum VMECID_P_EL2Field {
    MECID = -1, // 15-0
}

impl RegisterFieldRange<u64, u16> for VMECID_P_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        (v & 0xFFFF) as u16
    }
}

#[allow(non_camel_case_types)]
pub enum OSLSR_EL1Field {
    nTT = 2,
    OSLK = 1,
    OSLM = -1,
}

impl RegisterFieldRange<u64, u8> for OSLSR_EL1Field {
    fn fetch_range(self, v: u64) -> u8 {
        if let Self::OSLM = self {
            return ((nth_bit(v, 3) as u8) << 1) | (nth_bit(v, 0) as u8);
        }

        unreachable!()
    }
}

macro_rules! register_field {
    ($($ty:ty $( {$value_type:ty} )?),* $(,)?) => {
        $(
            impl RegisterField<register_field!(@type $($value_type)?)> for $ty {
                fn fetch_bit(self, v: register_field!(@type $($value_type)?)) -> bool {
                    nth_bit(v, self as usize)
                }
            }
        )*
    };

    (@type $ty:ty) => { $ty };
    (@type) => { u32 };
}

pub enum DBGOSDLRField {
    DLK = 0,
}

#[allow(non_camel_case_types)]
pub enum OSDLR_EL1Field {
    DLK = 0,
}

pub enum DBGPRCRField {
    CORENPDRQ = 0,
}

#[allow(non_camel_case_types)]
pub enum DBGPRCR_EL1Field {
    CORENPDRQ = 0,
}

#[allow(non_camel_case_types)]
pub enum SDER32_EL3Field {
    SUNIDEN = 1,
    SUIDEN = 0,
}

#[allow(non_camel_case_types)]
pub enum MDSCR_EL1Field {
    EnSTEPOP = 50,
    EHBWE = 35,
    EnSPM = 34,
    TTA = 33,
    EMBWE = 32,
    TFO = 31,
    RXfull = 30,
    TXfull = 29,
    RXO = 27,
    TXU = 26,
    INTdis = -1, // 23-22
    TDA = 21,
    SC2 = 19,
    MDE = 15,
    HDE = 14,
    KDE = 13,
    TDCC = 12,
    ERR = 6,
    SS = 0,
}

impl RegisterFieldRange<u64, u8> for MDSCR_EL1Field {
    fn fetch_range(self, v: u64) -> u8 {
        if let Self::INTdis = self {
            return ((v >> 22) & 0b11) as u8;
        }

        unreachable!()
    }
}

pub enum EDSCR2Field {
    EHBWE = 3,
    TTA = 1,
}

register_field!(
    BRBCR_EL1Field { u64 },
    BRBCR_EL2Field { u64 },
    BRBFCR_EL0Field { u64 },
    BRBFCR_EL1Field { u64 },
    DBGOSDLRField { u32 },
    DBGPRCRField { u32 },
    DBGPRCR_EL1Field { u64 },
    EDSCRField { u32 },
    GPCCR_EL3Field { u64 },
    HCRField { u32 },
    HCR2Field { u32 },
    HCR_EL2Field { u64 },
    HCRX_EL2Field { u64 },
    MDCR_EL2Field { u64 },
    MDCR_EL3Field { u64 },
    MDSCR_EL1Field { u64 },
    MPAM1_EL1Field { u64 },
    MPAM2_EL2Field { u64 },
    MPAM3_EL3Field { u64 },
    MPAMHCR_EL2Field { u64 },
    MPAMIDR_EL1Field { u64 },
    MPAMVPMV_EL2Field { u64 },
    OSDLR_EL1Field { u64 },
    OSLSR_EL1Field { u64 },
    SCRField { u32 },
    SCR_EL3Field { u64 },
    SCTLR_EL1Field { u64 },
    SCTLR_EL2Field { u64 },
    SCTLR_EL3Field { u64 },
    SCTLRField { u64 },
    SCTLR2_EL1Field { u64 },
    SCTLR2_EL2Field { u64 },
    SCTLR2_EL3Field { u64 },
    SDER32_EL3Field { u64 },
    TCR_EL1Field { u64 },
    E2H0_TCR_EL2Field { u64 },
    E2H1_TCR_EL2Field { u64 },
    TCR_EL3Field { u64 },
    TCR2_EL1Field { u64 },
    TCR2_EL2Field { u64 },
    E2H_TCR2_EL2Field { u64 },
    TTBR0_128_EL1Field { u128 },
    TTBR0_64_EL1Field { u64 },
    TTBR0_128_EL2Field { u128 },
    TTBR0_64_EL2Field { u64 },
    TTBR0_128_EL3Field { u128 },
    TTBR0_64_EL3Field { u64 },
    TTBR1_128_EL1Field { u128 },
    TTBR1_64_EL1Field { u64 },
    TTBR1_128_EL2Field { u128 },
    TTBR1_64_EL2Field { u64 },
    VTCR_EL2Field { u64 },
    EDSCR2Field { u32 },
    DBGWCR_N_EL1Field { u64 },
    DBGBCR_N_EL1Field { u64 }
);
