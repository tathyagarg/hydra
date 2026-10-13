use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum TCR2_EL2Field {
    AMEC0 = 12,
    HAFT = 11,
    PTTWI = 10,
    AIE = 4,
    POE = 3,
    PIE = 1,
    PnCH = 0,
}

impl RegisterField<u64> for TCR2_EL2Field {
    fn fetch_bit(self, tcr2_el2_v: u64) -> bool {
        nth_bit(tcr2_el2_v, self as usize)
    }
}
