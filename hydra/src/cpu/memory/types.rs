use crate::cpu::{
    exeception::{FullAddress, PASpace},
    fault::{AccessDescriptor, ErrorState, Fault, FaultRecord},
    regs::{MAIRType, S1PIRType},
    shared::Faultable,
};

pub type AddressSize = u8;

pub const DEFAULT_MECID: u16 = 0;

pub trait ValidMemReadSize {}
impl ValidMemReadSize for [(); 1] {}
impl ValidMemReadSize for [(); 2] {}
impl ValidMemReadSize for [(); 4] {}
impl ValidMemReadSize for [(); 8] {}
impl ValidMemReadSize for [(); 16] {}
impl ValidMemReadSize for [(); 32] {}

pub struct S1AccessControls {
    pub r: bool,
    pub w: bool,
    pub x: bool,
    pub gcs: bool,
    pub overlay: bool,
    pub or: bool,
    pub ow: bool,
    pub ox: bool,
    pub wxn: bool,
}

pub trait ValidMemReadSizeBits {}
pub struct Size<const N: usize>;
impl ValidMemReadSizeBits for Size<8> {}
impl ValidMemReadSizeBits for Size<16> {}
impl ValidMemReadSizeBits for Size<32> {}
impl ValidMemReadSizeBits for Size<64> {}
impl ValidMemReadSizeBits for Size<128> {}
impl ValidMemReadSizeBits for Size<256> {}
impl ValidMemReadSizeBits for Size<512> {}

pub trait ValidWalkSize {}
impl ValidWalkSize for Size<64> {}
impl ValidWalkSize for Size<128> {}

#[derive(PartialEq)]
pub enum DescriptorType {
    Table,
    Leaf,
    Invalid,
}

pub struct PhysMemRetStatus {
    pub statuscode: Fault,
    pub extflag: bool,
    pub merrorstate: Option<ErrorState>,
    pub store64bstatus: u64,
}

impl PhysMemRetStatus {
    pub fn arbitrary() -> Self {
        Self {
            statuscode: Fault::None,
            extflag: false,
            merrorstate: None,
            store64bstatus: 0,
        }
    }
}

impl Faultable for PhysMemRetStatus {
    fn is_fault(&self) -> bool {
        self.statuscode != Fault::None
    }
}

#[derive(PartialEq, Clone)]
pub enum MemType {
    Normal,
    Device,
}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone)]
pub enum DeviceType {
    GRE,
    nGRE,
    nGnRE,
    nGnRnE,
}

#[allow(non_upper_case_globals)]
pub const MemAttr_NC: u8 = 0b00;
#[allow(non_upper_case_globals)]
pub const MemAttr_WT: u8 = 0b10;
#[allow(non_upper_case_globals)]
pub const MemAttr_WB: u8 = 0b11;

#[allow(non_upper_case_globals)]
pub const MemHint_No: u8 = 0b00;
#[allow(non_upper_case_globals)]
pub const MemHint_WA: u8 = 0b01;
#[allow(non_upper_case_globals)]
pub const MemHint_RA: u8 = 0b10;
#[allow(non_upper_case_globals)]
pub const MemHint_RWA: u8 = 0b11;

#[derive(Clone)]
pub struct MemAttrHints {
    pub attrs: u8,
    pub hints: u8,
    pub transient: bool,
}

#[derive(Clone, Copy)]
pub enum Shareability {
    NSH,
    ISH,
    OSH,
}

#[derive(PartialEq, Clone)]
pub enum MemTagType {
    AllocationTagged,
    CanonicallyTagged,
    Untagged,
}

#[derive(Clone)]
pub struct MemoryAttributes {
    pub memtype: MemType,
    pub device: DeviceType,
    pub inner: MemAttrHints,
    pub outer: MemAttrHints,
    pub shareability: Shareability,
    pub tags: MemTagType,
    pub notagaccess: bool,
    pub xs: bool,
}

#[derive(Clone)]
pub struct AddressDescriptor {
    pub fault: FaultRecord,
    pub memattrs: MemoryAttributes,
    pub paddress: FullAddress,
    pub s1assured: bool,
    pub s2fs1mro: bool,
    pub mecid: u16,
    pub vaddress: u64,
}

