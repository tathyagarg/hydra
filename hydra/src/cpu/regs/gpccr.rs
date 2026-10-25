use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum GPCCR_EL3Field {
    GPCBW = 29,
    NA7 = 28,
    NA6 = 27,
    NSP = 26,
    SA = 25,
    APPSAA = 24,
    L0GPTSZ = -1, // 23-20
    NSO = 19,
    TBGPCD = 18,
    GPCP = 17,
    GPC = 16,
    PGS = -2,  // 15-14
    SH = -3,   // 13-12
    ORGN = -4, // 11-10
    IRGN = -5, // 9-8
    SPAD = 7,
    NSPAD = 6,
    RLPAD = 5,
    PPS3 = 3,
    PPS = -6, // 2-0
}

impl RegisterField<u64> for GPCCR_EL3Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for GPCCR_EL3Field {
    fn fetch_range(self, v: u64) -> u8 {
        (match self {
            Self::L0GPTSZ => (v >> 20) & 0b1111,
            Self::PGS => (v >> 14) & 0b11,
            Self::SH => (v >> 12) & 0b11,
            Self::ORGN => (v >> 10) & 0b11,
            Self::IRGN => (v >> 8) & 0b11,
            Self::PPS => v & 0b11,
            _ => unreachable!(),
        }) as u8
    }
}
