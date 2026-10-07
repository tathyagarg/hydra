use crate::cpu::{common::nth_bit, regs::RegisterField};

#[allow(non_camel_case_types)]
pub enum MPAMVPMV_EL2Field {
    VPM_V31,
    VPM_V30,
    VPM_V29,
    VPM_V28,
    VPM_V27,
    VPM_V26,
    VPM_V25,
    VPM_V24,
    VPM_V23,
    VPM_V22,
    VPM_V21,
    VPM_V20,
    VPM_V19,
    VPM_V18,
    VPM_V17,
    VPM_V16,
    VPM_V15,
    VPM_V14,
    VPM_V13,
    VPM_V12,
    VPM_V11,
    VPM_V10,
    VPM_V9,
    VPM_V8,
    VPM_V7,
    VPM_V6,
    VPM_V5,
    VPM_V4,
    VPM_V3,
    VPM_V2,
    VPM_V1,
    VPM_V0,
}

impl RegisterField<u64> for MPAMVPMV_EL2Field {
    fn fetch_bit(self, mpamvpmv_el2_v: u64) -> bool {
        nth_bit(mpamvpmv_el2_v, self as usize)
    }
}
