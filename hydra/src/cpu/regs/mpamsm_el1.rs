use crate::cpu::regs::RegisterFieldRange;

#[allow(non_camel_case_types)]
pub enum MPAMSM_EL1Field {
    PMG_D = -1,    // 47-40
    PARTID_D = -2, // 31-16
}

impl RegisterFieldRange<u64, u8> for MPAMSM_EL1Field {
    fn fetch_range(self, mpamsm_el1_v: u64) -> u8 {
        if let Self::PMG_D = self {
            return ((mpamsm_el1_v >> 40) & 0xFF) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u16> for MPAMSM_EL1Field {
    fn fetch_range(self, mpamasm_el1_v: u64) -> u16 {
        if let Self::PARTID_D = self {
            return ((mpamasm_el1_v >> 16) & 0xFFFF) as u16;
        }

        unreachable!()
    }
}
