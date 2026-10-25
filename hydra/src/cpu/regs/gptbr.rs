use crate::cpu::{Cpu, Feature, common::ones, regs::RegisterFieldRange};

#[allow(non_camel_case_types)]
pub enum GPTBR_EL3Field {
    BADDR = -1, // 39-0
}

impl RegisterFieldRange<u64, u64> for GPTBR_EL3Field {
    fn fetch_range(self, v: u64) -> u64 {
        if Cpu::is_feature_implemented(Feature::RME_GPC3) {
            v & ones(44)
        } else {
            v & ones(40)
        }
    }
}
