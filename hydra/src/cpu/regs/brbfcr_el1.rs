use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum BRBFCR_EL1Field {
    BANK = -1, // 29-28
    CONDDIR = 22,
    DIRCALL = 21,
    INDCALL = 20,
    RTN = 19,
    INDIRECT = 18,
    DIRECT = 17,
    EnI = 16,
    PAUSED = 7,
}

impl RegisterField<u64> for BRBFCR_EL1Field {
    fn fetch_bit(self, brbfcr_el1_v: u64) -> bool {
        nth_bit(brbfcr_el1_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for BRBFCR_EL1Field {
    fn fetch_range(self, brbfcr_el1_v: u64) -> u8 {
        if let Self::BANK = self {
            return ((brbfcr_el1_v >> 28) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
