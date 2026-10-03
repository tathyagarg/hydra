#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BRBSRC_EL1_Type(pub u64);

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BRBTGT_EL1_Type(pub u64);

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BRBINF_EL1_Type {
    pub ccu: bool,
    pub cc: u16,
    pub type_: u8,
    pub el: u8,
    pub mpred: bool,
    pub valid: u8,
}

pub struct Brbe {
    pub records_src: [BRBSRC_EL1_Type; 64],
    pub records_tgt: [BRBTGT_EL1_Type; 64],
    pub records_inf: [BRBINF_EL1_Type; 64],
}
