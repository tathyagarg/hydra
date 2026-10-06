use crate::cpu::{
    Cpu,
    shared::{EL, EL1, EL2, EL3},
};

pub type SCTLRType = u64;

impl Cpu {
    pub fn get_SCTLR_EL(&self, regime: EL) -> SCTLRType {
        match regime {
            EL1 => self.sctlr_el1,
            EL2 => self.sctlr_el2,
            EL3 => self.sctlr_el3,
            _ => unreachable!(),
        }
    }

    pub fn set_SCTLR_EL(&mut self, regime: EL, value: SCTLRType) {
        match regime {
            EL1 => self.sctlr_el1 = value,
            EL2 => self.sctlr_el2 = value,
            EL3 => self.sctlr_el3 = value,
            _ => unreachable!(),
        }
    }

    pub fn get_PC64(&self) -> u64 {
        self._pc
    }
}
