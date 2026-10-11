use crate::cpu::{
    Cpu,
    regs::{HCR_EL2Field, RegisterField},
    shared::{EL0, EL1, EL2},
};

impl Cpu {
    /// AArch32_GeneralExceptionsToAArch64()
    /// ====================================
    /// Returns TRUE if exceptions normally routed to EL1 are being handled at an Exception
    /// level using AArch64, because either EL1 is using AArch64 or TGE is in force and EL2
    /// is using AArch64.
    pub fn aarch32_general_execeptions_to_aarch64(&self) -> bool {
        return (self.pstate.EL() == EL0 && !self.el_using_aarch32(EL1))
            || (self.el2_enabled()
                && !self.el_using_aarch32(EL2)
                && HCR_EL2Field::TGE.fetch_bit(self.hcr_el2));
    }

    /// AArch32_Undefined()
    /// ===================
    pub fn aarch32_undefined(&self) {
        if self.aarch32_general_execeptions_to_aarch64() {
            self.aarch64_undefined();
        }

        unimplemented!("AArch32_TakeUndefInstrException");
    }
}
