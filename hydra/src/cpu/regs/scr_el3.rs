use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum SCR_EL3Field {
    NSE = 62,
    HACDBSEn = 61,
    HDBSSEn = 60,
    FGTEn2 = 59,
    EnDSE = 58,
    DSE = 57,
    EnIDCP128 = 55,
    SRMASKEn = 54,
    PFAREn = 53,
    TWERR = 52,
    TMEA = 51,
    EnFPM = 50,
    MECEn = 49,
    GPF = 48,
    D128En = 47,
    AIEn = 46,
    PIEn = 45,
    SCTLR2En = 44,
    TCR2En = 43,
    RCWMASKEn = 42,
    EnTP2 = 41,
    TRNDR = 40,
    GCSEn = 39,
    HXEn = 38,
    ADEn = 37,
    EnAS = 36,
    AMVOFFEN = 35,
    TWEDEL = -1, // 33-30
    TWEDEn = 29,
    ECVEn = 28,
    FGTEn = 27,
    ATA = 26,
    EnSCXT = 25,
    TID5 = 23,
    TID3 = 22,
    FIEN = 21,
    NMEA = 20,
    EASE = 19,
    EEL2 = 18,
    API = 17,
    APK = 16,
    TERR = 15,
    TLOR = 14,
    TWE = 13,
    TWI = 12,
    ST = 11,
    RW = 10,
    SIF = 9,
    HCE = 8,
    SMD = 7,
    EA = 3,
    FIQ = 2,
    IRQ = 1,
    NS = 0,
}

impl RegisterField<u64> for SCR_EL3Field {
    fn fetch_bit(self, scr_el3_v: u64) -> bool {
        nth_bit(scr_el3_v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for SCR_EL3Field {
    fn fetch_range(self, scr_el3_v: u64) -> u8 {
        if let Self::TWEDEL = self {
            return ((scr_el3_v >> 30) & 0b1111) as u8;
        }

        unreachable!("??")
    }
}
