use crate::cpu::{
    Cpu, Feature,
    common::*,
    exeception::{FullAddress, PASpace},
    fault::{AccessDescriptor, AccessType, ErrorState, Fault, FaultRecord, VARange},
    regs::{
        E2H0_TCR_EL2Field, E2H1_TCR_EL2Field, HCR_EL2Field, MAIRType, RegisterField,
        RegisterFieldRange, S1PIRType, SCR_EL3Field, SCTLR2_EL2Field, SCTLR2_EL3Field, SCTLRField,
        TCR_EL1Field, TCR_EL2CommonField, TCR_EL3Field, TCR2_EL2Field, VTCR_EL2Field,
        mairtype_from_mair,
    },
    shared::{EL, EL0, EL3, Regime, SecurityState},
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

pub enum MemType {
    Normal,
    Device,
}

#[allow(non_camel_case_types)]
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

pub struct MemAttrHints {
    pub attrs: u8,
    pub hints: u8,
    pub transient: bool,
}

pub enum Shareability {
    NSH,
    ISH,
    OSH,
}

pub enum MemTagType {
    AllocationTagged,
    CanonicallyTagged,
    Untagged,
}

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

#[derive(Default)]
pub struct Permissions {
    ap_table: u8,    // Stage 1 hierarchical access permissions
    xn_table: bool,  // Stage 1 hierarchical execute-never for single EL regimes
    pxn_table: bool, // Stage 1 hierarchical privileged execute-never
    uxn_table: bool, // Stage 1 hierarchical unprivileged execute-never
    ap: u8,          // Stage 1 access permissions
    xn: bool,        // Stage 1 execute-never for single EL regimes
    uxn: bool,       // Stage 1 unprivileged execute-never
    pxn: bool,       // Stage 1 privileged execute-never
    ppi: u8,         // Stage 1 privileged indirect permissions
    upi: u8,         // Stage 1 unprivileged indirect permissions
    ndirty: bool,    // Stage 1 dirty state for indirect permissions scheme
    s2pi: u8,        // Stage 2 indirect permissions
    s2dirty: bool,   // Stage 2 dirty state
    po_index: u8,    // Stage 1 overlay permissions index
    s2po_index: u8,  // Stage 2 overlay permissions index
    s2ap: u8,        // Stage 2 access permissions
    s2tag_na: bool,  // Stage 2 tag access
    s2xnx: bool,     // Stage 2 extended execute-never
    dbm: bool,       // Dirty bit management
    s2xn: bool,      // Stage 2 execute-never
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

impl Cpu {
    /// S is the number of bytes to read, not number of bits.
    pub fn get_mem<const S: usize>(&self, address: u64, accdesc_in: &AccessDescriptor) -> [u8; S]
    where
        [(); S]: ValidMemReadSize,
    {
        let aligned = if accdesc_in.ispair && !accdesc_in.exclusive {
            todo!("Paired access descriptor");
        } else {
            is_aligned_size(address, S as u64)
        };

        if !aligned {
            todo!("Unaligned access");
        }

        let value = if accdesc_in.acctype == AccessType::ASIMD
            && S == 16
            && is_aligned_size(address, 8)
        {
            todo!("SIMD access");
        } else if Cpu::is_feature_implemented(Feature::LS64WB)
            && accdesc_in.acctype == AccessType::ASIMD
            && S == 32
            && accdesc_in.ispair
            && is_aligned_size(address, 32)
        {
            todo!("SIMD again?");
        } else if accdesc_in.acctype == AccessType::ASIMD && S == 32 && is_aligned_size(address, 8)
        {
            todo!("SIMD again?");
        } else if Cpu::is_feature_implemented(Feature::LSE2) {
            todo!("What even");
        } else if (aligned && accdesc_in.ispair)
            || (accdesc_in.acctype == AccessType::SVE
                && accdesc_in.predicated
                && S == 16
                && is_aligned_size(address, 8))
        {
            todo!("What even");
        } else if aligned {
            self.get_aarch64_mem_single(address, accdesc_in, aligned)
        } else {
            todo!("Unaligned");
        };

        todo!()
    }

    pub fn get_aarch64_mem_single<const S: usize>(
        &self,
        address: u64,
        accdesc_in: &AccessDescriptor,
        aligned: bool,
    ) -> [u8; S]
    where
        [(); S]: ValidMemReadSize,
    {
        let (value, memaddrdesc, memstatus) =
            self.aarch64_mem_single_read(address, accdesc_in, aligned);

        if is_fault_addrdesc(&memaddrdesc) {
            todo!("Abort");
        }

        if is_fault_retstatus(&memstatus) {
            todo!("Abort")
        }

        value
    }

    pub fn aarch64_mem_single_read<const S: usize>(
        &self,
        address: u64,
        accdesc_in: &AccessDescriptor,
        aligned: bool,
    ) -> ([u8; S], AddressDescriptor, PhysMemRetStatus)
    where
        [(); S]: ValidMemReadSize,
    {
        let mut value = [0u8; S];

        let mut memstatus = PhysMemRetStatus::arbitrary();
        memstatus.statuscode = Fault::None;

        let mut accdesc = accdesc_in.clone();

        if Cpu::is_feature_implemented(Feature::LSE2) {
            todo!("LSE2");
        } else {
            assert!(is_aligned_size(address, S as u64));
        }

        if accdesc.tagchecked {
            accdesc.tagchecked = self.aarch64_access_is_tag_checked(address, &accdesc);
        }

        // let memaddrdesc =

        todo!()
    }

    pub fn aarch64_full_translate(
        &self,
        va: u64,
        size: usize,
        accdesc: &AccessDescriptor,
        aligned: bool,
    ) -> AddressDescriptor {
        let regime = self.translation_regime(accdesc.el);
        let mut fault = FaultRecord::no_fault_accdecs_va(accdesc, va);

        todo!()
    }

    pub fn aarch64_translate_address(
        &self,
        va: u64,
        accdesc: &AccessDescriptor,
        aligned: bool,
        size: usize,
    ) -> AddressDescriptor {
        todo!()
        // let mut result = self.
    }

    #[allow(unused_variables)]
    pub fn aarch64_access_is_tag_checked(&self, vaddr: u64, accdesc: &AccessDescriptor) -> bool {
        if self.using_aarch32() {
            return false;
        }

        // TODO: More checks and stuff

        true
    }

    pub fn aarch64_s1enabled(&self, regime: Regime, acctype: AccessType) -> bool {
        // TODO: Effective TRBLIMITR yada yada
        if acctype == AccessType::TRBE {
            return false;
        }

        // TODO: Effective PMBLIMITR yada yada
        if acctype == AccessType::SPE {
            return false;
        }

        match regime {
            Regime::EL3 => SCTLRField::M.fetch_bit(self.sctlr_el3),
            Regime::EL2 | Regime::EL20 => SCTLRField::M.fetch_bit(self.sctlr_el2),
            Regime::EL10 => {
                (!self.el2_enabled()
                    || (!HCR_EL2Field::DC.fetch_bit(self.hcr_el2)
                        && !HCR_EL2Field::TGE.fetch_bit(self.hcr_el2)))
                    && SCTLRField::M.fetch_bit(self.sctlr_el1)
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_get_varange(&self, va: u64) -> VARange {
        if nth_bit(va, 55) {
            VARange::LOWER
        } else {
            VARange::UPPER
        }
    }

    pub fn aarch64_s1decode_tg0(&self, tg0_in: u8) -> TGx {
        if tg0_in == 0b11 {
            panic!("Imp def bits");
        }

        match tg0_in {
            0b00 => TGx::_4KB,
            0b01 => TGx::_64KB,
            0b10 => TGx::_16KB,
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1ttw_params_el3(&self) -> S1TTWParams {
        let tgx = self.aarch64_s1decode_tg0(TCR_EL3Field::TG0.fetch_range(self.tcr_el3));
        let txsz = TCR_EL3Field::T0SZ.fetch_range(self.tcr_el3);
        let ps = TCR_EL3Field::PS.fetch_range(self.tcr_el3);
        let irgn = TCR_EL3Field::IRGN0.fetch_range(self.tcr_el3);
        let orgn = TCR_EL3Field::ORGN0.fetch_range(self.tcr_el3);
        let sh = TCR_EL3Field::SH0.fetch_range(self.tcr_el3);
        let tbi = TCR_EL3Field::TBI.fetch_bit(self.tcr_el3);
        let mair = mairtype_from_mair(self.mair_el3);
        let skl = 0;
        let d128 = if Cpu::is_feature_implemented(Feature::D128) {
            TCR_EL3Field::D128.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let disch = if d128 {
            TCR_EL3Field::DisCH0.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pie = if d128 {
            true
        } else if Cpu::is_feature_implemented(Feature::S1PIE) {
            TCR_EL3Field::PIE.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pir = [0; 16];
        let mair2 = [0; 8];
        let aie = if Cpu::is_feature_implemented(Feature::AIE) {
            TCR_EL3Field::AIE.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let wxn = SCTLRField::WXN.fetch_bit(self.sctlr_el3);
        let ee = SCTLRField::EE.fetch_bit(self.sctlr_el3);
        let sif = if !Cpu::is_feature_implemented(Feature::RME)
            || Cpu::is_feature_implemented(Feature::SEL2)
        {
            SCR_EL3Field::SIF.fetch_bit(self.scr_el3)
        } else {
            false
        };
        let tbid = if Cpu::is_feature_implemented(Feature::PAuth) {
            TCR_EL3Field::TBID.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let mut hpd = if Cpu::is_feature_implemented(Feature::HPDS) {
            TCR_EL3Field::HPD.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        if !hpd {
            if aie {
                hpd = true;
            }
            if pie {
                hpd = true;
            }
        }
        let ha = if Cpu::is_feature_implemented(Feature::HAF) {
            TCR_EL3Field::HA.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let hd = if Cpu::is_feature_implemented(Feature::HAFDBS) && ha {
            TCR_EL3Field::HD.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let ds = if [TGx::_4KB, TGx::_16KB].contains(&tgx)
            && Cpu::is_feature_implemented(Feature::LPA2)
            && !d128
        {
            TCR_EL3Field::DS.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let mtx = if Cpu::is_feature_implemented(Feature::MTE_CANONICAL_TAGS)
            || Cpu::is_feature_implemented(Feature::MTE_NO_ADDRESS_TAGS)
        {
            TCR_EL3Field::MTX.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pnch = if Cpu::is_feature_implemented(Feature::THE) && !d128 {
            TCR_EL3Field::PnCH.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let haft = if Cpu::is_feature_implemented(Feature::HAFT) && ha {
            TCR_EL3Field::HAFT.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let emec = if Cpu::is_feature_implemented(Feature::MEC) {
            SCTLR2_EL3Field::EMEC.fetch_bit(self.sctlr2_el3)
        } else {
            false
        };

        S1TTWParams {
            tgx,
            txsz,
            ps,
            irgn,
            orgn,
            sh,
            tbi,
            mair,
            d128,
            skl,
            disch,
            pie,
            pir,
            mair2,
            aie,
            wxn,
            ee,
            sif,
            tbid,
            hpd,
            ha,
            hd,
            ds,
            mtx,
            pnch,
            haft,
            emec,
            nfd: false,
            amec: false,
            cmow: false,
            dc: false,
            dct: false,
            e0pd: false,
            epan: false,
            nv1: false,
            fng: false,
            fngna: false,
            ntlsmd: false,
            pire0: [0; 16],
            t0sz: 0,
            t1sz: 0,
            uwxn: false,
        }
    }

    pub fn aarch64_s1ttw_params_el2(&self, ss: SecurityState) -> S1TTWParams {
        let tgx = self.aarch64_s1decode_tg0(TCR_EL2CommonField::TG0.fetch_range(self.tcr_el2));
        let txsz = TCR_EL2CommonField::T0SZ.fetch_range(self.tcr_el2);
        let ps = E2H0_TCR_EL2Field::PS.fetch_range(self.tcr_el2);
        let irgn = TCR_EL2CommonField::IRGN0.fetch_range(self.tcr_el2);
        let orgn = TCR_EL2CommonField::ORGN0.fetch_range(self.tcr_el2);
        let sh = TCR_EL2CommonField::SH0.fetch_range(self.tcr_el2);
        let tbi = E2H0_TCR_EL2Field::TBI.fetch_bit(self.tcr_el2);
        let mair = mairtype_from_mair(self.mair_el2);
        let pie = if Cpu::is_feature_implemented(Feature::S1PIE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::PIE.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let mut pir = [0u8; 16] as S1PIRType;
        if Cpu::is_feature_implemented(Feature::S1PIE) {
            if !(self.have_el(EL3) && !SCR_EL3Field::PIEn.fetch_bit(self.scr_el3)) {
                pir = self.pir_el2 as S1PIRType;
            } else {
                pir = [0u8; 16] as S1PIRType;
            }
        }

        // TODO: MAIR2_EL2
        let mair2 = [0; 8];

        let aie = if Cpu::is_feature_implemented(Feature::AIE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::AIE.fetch_bit(self.tcr2_el2)
        } else {
            false
        };

        let wxn = SCTLRField::WXN.fetch_bit(self.sctlr_el2);
        let ee = SCTLRField::EE.fetch_bit(self.sctlr_el2);
        let sif = if self.have_el(EL3)
            && (!Cpu::is_feature_implemented(Feature::RME)
                || Cpu::is_feature_implemented(Feature::SEL2))
        {
            SCR_EL3Field::SIF.fetch_bit(self.scr_el3)
        } else {
            false
        };

        let tbid = if Cpu::is_feature_implemented(Feature::PAuth) {
            E2H0_TCR_EL2Field::TBID.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let mut hpd = if Cpu::is_feature_implemented(Feature::HPDS) {
            E2H0_TCR_EL2Field::HPD.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        if !hpd {
            if aie {
                hpd = true;
            }

            if pie {
                hpd = true;
            }
        }

        let ha = if Cpu::is_feature_implemented(Feature::HAF) {
            E2H0_TCR_EL2Field::HA.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let hd = if Cpu::is_feature_implemented(Feature::HAFDBS) && ha {
            E2H0_TCR_EL2Field::HD.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let ds = if [TGx::_4KB, TGx::_16KB].contains(&tgx)
            && Cpu::is_feature_implemented(Feature::LPA2)
        {
            E2H0_TCR_EL2Field::DS.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let mtx = if Cpu::is_feature_implemented(Feature::MTE_NO_ADDRESS_TAGS)
            || Cpu::is_feature_implemented(Feature::MTE_CANONICAL_TAGS)
        {
            E2H0_TCR_EL2Field::MTX.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let pnch = if Cpu::is_feature_implemented(Feature::THE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::PnCH.fetch_bit(self.tcr2_el2)
        } else {
            false
        };
        let haft = if Cpu::is_feature_implemented(Feature::HAFT) && ha && self.is_tcr2el2_enabled()
        {
            TCR2_EL2Field::HAFT.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let emec = if Cpu::is_feature_implemented(Feature::MEC) && self.is_sctlr2el2_enabled() {
            SCTLR2_EL2Field::EMEC.fetch_bit(self.sctlr2_el2)
        } else {
            false
        };

        let amec = if Cpu::is_feature_implemented(Feature::MEC)
            && ss == SecurityState::Realm
            && self.is_tcr2el2_enabled()
        {
            TCR2_EL2Field::AMEC0.fetch_bit(self.tcr2_el2)
        } else {
            false
        };

        S1TTWParams {
            tgx,
            txsz,
            ps,
            irgn,
            orgn,
            sh,
            tbi,
            mair,
            d128: false,
            skl: 0,
            disch: false,
            pie,
            pir,
            mair2,
            aie,
            wxn,
            ee,
            sif,
            tbid,
            hpd,
            ha,
            hd,
            ds,
            mtx,
            pnch,
            haft,
            emec,
            nfd: false,
            amec,
            cmow: false,
            dc: false,
            dct: false,
            e0pd: false,
            epan: false,
            nv1: false,
            fng: false,
            fngna: false,
            ntlsmd: false,
            pire0: [0; 16],
            t0sz: 0,
            t1sz: 0,
            uwxn: false,
        }
    }

    pub fn aarch64_get_s1ttw_params(
        &self,
        regime: Regime,
        el: EL,
        ss: SecurityState,
        va: u64,
    ) -> S1TTWParams {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL3 => self.aarch64_s1ttw_params_el3(),
            Regime::EL2 => self.aarch64_s1ttw_params_el2(ss),
            _ => todo!("S1TTW Params for {:?}", regime),
        }
    }

    #[allow(unused_variables)]
    pub fn aarch64_s1min_txsz(&self, regime: Regime, walkparams: &S1TTWParams) -> u8 {
        // TODO: LVA3 and LVA features
        return 16;
    }

    #[allow(unused_variables)]
    pub fn aarch64_max_txsz(&self, tgx: TGx) -> u8 {
        // TODO: TTST featuer
        return 39;
    }

    #[allow(unused_variables)]
    pub fn aarch64_s1txsz_faults(&self, regime: Regime, walkparams: &S1TTWParams) -> bool {
        // TODO: boring stuff
        return false;
    }

    pub fn aarch64_addrtop(&self, tbid: bool, acctype: AccessType, tbi: bool) -> AddressSize {
        if tbid && acctype == AccessType::IFETCH {
            return 63;
        }

        if tbi {
            return 55;
        } else {
            return 63;
        }
    }

    pub fn aarch64_iasize(&self, txsz: u8) -> AddressSize {
        (64 - txsz) as AddressSize
    }

    pub fn aarch64_va_is_out_of_range(
        &self,
        va_in: u64,
        acctype: AccessType,
        regime: Regime,
        walkparams: &S1TTWParams,
    ) -> bool {
        let mut va = va_in;

        let addrtop = self.aarch64_addrtop(walkparams.tbid, acctype, walkparams.tbi);

        if walkparams.mtx && acctype != AccessType::IFETCH {
            if self.aarch64_get_varange(va) == VARange::UPPER {
                va |= 0b1111 << 56;
            } else {
                va &= !(0b1111 << 56);
            }
        }

        let iasize = self.aarch64_iasize(walkparams.txsz);

        if addrtop < iasize {
            return false;
        }

        let width = addrtop - iasize + 1;
        let bits = (va >> iasize) & ((1u64 << width) - 1);

        if self.has_unprivileged(regime) {
            if self.aarch64_get_varange(va) == VARange::LOWER {
                bits != 0
            } else {
                bits != ((1u64 << width) - 1)
            }
        } else {
            bits != 0
        }
    }

    pub fn aarch64_s1epd(&self, regime: Regime, va: u64) -> bool {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL20 => {
                if varange == VARange::LOWER {
                    E2H1_TCR_EL2Field::EPD0.fetch_bit(self.tcr_el2)
                } else {
                    E2H1_TCR_EL2Field::EPD1.fetch_bit(self.tcr_el2)
                }
            }
            Regime::EL10 => {
                if varange == VARange::LOWER {
                    TCR_EL1Field::EPD0.fetch_bit(self.tcr_el1)
                } else {
                    TCR_EL1Field::EPD1.fetch_bit(self.tcr_el1)
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1ttbr(&self, regime: Regime, va: u64) -> u128 {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL3 => self.ttbr0_el3 as u128,
            Regime::EL2 => self.ttbr0_el2 as u128,
            Regime::EL20 => {
                if varange == VARange::LOWER {
                    self.ttbr0_el2 as u128
                } else {
                    self.ttbr1_el2 as u128
                }
            }
            Regime::EL10 => {
                if varange == VARange::LOWER {
                    self.ttbr0_el1 as u128
                } else {
                    self.ttbr1_el1 as u128
                }
            }
            _ => todo!(),
        }
    }

    pub fn aarch64_s1start_level(&self, walkparams: &S1TTWParams) -> u8 {
        let iasize = self.aarch64_iasize(walkparams.txsz);
        let granulebits = self.tgx_granulebits(walkparams.tgx);
        let descsizelog2 = 3 + (walkparams.d128 as u8);
        let stride = granulebits - descsizelog2;
        let mut s1startlevel = FINAL_LEVEL - (((iasize - 1) - granulebits) / stride);
        if walkparams.d128 {
            s1startlevel += walkparams.skl;
        }

        s1startlevel
    }

    pub fn aarch64_s1ttb_base_address<T: Copy + Into<u128>>(
        &self,
        walkparams: &S1TTWParams,
        regime: Regime,
        ttbr: T,
    ) -> u64 {
        let tablebase: u64;

        let iasize = self.aarch64_iasize(walkparams.txsz);
        let granulebits = self.tgx_granulebits(walkparams.tgx);
        let descsizelog2 = 3 + (walkparams.d128 as u8);
        let stride = granulebits - descsizelog2;
        let startlevel = self.aarch64_s1start_level(walkparams);
        let levels = FINAL_LEVEL - startlevel;

        let mut tsize = (iasize - (levels * stride + granulebits)) + descsizelog2;

        if walkparams.d128 {
            tsize = tsize.max(5);
            tablebase = if regime == Regime::EL3 {
                ((ttbr.into() >> 5) as u64) & ones(51)
            } else {
                let hi = ((ttbr.into() >> 80) & 0xFF) as u64;
                let lo = ((ttbr.into() >> 5) as u64) & ones(43);
                (hi << 43) | lo
            };
        } else if walkparams.ds
            || (walkparams.tgx == TGx::_64KB
                && walkparams.ps == 0b110
                && Cpu::is_feature_implemented(Feature::LPA))
        {
            tsize = tsize.max(6);
            tablebase = ((((ttbr.into() >> 2) & 0xF) << 42) as u64
                | ((ttbr.into() >> 6) as u64 & ones(42))) as u64;
        } else {
            tablebase = ((ttbr.into() >> 1) as u64 & ones(47)) << 1;
        }

        align_down_p2::<56>(tablebase, tsize as u32)
    }

    pub fn aarch64_s1initial_ttwstate(
        &self,
        walkparams: &S1TTWParams,
        va: u64,
        regime: Regime,
        ss: SecurityState,
    ) -> TTWState {
        let ttbr = self.aarch64_s1ttbr(regime, va);
        let paspace = match ss {
            SecurityState::Secure => PASpace::Secure,
            SecurityState::NonSecure => PASpace::NonSecure,
            SecurityState::Root => PASpace::Root,
            SecurityState::Realm => PASpace::Realm,
        };

        let address = self.aarch64_s1ttb_base_address(walkparams, regime, ttbr);
        let tablebase = FullAddress { paspace, address };
        let perms = Permissions {
            ap_table: 0b00,
            uxn_table: if self.has_unprivileged(regime) {
                false
            } else {
                Default::default()
            },
            pxn_table: if self.has_unprivileged(regime) {
                false
            } else {
                Default::default()
            },
            xn_table: if !self.has_unprivileged(regime) {
                false
            } else {
                Default::default()
            },
            ..Default::default()
        };

        let level = self.aarch64_s1start_level(walkparams);
        let istable = true;
        let nG = self.has_unprivileged(regime);
        let memattrs = self.walk_mem_attrs(walkparams.sh, walkparams.irgn, walkparams.orgn);
        let s1assured = if regime == Regime::EL10
            && self.el2_enabled()
            && HCR_EL2Field::VM.fetch_bit(self.hcr_el2)
        {
            if (self.aarch64_get_varange(va) == VARange::LOWER
                && VTCR_EL2Field::TL0.fetch_bit(self.vtcr_el2))
                || (self.aarch64_get_varange(va) == VARange::UPPER
                    && VTCR_EL2Field::TL1.fetch_bit(self.vtcr_el2))
            {
                true
            } else {
                false
            }
        } else {
            false
        };

        let disch = walkparams.disch;

        TTWState {
            baseaddress: tablebase,
            level,
            istable,
            nG,
            memattrs,
            permissions: perms,
            s1assured,
            disch,
            contiguous: false,
            s2assuredonly: false,
            guardedpage: false,
            sdftype: SDFType::Table,
            domain: 0,
        }
    }

    pub fn aarch64_s1ttwalk_mecid(&self, emec: bool, regime: Regime, ss: SecurityState) -> u16 {
        if !emec {
            return DEFAULT_MECID;
        }

        if ss != SecurityState::Realm {
            return DEFAULT_MECID;
        }

        if !self.effective_scr_el3_mecen() {
            return DEFAULT_MECID;
        }

        todo!()
    }

    pub fn aarch64_s1walk<const N: usize>(
        &self,
        fault_in: FaultRecord,
        walkparams: &S1TTWParams,
        va: u64,
        regime: Regime,
        accdesc: &AccessDescriptor,
    ) -> (FaultRecord, AddressDescriptor, TTWState, u128) {
        let mut fault = fault_in;
        let mut aligned = false;

        if self.has_unprivileged(regime) && self.aarch64_s1epd(regime, va) {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (
                fault,
                AddressDescriptor::arbitrary(),
                TTWState::arbitrary(),
                0,
            );
        }

        let mut walkstate = self.aarch64_s1initial_ttwstate(walkparams, va, regime, accdesc.ss);
        let startlevel = walkstate.level;

        if startlevel > 3 {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (
                fault,
                AddressDescriptor::arbitrary(),
                TTWState::arbitrary(),
                0,
            );
        }

        todo!()
    }

    pub fn aarch64_s1translate(
        &self,
        fault_in: FaultRecord,
        regime: Regime,
        va: u64,
        size: usize,
        aligned: bool,
        accdesc: &AccessDescriptor,
    ) -> (FaultRecord, AddressDescriptor) {
        let mut fault = fault_in;
        fault.secondstage = false;
        fault.s2fs1walk = false;

        if !self.aarch64_s1enabled(regime, accdesc.acctype) {
            todo!("Disabled output");
        }

        let mut walkparams = self.aarch64_get_s1ttw_params(regime, accdesc.el, accdesc.ss, va);
        let s1mintxsz = self.aarch64_s1min_txsz(regime, &walkparams);
        let s1maxtxsz = self.aarch64_max_txsz(walkparams.tgx);

        if self.aarch64_s1txsz_faults(regime, &walkparams) {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (fault, AddressDescriptor::arbitrary());
        } else if walkparams.txsz < s1mintxsz {
            walkparams.txsz = s1mintxsz & 0b11111;
        } else if walkparams.txsz > s1maxtxsz {
            walkparams.txsz = s1maxtxsz & 0b11111;
        }

        if self.aarch64_va_is_out_of_range(va, accdesc.acctype, regime, &walkparams) {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (fault, AddressDescriptor::arbitrary());
        }

        if accdesc.el == EL0 && walkparams.e0pd {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (fault, AddressDescriptor::arbitrary());
        }

        // TODO: SVE feature

        todo!()
    }
}
