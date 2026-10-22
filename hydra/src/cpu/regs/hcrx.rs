use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum HCRX_EL2Field {
    SRMASKEn = 26,
    PACMEn = 24,
    EnFPM = 23,
    GCSEn = 22,
    EnIDCP128 = 21,
    EnSDERR = 20,
    TMEA = 19,
    EnSNERR = 18,
    D128En = 17,
    PTTWI = 16,
    SCTLR2En = 15,
    TCR2En = 14,
    MSCEn = 11,
    MCE2 = 10,
    CMOW = 9,
    VFNMI = 8,
    VINMI = 7,
    TALLINT = 6,
    SMPME = 5,
    FGTnXS = 4,
    FnXS = 3,
    EnASR = 2,
    EnALS = 1,
    EnAS0 = 0,
}

impl RegisterField<u64> for HCRX_EL2Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}