impl AddressDescriptor {
    pub fn arbitrary() -> AddressDescriptor {
        AddressDescriptor {
            fault: FaultRecord::no_fault(),
            memattrs: MemoryAttributes {
                memtype: MemType::Normal,
                device: DeviceType::nGnRnE,
                inner: MemAttrHints {
                    attrs: 0,
                    hints: 0,
                    transient: false,
                },
                outer: MemAttrHints {
                    attrs: 0,
                    hints: 0,
                    transient: false,
                },
                shareability: Shareability::NSH,
                tags: MemTagType::Untagged,
                notagaccess: false,
                xs: false,
            },
            paddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            s1assured: false,
            s2fs1mro: false,
            mecid: 0,
            vaddress: 0,
        }
    }

    pub fn create(
        va: u64,
        pa: FullAddress,
        memattrs: MemoryAttributes,
        accdesc: &AccessDescriptor,
    ) -> Self {
        Self {
            paddress: pa,
            vaddress: va,
            memattrs,
            fault: FaultRecord::no_fault_accdesc(accdesc),
            s1assured: false,
            s2fs1mro: false,
            mecid: 0,
        }
    }

    pub fn create_faulty(fault: &FaultRecord) -> Self {
        Self {
            vaddress: fault.vaddress,
            fault: fault.clone(),
            ..Self::arbitrary()
        }
    }
}

