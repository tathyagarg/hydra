use crate::cpu::regs::RegisterFieldRange;

#[allow(non_camel_case_types)]
pub enum MPAM3_EL3Field {
    MPAMEN = 63,
    TRAPLOWER = 62,
    SDEFLT = 61,
    FORCE_NS = 60,
    ALTSP_HEN = 59,
    ALTSP_HFC = 58,
    ALTSP_EL3 = 57,
    RT_ALTSP_NS = 56,
    PMG_D = -1,    // 47-40
    PMG_I = -2,    // 39-32
    PARTID_D = -3, // 31-16
    PARTID_I = -4, // 15-0
}

impl RegisterFieldRange<u64, u8> for MPAM3_EL3Field {
    fn fetch_range(self, mpam3_el3_v: u64) -> u8 {
        if let Self::PMG_D = self {
            return ((mpam3_el3_v >> 40) & 0xFF) as u8;
        }

        if let Self::PMG_I = self {
            return ((mpam3_el3_v >> 32) & 0xFF) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u16> for MPAM3_EL3Field {
    fn fetch_range(self, mpam3_el3_v: u64) -> u16 {
        if let Self::PARTID_D = self {
            return ((mpam3_el3_v >> 16) & 0xFFFF) as u16;
        }

        if let Self::PARTID_I = self {
            return (mpam3_el3_v & 0xFFFF) as u16;
        }

        unreachable!()
    }
}
