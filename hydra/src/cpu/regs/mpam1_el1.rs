use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum MPAM1_EL1Field {
    MPAMEN = 63,
    FORCED_NS = 60,
    ALTSP_FRCD = 54,
    PMG_D = -1,    // 47-40
    PMG_I = -2,    // 39-32
    PARTID_D = -3, // 31-16
    PARTID_I = -4, // 15-0
}

impl RegisterField<u64> for MPAM1_EL1Field {
    fn fetch_bit(self, mpam1_el1_v: u64) -> bool {
        nth_bit(mpam1_el1_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for MPAM1_EL1Field {
    fn fetch_range(self, mpam1_el1_v: u64) -> u8 {
        if let Self::PMG_D = self {
            return ((mpam1_el1_v >> 40) & 0xFF) as u8;
        }

        if let Self::PMG_I = self {
            return ((mpam1_el1_v >> 32) & 0xFF) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u16> for MPAM1_EL1Field {
    fn fetch_range(self, mpam1_el1_v: u64) -> u16 {
        if let Self::PARTID_D = self {
            return ((mpam1_el1_v >> 16) & 0xFFFF) as u16;
        }

        if let Self::PARTID_I = self {
            return (mpam1_el1_v & 0xFFFF) as u16;
        }

        unreachable!()
    }
}
