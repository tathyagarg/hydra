use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum SCTLR2_EL3Field {
    CPTM = 11,
    CPTA = 9,
    EnPACM = 7,
    EnANERR = 4,
    EnADERR = 3,
    EMEC = 1,
}

impl RegisterField<u64> for SCTLR2_EL3Field {
    fn fetch_bit(self, sctlr2_el3_v: u64) -> bool {
        nth_bit(sctlr2_el3_v, self as usize)
    }
}
