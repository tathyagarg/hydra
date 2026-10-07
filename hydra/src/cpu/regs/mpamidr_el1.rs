use crate::cpu::common::nth_bit;

#[allow(non_camel_case_types)]
pub enum MPAMIDR_EL1Field {
    HAS_SDEFLT = 61,
    HAS_FORCE_NS = 60,
    SP4 = 59,
    HAS_TIDR = 58,
    HAS_ALTSP = 57,
    HAS_BW_CTRL = 56,
    PMG_MAX = -1,  // 39-32
    VPMR_MAX = -2, // 20-18
    HAS_HCR = 17,
    PARTID_MAX = -3, // 15-0
}

impl MPAMIDR_EL1Field {
    pub fn fetch_bit(self, mpamidr_el1_v: u64) -> bool {
        nth_bit(mpamidr_el1_v, self as usize)
    }

    pub fn fetch_range(self, mpamidr_el1_v: u64) -> u16 {
        if let Self::PMG_MAX = self {
            return ((mpamidr_el1_v >> 32) & 0xFF) as u16;
        }

        if let Self::VPMR_MAX = self {
            return ((mpamidr_el1_v >> 18) & 0b11) as u16;
        }

        if let Self::PARTID_MAX = self {
            return (mpamidr_el1_v & 0xFFFF) as u16;
        }

        unreachable!()
    }
}
