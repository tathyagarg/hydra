#[allow(non_camel_case_types)]
pub enum SCTLR2_EL1Field {
    CPTM0 = 12,
    CPTM = 11,
    CPTA0 = 10,
    CPTA = 9,
    EnPACM0 = 8,
    EnPACM = 7,
    EnIDCP128 = 6,
    EASE = 5,
    EnANERR = 4,
    EnADERR = 3,
    NMEA = 2,
}

#[allow(non_camel_case_types)]
pub enum SCTLR2_EL2Field {
    CPTM0 = 12,
    CPTM = 11,
    CPTA0 = 10,
    CPTA = 9,
    EnPACM0 = 8,
    EnPACM = 7,
    EnIDCP128 = 6,
    EASE = 5,
    EnANERR = 4,
    EnADERR = 3,
    NMEA = 2,
    EMEC = 1,
}

#[allow(non_camel_case_types)]
pub enum SCTLR2_EL3Field {
    CPTM = 11,
    CPTA = 9,
    EnPACM = 7,
    EnANERR = 4,
    EnADERR = 3,
    EMEC = 1,
}
