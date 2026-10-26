use crate::cpu::regs::RegisterFieldRange;

#[allow(non_camel_case_types)]
pub enum MPAM2_EL2Field {
    MPAMEN = 63,
    TIDR = 58,
    ALTSP_HFC = 56,
    ALTSP_EL2 = 55,
    ALTSP_FRCD = 54,
    EnMPAMSM = 50,
    TRAPMPAM0EL1 = 49,
    TRAPMPAM1EL1 = 48,
    PMG_D = -1,    // 47-40
    PMG_I = -2,    // 39-32
    PARTID_D = -3, // 31-16
    PARTID_I = -4, // 15-0
}

impl RegisterFieldRange<u64, u8> for MPAM2_EL2Field {
    fn fetch_range(self, mpam2_el2_v: u64) -> u8 {
        if let Self::PMG_D = self {
            return ((mpam2_el2_v >> 40) & 0xFF) as u8;
        }

        if let Self::PMG_I = self {
            return ((mpam2_el2_v >> 32) & 0xFF) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u16> for MPAM2_EL2Field {
    fn fetch_range(self, mpam2_el2_v: u64) -> u16 {
        if let Self::PARTID_D = self {
            return ((mpam2_el2_v >> 16) & 0xFFFF) as u16;
        }

        if let Self::PARTID_I = self {
            return (mpam2_el2_v & 0xFFFF) as u16;
        }

        unreachable!()
    }
}
