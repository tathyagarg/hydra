use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum TCR2_EL1Field {
    FNGNA1 = 21,
    FNGNA0 = 20,
    FNG1 = 18,
    FNG0 = 17,
    A2 = 16,
    DisCH1 = 15,
    DisCH0 = 14,
    HAFT = 11,
    PTTWI = 10,
    D128 = 5,
    AIE = 4,
    POE = 3,
    E0POE = 2,
    PIE = 1,
    PnCH = 0,
}

impl RegisterField<u64> for TCR2_EL1Field {
    fn fetch_bit(self, tcr2_el1_v: u64) -> bool {
        nth_bit(tcr2_el1_v, self as usize)
    }
}