impl Faultable for AddressDescriptor {
    fn is_fault(&self) -> bool {
        self.fault.statuscode != Fault::None
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum TGx {
    _4KB,
    _16KB,
    _64KB,
}

pub struct S1TTWParams {
    pub ha: bool,         // TCR_ELx.HA
    pub hd: bool,         // TCR_ELx.HD
    pub tbi: bool,        // TCR_ELx.TBI{x}
    pub tbid: bool,       // TCR_ELx.TBID{x}
    pub nfd: bool,        // TCR_EL1.NFDx or TCR_EL2.NFDx when HCR_EL2.E2H == '1'
    pub e0pd: bool,       // TCR_EL1.E0PDx or TCR_EL2.E0PDx when HCR_EL2.E2H == '1'
    pub d128: bool,       // TCR_ELx.D128
    pub aie: bool,        // (TCR2_ELx/TCR_EL3).AIE
    pub mair2: MAIRType,  // MAIR2_ELx
    pub ds: bool,         // TCR_ELx.DS
    pub ps: u8,           // TCR_ELx.{I}PS
    pub txsz: u8,         // TCR_ELx.TxSZ
    pub epan: bool,       // SCTLR_EL1.EPAN or SCTLR_EL2.EPAN when HCR_EL2.E2H == '1'
    pub dct: bool,        // HCR_EL2.DCT
    pub nv1: bool,        // HCR_EL2.NV1
    pub cmow: bool,       // SCTLR_EL1.CMOW or SCTLR_EL2.CMOW when HCR_EL2.E2H == '1'
    pub pnch: bool,       // TCR{2}_ELx.PnCH
    pub disch: bool,      // TCR{2}_ELx.DisCH
    pub haft: bool,       // TCR{2}_ELx.HAFT
    pub mtx: bool,        // TCR_ELx.MTX{y}
    pub skl: u8,          // TTBRn_ELx.SKL
    pub pie: bool,        // TCR2_ELx.PIE or TCR_EL3.PIE
    pub pir: S1PIRType,   // PIR_ELx
    pub pire0: S1PIRType, // PIRE0_EL1 or PIRE0_EL2 when HCR_EL2.E2H == '1'
    pub emec: bool,       // SCTLR2_EL2.EMEC or SCTLR2_EL3.EMEC
    pub amec: bool,       // TCR2_EL2.AMEC0 or TCR2_EL2.AMEC1 when HCR_EL2.E2H == '1'
    pub fng: bool,        // TCR2_EL1.FNGx or TCR2_EL2.FNGx when HCR_EL2.E2H == '1'
    pub fngna: bool,      // TCR2_EL1.FNGx
    // A32-VMSA exclusive parameters
    pub t0sz: u8,   // TTBCR.T0SZ
    pub t1sz: u8,   // TTBCR.T1SZ
    pub uwxn: bool, // SCTLR.UWXN
    // Parameters common to both A64-VMSA & A32-VMSA (A64/A32)
    pub tgx: TGx,       // TCR_ELx.TGx / Always TGx_4KB
    pub irgn: u8,       // TCR_ELx.IRGNx / TTBCR.IRGNx or HTCR.IRGN0
    pub orgn: u8,       // TCR_ELx.ORGNx / TTBCR.ORGNx or HTCR.ORGN0
    pub sh: u8,         // TCR_ELx.SHx / TTBCR.SHx or HTCR.SH0
    pub hpd: bool,      // TCR_ELx.HPD{x} / TTBCR2.HPDx or HTCR.HPD
    pub ee: bool,       // SCTLR_ELx.EE / SCTLR.EE or HSCTLR.EE
    pub wxn: bool,      // SCTLR_ELx.WXN / SCTLR.WXN or HSCTLR.WXN
    pub ntlsmd: bool,   // SCTLR_ELx.nTLSMD / SCTLR.nTLSMD or HSCTLR.nTLSMD
    pub dc: bool,       // HCR_EL2.DC / HCR.DC
    pub sif: bool,      // SCR_EL3.SIF / SCR.SIF
    pub mair: MAIRType, // MAIR_ELx / MAIR1:MAIR0 or HMAIR1:HMAIR0
}

pub enum SDFType {
    Table,
    Invalid,
    Supersection,
    Section,
    LargePage,
    SmallPage,
}

#[derive(Default, Clone)]
pub struct Permissions {
    pub(crate) ap_table: u8,    // Stage 1 hierarchical access permissions
    pub(crate) xn_table: bool,  // Stage 1 hierarchical execute-never for single EL regimes
    pub(crate) pxn_table: bool, // Stage 1 hierarchical privileged execute-never
    pub(crate) uxn_table: bool, // Stage 1 hierarchical unprivileged execute-never
    pub(crate) ap: u8,          // Stage 1 access permissions
    pub(crate) xn: bool,        // Stage 1 execute-never for single EL regimes
    pub(crate) uxn: bool,       // Stage 1 unprivileged execute-never
    pub(crate) pxn: bool,       // Stage 1 privileged execute-never
    pub(crate) ppi: u8,         // Stage 1 privileged indirect permissions
    pub(crate) upi: u8,         // Stage 1 unprivileged indirect permissions
    pub(crate) ndirty: bool,    // Stage 1 dirty state for indirect permissions scheme
    pub(crate) s2pi: u8,        // Stage 2 indirect permissions
    pub(crate) s2dirty: bool,   // Stage 2 dirty state
    pub(crate) po_index: u8,    // Stage 1 overlay permissions index
    pub(crate) s2po_index: u8,  // Stage 2 overlay permissions index
    pub(crate) s2ap: u8,        // Stage 2 access permissions
    pub(crate) s2tag_na: bool,  // Stage 2 tag access
    pub(crate) s2xnx: bool,     // Stage 2 extended execute-never
    pub(crate) dbm: bool,       // Dirty bit management
    pub(crate) s2xn: bool,      // Stage 2 execute-never
}

pub struct TTWState {
    pub istable: bool,
    pub level: u8,
    pub baseaddress: FullAddress,
    pub contiguous: bool,
    pub s1assured: bool,
    pub s2assuredonly: bool,
    pub disch: bool,
    pub nG: bool,
    pub guardedpage: bool,
    pub sdftype: SDFType,
    pub domain: u8,
    pub memattrs: MemoryAttributes,
    pub permissions: Permissions,
}

impl TTWState {
    pub fn arbitrary() -> Self {
        TTWState {
            istable: false,
            level: 0,
            baseaddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            contiguous: false,
            s1assured: false,
            s2assuredonly: false,
            disch: false,
            nG: false,
            guardedpage: false,
            sdftype: SDFType::Invalid,
            domain: 0,
            memattrs: MemoryAttributes {
                memtype: MemType::Normal,
                device: DeviceType::nGnRnE,
                inner: MemAttrHints {
                    attrs: 0,
                    hints: 0,
                    transient: false,
                },
                outer: MemAttrHints {
                    attrs: 0,
                    hints: 0,
                    transient: false,
                },
                shareability: Shareability::NSH,
                tags: MemTagType::Untagged,
                notagaccess: false,
                xs: false,
            },
            permissions: Permissions {
                ap_table: 0,
                xn_table: false,
                pxn_table: false,
                uxn_table: false,
                ap: 0,
                xn: false,
                uxn: false,
                pxn: false,
                ppi: 0,
                upi: 0,
                ndirty: false,
                s2pi: 0,
                s2dirty: false,
                po_index: 0,
                s2po_index: 0,
                s2ap: 0,
                s2tag_na: false,
                s2xnx: false,
                dbm: false,
                s2xn: false,
            },
        }
    }
}
