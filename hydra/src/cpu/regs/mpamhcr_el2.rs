use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum MPAMHCR_EL2Field {
    TRAP_MPAMIDR_EL1 = 31,
    GSTAPP_PLK = 8,
    EL1_VPMEN = 1,
    EL0_VPMEN = 0,
}

impl RegisterField<u64> for MPAMHCR_EL2Field {
    fn fetch_bit(self, mpamhcr_el2_v: u64) -> bool {
        nth_bit(mpamhcr_el2_v, self as usize)
    }
}
