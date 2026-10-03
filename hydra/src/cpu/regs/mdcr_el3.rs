use crate::cpu::common::nth_bit;

#[allow(non_camel_case_types)]
pub enum MDCR_EL3Field {
    EnPMS4 = 55,
    TRBEE = -1, // 54-53
    PMSEE = -2, // 52-51
    EnSTEPOP = 50,
    ETBAD = -3, // 49-48
    EnITE = 47,
    EPMSSAD = -4, // 46-45
    EnPMSS = 44,
    EBWE = 43,
    EnPMS3 = 42,
    PMEE = -5, // 41-40
    EnTB2 = 39,
    E3BREC = 38,
    E3BREW = 37,
    EnPMSN = 36,
    MPMX = 35,
    MCCD = 34,
    SBRBE = -6, // 33-32
    PMSSE = -7, // 31-30
    MTPME = 28,
    TDCC = 27,
    NSTBE = 26,
    NSTB = -8, // 25-24
    SCCD = 23,
    ETAD = 22,
    EPMAD = 21,
    EDAD = 20,
    TTRF = 19,
    STE = 18,
    SPME = 17,
    SDD = 16,
    SPD32 = -9, // 15-14
    NSPB = -10, // 13-12
    NSPBE = 11,
    TDOSA = 10,
    TDA = 9,
    EnPM2 = 7,
    TPM = 6,
    EDADE = 4,
    ETADE = 3,
    EPMADE = 2,
    RLTE = 0,
}

impl MDCR_EL3Field {
    pub fn fetch_bit(self, mdcr_el3_v: u64) -> bool {
        nth_bit(mdcr_el3_v, self as usize)
    }

    pub fn fetch_range(self, mdcr_el3_v: u64) -> u8 {
        (match self {
            Self::TRBEE => (mdcr_el3_v >> 53) & 0b11,
            Self::PMSEE => (mdcr_el3_v >> 51) & 0b11,
            Self::ETBAD => (mdcr_el3_v >> 48) & 0b11,
            Self::EPMSSAD => (mdcr_el3_v >> 45) & 0b11,
            Self::PMEE => (mdcr_el3_v >> 40) & 0b11,
            Self::SBRBE => (mdcr_el3_v >> 32) & 0b11,
            Self::PMSSE => (mdcr_el3_v >> 30) & 0b11,
            Self::NSTB => (mdcr_el3_v >> 24) & 0b11,
            Self::SPD32 => (mdcr_el3_v >> 14) & 0b11,
            Self::NSPB => (mdcr_el3_v >> 12) & 0b11,
            _ => unreachable!("??"),
        }) as u8
    }
}
