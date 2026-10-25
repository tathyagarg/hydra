use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

pub enum HCRField {
    TRVM = 30,
    HCD = 29,
    TGE = 27,
    TVM = 26,
    TTLB = 25,
    TPU = 24,
    TPC = 23,
    TSW = 22,
    TAC = 21,
    TIDCP = 20,
    TSC = 19,
    TID3 = 18,
    TID2 = 17,
    TID1 = 16,
    TID0 = 15,
    TWE = 14,
    TWI = 13,
    DC = 12,
    BSU = -1, // 11-10
    FB = 9,
    VA = 8,
    VI = 7,
    VF = 6,
    AMO = 5,
    IMO = 4,
    FMO = 3,
    PTW = 2,
    SWIO = 1,
    VM = 0,
}

impl RegisterField<u32> for HCRField {
    fn fetch_bit(self, v: u32) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u32, u8> for HCRField {
    fn fetch_range(self, v: u32) -> u8 {
        if let Self::BSU = self {
            return ((v >> 10) & 0b11) as u8;
        }

        unreachable!()
    }
}
