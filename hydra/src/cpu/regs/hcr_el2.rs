use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum HCR_EL2Field {
    TWEDEL = -1, // 63-60
    TWEDEn = 59,
    TID5 = 58,
    DCT = 57,
    ATA = 56,
    TTLBOS = 55,
    TTLBIS = 54,
    EnSCXT = 53,
    TOCU = 52,
    AMVOFFEN = 51,
    TICAB = 50,
    TID = 49,
    GPF = 48,
    FIEN = 47,
    FWB = 46,
    NV2 = 45,
    AT = 44,
    NV1 = 43,
    NV = 42,
    API = 41,
    APK = 40,
    TEA = 37,
    TERR = 36,
    TLOR = 35,
    E2H = 34,
    ID = 33,
    CD = 32,
    RW = 31,
    TRVM = 30,
    HCD = 29,
    TDZ = 28,
    TGE = 27,
    TVM = 26,
    TTLB = 25,
    TPU = 24,
    TPCP = 23,
    TSW = 22,
    TACR = 21,
    TIDCP = 20,
    TSC = 19,
    TID3 = 18,
    TID2 = 17,
    TID1 = 16,
    TID0 = 15,
    TWE = 14,
    TWI = 13,
    DC = 12,
    BSU = -2, // 11-10
    FB = 9,
    VSE = 8,
    VI = 7,
    VF = 6,
    AMO = 5,
    IMO = 4,
    FMO = 3,
    PTW = 2,
    SWIO = 1,
    VM = 0,
}

impl RegisterField<u64> for HCR_EL2Field {
    fn fetch_bit(self, hcr_el2_v: u64) -> bool {
        nth_bit(hcr_el2_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for HCR_EL2Field {
    fn fetch_range(self, hcr_el2_v: u64) -> u8 {
        if let Self::TWEDEL = self {
            return ((hcr_el2_v >> 60) & 0b1111) as u8;
        }

        if let Self::BSU = self {
            return ((hcr_el2_v >> 10) & 0b11) as u8;
        }

        unreachable!("??")
    }
}
