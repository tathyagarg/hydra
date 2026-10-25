use crate::cpu::{
    Cpu, Feature,
    common::*,
    constraint::{Constraint, Unpredictable},
    exeception::{FullAddress, PASpace},
    fault::{
        AccessDescriptor, AccessType, CacheOp, CacheType, ErrorState, Fault, FaultRecord, VARange,
    },
    regs::{
        E2H_TCR2_EL2Field, E2H0_TCR_EL2Field, E2H1_TCR_EL2Field, HCR_EL2Field, MAIRType,
        MECID_P0_EL2, MECID_P1_EL2, RegisterField, RegisterFieldRange, S1PIRType, S1PORType,
        SCR_EL3Field, SCTLR_EL1Field, SCTLR_EL2Field, SCTLR_EL3Field, SCTLR2_EL2Field,
        SCTLR2_EL3Field, SCTLRField, TCR_EL1Field, TCR_EL2CommonField, TCR_EL3Field, TCR2_EL1Field,
        TCR2_EL2Field, VMECID_P_EL2, VTCR_EL2Field, mairtype_from_mair,
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

#[derive(PartialEq, Clone)]
pub struct Bits<const N: usize> {
    // bytes.len() == N / 8
    bytes: Vec<u8>,
}

impl<const N: usize> Bits<N> {
    pub fn zero() -> Bits<N> {
        Bits { bytes: vec![0] }
    }

    pub fn reverse(&self) -> Self {
        Self {
            bytes: self.bytes.iter().rev().copied().collect(),
        }
    }

    pub fn nth_bit(&self, n: usize) -> bool {
        let byte_index = n / 8;
        if byte_index > self.bytes.len() {
            panic!("Byte index out of range");
        }

        nth_bit(self.bytes[byte_index], n % 8)
    }

    pub fn get_bits(&self, x: usize, y: usize) -> u64 {
        let mut result = 0u64;

        for bit in x..=y {
            let byte_index = bit / 8;
            let bit_index = bit % 8;
            let value = (self.bytes[byte_index] >> bit_index) & 1;

            result |= (value as u64) << (bit - x);
        }

        result
    }

    pub fn set_bit(&mut self, n: usize, v: bool) {
        let byte_index = n / 8;
        let bit_index = n % 8;
        let mask = 1u8 << bit_index;

        if v {
            self.bytes[byte_index] |= mask;
        } else {
            self.bytes[byte_index] &= !mask;
        }
    }

    pub fn compile_u128(&self) -> u128 {
        let mut result = 0u128;
        for byte in &self.bytes {
            result |= *byte as u128;
            result <<= 8;
        }

        result
    }
}

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

        match regime {
            Regime::EL2 => MECID_P0_EL2::MECID.fetch_range(self.mecid_p0_el2),
            Regime::EL20 => {
                if !E2H1_TCR_EL2Field::A1.fetch_bit(self.tcr_el2) {
                    return MECID_P1_EL2::MECID.fetch_range(self.mecid_p1_el2);
                } else {
                    return MECID_P0_EL2::MECID.fetch_range(self.mecid_p0_el2);
                }
            }
            Regime::EL10 => VMECID_P_EL2::MECID.fetch_range(self.vmecid_p_el2),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1dcache_enabled(&self, regime: Regime) -> bool {
        match regime {
            Regime::EL3 => SCTLR_EL3Field::C.fetch_bit(self.sctlr_el3),
            Regime::EL2 => SCTLR_EL2Field::C.fetch_bit(self.sctlr_el2),
            Regime::EL20 => SCTLR_EL2Field::C.fetch_bit(self.sctlr_el2),
            Regime::EL10 => SCTLR_EL1Field::C.fetch_bit(self.sctlr_el1),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_ttentry_address_bounds(
        &self,
        level: u8,
        txsz: u8,
        tgx: TGx,
        descsizelog2: u8,
        start_level_lookup: bool,
        nstride: u8,
    ) -> (AddressSize, AddressSize) {
        let iasize = self.aarch64_iasize(txsz);
        let granulebits = self.tgx_granulebits(tgx);
        let stride = granulebits - descsizelog2;
        let level = FINAL_LEVEL - level;

        let lsb = (level * stride + granulebits) as AddressSize;
        let msb = if start_level_lookup {
            (iasize - 1) as AddressSize
        } else {
            ((lsb + (stride * nstride)) - 1) as AddressSize
        };

        (lsb, msb)
    }

    pub fn aarch64_s1sltt_entry_address(
        &self,
        level: u8,
        walkparams: &S1TTWParams,
        ia: u64,
        tablebase: &FullAddress,
    ) -> FullAddress {
        let descsizelog2 = 3 + (walkparams.d128 as u8);
        let nstride = 1u8;
        let start_level_lookup = true;

        let (lsb, msb) = self.aarch64_ttentry_address_bounds(
            level,
            walkparams.txsz,
            walkparams.tgx,
            descsizelog2,
            start_level_lookup,
            nstride,
        );
        let index = ((ia >> lsb) & ((1u64 << (msb - lsb + 1)) - 1)) << descsizelog2;

        FullAddress {
            address: tablebase.address | index,
            paspace: tablebase.paspace,
        }
    }

    pub fn aarch64_pamax(&self) -> AddressSize {
        self.imp_def_int("Maximum Physical Address Size") as AddressSize
    }

    pub fn aarch64_physical_address_size(
        &self,
        d128: bool,
        ds: bool,
        encoded_ps: u8,
        tgx: TGx,
    ) -> AddressSize {
        let ps = match encoded_ps {
            0b000 => 32,
            0b001 => 36,
            0b010 => 40,
            0b011 => 42,
            0b100 => 44,
            0b101 => 48,
            0b110 => 52,
            0b111 => 56,
            _ => unreachable!(),
        } as u8;

        let max_ps = if d128 {
            self.aarch64_pamax()
        } else if Cpu::is_feature_implemented(Feature::LPA) && (tgx == TGx::_64KB || ds) {
            self.aarch64_pamax().min(52)
        } else {
            self.aarch64_pamax().min(48)
        };

        ps.max(max_ps) as AddressSize
    }

    pub fn aarch64_oaout_of_range(
        &self,
        address: u64,
        d128: bool,
        ds: bool,
        ps: u8,
        tgx: TGx,
    ) -> bool {
        let oasize = self.aarch64_physical_address_size(d128, ds, ps, tgx);

        if oasize < NUM_PABITS {
            ((address >> oasize) & ones((NUM_PABITS - oasize) as u32)) != 0
        } else {
            false
        }
    }

    pub fn aarch64_s1oaout_of_range(&self, address: u64, walkparams: &S1TTWParams) -> bool {
        self.aarch64_oaout_of_range(
            address,
            walkparams.d128,
            walkparams.ds,
            walkparams.ps,
            walkparams.tgx,
        )
    }

    #[allow(unused_variables)]
    pub fn phys_mem_read<const N: usize>(
        &self,
        desc: &AddressDescriptor,
        accdesc: &AccessDescriptor,
    ) -> (PhysMemRetStatus, Bits<N>)
    where
        Size<N>: ValidMemReadSizeBits,
    {
        todo!("Read")
    }

    pub fn fetch_descriptor<const N: usize>(
        &self,
        ee: bool,
        walkaddress: &AddressDescriptor,
        walkaccess: &AccessDescriptor,
        fault_in: &FaultRecord,
    ) -> (FaultRecord, Bits<N>)
    where
        Size<N>: ValidMemReadSizeBits,
    {
        let fault = fault_in.clone();

        if Cpu::is_feature_implemented(Feature::RME) {
            todo!("RME stuff");
        }

        let (memstatus, mut descriptor) = self.phys_mem_read::<N>(walkaddress, walkaccess);
        if is_fault_retstatus(&memstatus) {
            todo!("Fault booo")
        }

        if ee {
            descriptor = descriptor.reverse();
        }

        (fault, descriptor)
    }

    pub fn aarch64_block_desc_supported(&self, d128: bool, ds: bool, tgx: TGx, level: u8) -> bool {
        match tgx {
            TGx::_4KB => (level == 0 && (ds || d128)) || level == 1 || level == 2,
            TGx::_16KB => (level == 1 && (ds || d128)) || level == 2,
            TGx::_64KB => (level == 1 && (d128 || self.aarch64_pamax() >= 52)) || level == 2,
        }
    }

    pub fn aarch64_decode_descriptor_type<const N: usize>(
        &self,
        desc: &Bits<N>,
        d128: bool,
        ds: bool,
        tgx: TGx,
        level: u8,
    ) -> DescriptorType {
        if !desc.nth_bit(0) {
            return DescriptorType::Invalid;
        } else if d128 {
            let skl = ((desc.nth_bit(110) as u8) << 1) | desc.nth_bit(109) as u8;
            if [TGx::_16KB, TGx::_64KB].contains(&tgx) && skl == 3 {
                return DescriptorType::Invalid;
            }

            let effective_level = level + skl;
            if effective_level > FINAL_LEVEL {
                return DescriptorType::Invalid;
            } else if effective_level == FINAL_LEVEL {
                return DescriptorType::Leaf;
            } else {
                return DescriptorType::Table;
            }
        } else {
            if desc.nth_bit(1) {
                if level == FINAL_LEVEL {
                    return DescriptorType::Leaf;
                } else {
                    return DescriptorType::Table;
                }
            } else {
                if self.aarch64_block_desc_supported(d128, ds, tgx, level) {
                    return DescriptorType::Leaf;
                } else {
                    return DescriptorType::Invalid;
                }
            }
        }
    }

    pub fn aarch64_next_table_base<const N: usize>(
        &self,
        desc: &Bits<N>,
        d128: bool,
        skl: u8,
        ds: bool,
        tgx: TGx,
    ) -> u64 {
        let mut tablebase;
        let granulebits = self.tgx_granulebits(tgx);
        let tablesize = if d128 {
            let descsizelog2 = 4u8;
            let stride = granulebits - descsizelog2;
            stride * (1 + skl) + descsizelog2
        } else {
            granulebits
        };

        tablebase = match tgx {
            TGx::_4KB => desc.get_bits(47, 12),
            TGx::_16KB => desc.get_bits(47, 14),
            TGx::_64KB => desc.get_bits(47, 16),
        };

        tablebase = align_down_p2::<56>(tablebase, tablesize as u32);

        if d128 {
            tablebase |= desc.get_bits(48, 55) << 48;
        } else if tgx == TGx::_64KB && self.aarch64_pamax() >= 52 {
            tablebase |= desc.get_bits(12, 15) << 48;
        } else if ds {
            tablebase |= (desc.get_bits(8, 9) << 50) | (desc.get_bits(48, 49) << 48);
        };

        tablebase
    }

    pub fn aarch64_s1apply_table_perms<const N: usize>(
        &self,
        perms_in: &Permissions,
        desc: &Bits<N>,
        regime: Regime,
        walkparams: &S1TTWParams,
    ) -> Permissions {
        if regime == Regime::EL10 && self.el2_enabled() && walkparams.nv1 {
            Permissions {
                ap_table: perms_in.ap_table | ((desc.nth_bit(62) as u8) << 1),
                pxn_table: perms_in.pxn_table | desc.nth_bit(60),
                ..Default::default()
            }
        } else if self.has_unprivileged(regime) {
            Permissions {
                ap_table: perms_in.ap_table | (desc.get_bits(62, 61) as u8),
                uxn_table: perms_in.uxn_table | desc.nth_bit(60),
                pxn_table: perms_in.pxn_table | desc.nth_bit(59),
                ..Default::default()
            }
        } else {
            Permissions {
                ap_table: perms_in.ap_table | ((desc.nth_bit(62) as u8) << 1),
                xn_table: perms_in.xn_table | desc.nth_bit(60),
                ..Default::default()
            }
        }
    }

    pub fn aarch64_s1next_walk_state_table<const N: usize>(
        &self,
        currentstate: &TTWState,
        s2fs1mro: bool,
        regime: Regime,
        walkparams: &S1TTWParams,
        desc: &Bits<N>,
    ) -> TTWState {
        let skl = if walkparams.d128 {
            ((desc.nth_bit(110) as u8) << 1) | desc.nth_bit(109) as u8
        } else {
            0
        };

        let tablebase = FullAddress {
            address: self.aarch64_next_table_base(
                desc,
                walkparams.d128,
                skl,
                walkparams.ds,
                walkparams.tgx,
            ),
            paspace: if currentstate.baseaddress.paspace == PASpace::Secure {
                let nstable = if walkparams.d128 {
                    desc.nth_bit(127)
                } else {
                    desc.nth_bit(63)
                };
                if !nstable {
                    PASpace::Secure
                } else {
                    PASpace::NonSecure
                }
            } else {
                currentstate.baseaddress.paspace
            },
        };

        let protectedbit = if walkparams.d128 {
            desc.nth_bit(114)
        } else {
            if walkparams.pnch {
                desc.nth_bit(52)
            } else {
                false
            }
        };

        TTWState {
            istable: true,
            level: if walkparams.d128 {
                currentstate.level + skl + 1
            } else {
                currentstate.level + 1
            },
            baseaddress: tablebase,
            contiguous: false,
            s1assured: currentstate.s1assured && s2fs1mro && protectedbit,
            s2assuredonly: false,
            disch: if walkparams.d128 {
                desc.nth_bit(112)
            } else {
                false
            },
            nG: currentstate.nG,
            guardedpage: false,
            sdftype: SDFType::Invalid,
            domain: 0,
            memattrs: currentstate.memattrs.clone(),
            permissions: if !walkparams.hpd && !walkparams.pie {
                self.aarch64_s1apply_table_perms(
                    &currentstate.permissions,
                    desc,
                    regime,
                    walkparams,
                )
            } else {
                currentstate.permissions.clone()
            },
        }
    }

    pub fn aarch64_s1ttentry_address(
        &self,
        level: u8,
        walkparams: &S1TTWParams,
        skl: u8,
        ia: u64,
        tablebase: &FullAddress,
    ) -> FullAddress {
        let nstride = if walkparams.d128 { skl + 1 } else { 1 };
        let descsizelog2 = if walkparams.d128 { 4u8 } else { 3u8 };
        let start_level_lookup = false;

        let (lsb, msb) = self.aarch64_ttentry_address_bounds(
            level,
            walkparams.txsz,
            walkparams.tgx,
            descsizelog2,
            start_level_lookup,
            nstride,
        );

        let index = (get_bits(ia, msb as usize, lsb as usize) as u64) << descsizelog2;

        FullAddress {
            address: tablebase.address | index,
            paspace: tablebase.paspace,
        }
    }

    pub fn aarch64_ntfaults<const N: usize>(&self, d128: bool, desc: &Bits<N>) -> bool {
        if d128 {
            desc.nth_bit(6)
        } else {
            desc.nth_bit(16)
        }
    }

    pub fn aarch64_s1contiguous_bit_faults(
        &self,
        walkparams: &S1TTWParams,
        walkstate: &TTWState,
    ) -> bool {
        let iasize = self.aarch64_iasize(walkparams.txsz);
        let tsize = self.translation_size(walkparams.d128, walkparams.tgx, walkstate.level);
        let csize = self.contiguous_size(walkparams.d128, walkparams.tgx, walkstate);

        (tsize + csize) > iasize
    }

    pub fn aarch64_s1amec_fault<const N: usize>(
        &self,
        walkparams: &S1TTWParams,
        paspace: PASpace,
        regime: Regime,
        desc: &Bits<N>,
    ) -> bool {
        assert!((N == 64) | (N == 128));
        let descriptor_amec = if walkparams.d128 {
            desc.nth_bit(108)
        } else {
            desc.nth_bit(63)
        };

        walkparams.emec
            && !walkparams.amec
            && [Regime::EL2, Regime::EL20].contains(&regime)
            && paspace == PASpace::Realm
            && descriptor_amec
    }

    pub fn aarch64_s1walk<const N: usize>(
        &self,
        fault_in: FaultRecord,
        walkparams: &S1TTWParams,
        va: u64,
        regime: Regime,
        accdesc: &AccessDescriptor,
    ) -> (FaultRecord, AddressDescriptor, TTWState, Bits<N>)
    where
        Size<N>: ValidWalkSize + ValidMemReadSizeBits,
    {
        let mut skl: u8;
        let mut fault = fault_in;
        let mut aligned = false;

        if self.has_unprivileged(regime) && self.aarch64_s1epd(regime, va) {
            fault.statuscode = Fault::Translation;
            fault.level = 0;
            return (
                fault,
                AddressDescriptor::arbitrary(),
                TTWState::arbitrary(),
                Bits::zero(),
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
                Bits::zero(),
            );
        }

        let mecid = self.aarch64_s1ttwalk_mecid(walkparams.emec, regime, accdesc.ss);

        let mut memattrs = if !self.aarch64_s1dcache_enabled(regime) {
            let mut attrs = self.normal_nc_memattr();
            attrs.xs = walkstate.memattrs.xs;
            attrs
        } else {
            walkstate.memattrs.clone()
        };

        memattrs.shareability = if regime == Regime::EL10
            && self.el2_enabled()
            && HCR_EL2Field::VM.fetch_bit(self.hcr_el2)
        {
            walkstate.memattrs.shareability
        } else {
            self.effective_shareability(&memattrs)
        };

        let mut s2fs1mro = false;
        let mut descaddress = self.aarch64_s1sltt_entry_address(
            walkstate.level,
            walkparams,
            va,
            &walkstate.baseaddress,
        );

        if self.aarch64_s1oaout_of_range(descaddress.address, walkparams) {
            fault.statuscode = Fault::AddressSize;
            fault.level = 0;
            return (
                fault,
                AddressDescriptor::arbitrary(),
                TTWState::arbitrary(),
                Bits::zero(),
            );
        }

        let mut paddress: FullAddress;
        let mut s1assured: bool;

        let mut desctype: DescriptorType;

        let mut walkaddress = AddressDescriptor {
            vaddress: va,
            mecid,
            memattrs,
            fault: FaultRecord::no_fault(),
            paddress: FullAddress {
                paspace: PASpace::Root,
                address: 0,
            }, // replaced later
            s1assured: false,
            s2fs1mro,
        };

        let mut descriptor: Bits<N>;

        loop {
            fault.level = walkstate.level;
            walkaddress.paddress = descaddress;
            walkaddress.s1assured = walkstate.s1assured;

            let toplevel = walkstate.level == startlevel;
            let varange = self.aarch64_get_varange(va);
            let walkaccess = self.create_accdesc_s1ttw(toplevel, varange, &accdesc);

            descriptor = if regime == Regime::EL10 && self.el2_enabled() {
                todo!("End it all bruh")
            } else {
                let (f, descriptor) =
                    self.fetch_descriptor::<N>(walkparams.ee, &walkaddress, &walkaccess, &fault);
                fault = f;
                descriptor
            };

            if fault.statuscode != Fault::None {
                return (
                    fault,
                    AddressDescriptor::arbitrary(),
                    TTWState::arbitrary(),
                    Bits::zero(),
                );
            }

            loop {
                let mut new_desc = descriptor.clone();
                desctype = self.aarch64_decode_descriptor_type(
                    &descriptor,
                    walkparams.d128,
                    walkparams.ds,
                    walkparams.tgx,
                    walkstate.level,
                );

                match desctype {
                    DescriptorType::Table => {
                        walkstate = self.aarch64_s1next_walk_state_table(
                            &walkstate,
                            s2fs1mro,
                            regime,
                            walkparams,
                            &descriptor,
                        );

                        skl = if walkparams.d128 {
                            descriptor.get_bits(110, 109) as u8
                        } else {
                            0
                        };

                        descaddress = self.aarch64_s1ttentry_address(
                            walkstate.level,
                            walkparams,
                            skl,
                            va,
                            &walkstate.baseaddress,
                        );

                        if self.aarch64_s1oaout_of_range(descaddress.address, walkparams) {
                            fault.statuscode = Fault::Translation;
                            return (
                                fault,
                                AddressDescriptor::arbitrary(),
                                TTWState::arbitrary(),
                                Bits::zero(),
                            );
                        }

                        if walkparams.haft {
                            new_desc.set_bit(10, true);
                        }

                        if walkparams.d128
                            && skl != 0b00
                            && self.aarch64_ntfaults(walkparams.d128, &descriptor)
                        {
                            fault.statuscode = Fault::Translation;
                            return (
                                fault,
                                AddressDescriptor::arbitrary(),
                                TTWState::arbitrary(),
                                Bits::zero(),
                            );
                        }
                    }
                    DescriptorType::Invalid => {
                        fault.statuscode = Fault::Translation;
                        return (
                            fault,
                            AddressDescriptor::arbitrary(),
                            TTWState::arbitrary(),
                            Bits::zero(),
                        );
                    }
                    _ => todo!(),
                }

                if new_desc != descriptor {
                    if !self.is_wb_shareable(&walkaddress.memattrs)
                        && Unpredictable::Unsupported_Atomic_HW_Update.constrain_bool()
                    {
                        fault.statuscode = Fault::UnsupportedAtomicHWUpdate;
                        return (
                            fault,
                            AddressDescriptor::arbitrary(),
                            TTWState::arbitrary(),
                            Bits::zero(),
                        );
                    }

                    let descaccess = self.create_accdesc_tte_update(&accdesc);
                    let descpaddr = if regime == Regime::EL10 && self.el2_enabled() {
                        let s1aarch64 = true;
                        aligned = true;

                        todo!()
                    } else {
                        walkaddress.clone()
                    };

                    todo!("MemSwapTableDesc");
                }

                if new_desc == descriptor {
                    break;
                }
            }

            if desctype == DescriptorType::Leaf {
                break;
            }
        }

        let oa = self.stage1oa(va, walkparams, &walkstate);

        if walkstate.contiguous && self.aarch64_s1contiguous_bit_faults(walkparams, &walkstate) {
            fault.statuscode = Fault::Translation;
        } else if walkstate.level < FINAL_LEVEL
            && self.aarch64_ntfaults(walkparams.d128, &descriptor)
        {
            fault.statuscode = Fault::Translation;
        } else if self.aarch64_s1amec_fault(
            walkparams,
            walkstate.baseaddress.paspace,
            regime,
            &descriptor,
        ) {
            fault.statuscode = Fault::Translation;
        } else if self.aarch64_s1oaout_of_range(oa.address, walkparams) {
            fault.statuscode = Fault::AddressSize;
        } else if !descriptor.nth_bit(10)
            && !walkparams.ha
            && ![AccessType::DC, AccessType::IC].contains(&accdesc.acctype)
        {
            fault.statuscode = Fault::AccessFlag;
        }

        if fault.statuscode != Fault::None {
            return (
                fault,
                AddressDescriptor::arbitrary(),
                TTWState::arbitrary(),
                Bits::zero(),
            );
        }

        (fault, walkaddress, walkstate, descriptor)
    }

    pub fn aarch64_s1has_alignment_fault_due_to_memtype(
        &self,
        regime: Regime,
        accdesc: &AccessDescriptor,
        aligned: bool,
        ntlsmd: bool,
        memattrs: &MemoryAttributes,
    ) -> bool {
        if accdesc.exclusive || accdesc.atomicop || accdesc.acqsc || accdesc.acqpc || accdesc.relsc
        {
            if !aligned
                && !(self.is_wb_shareable(memattrs) && self.aarch64_s1dcache_enabled(regime))
                && Unpredictable::LSE2_ALIGNMENT_FAULT.constrain_bool()
            {
                return true;
            }
        }

        if memattrs.memtype != MemType::Device {
            return false;
        } else if (accdesc.acctype == AccessType::DCZero
            && accdesc.cachetype == Some(CacheType::Tag))
            || accdesc.stzgm
        {
            return Unpredictable::DEVICETAGSTORE.constrain() == Constraint::FAULT;
        } else if accdesc.a32lsmd && !ntlsmd {
            return memattrs.device != DeviceType::GRE;
        } else if accdesc.acctype == AccessType::DCZero {
            return true;
        } else if !aligned {
            // TODO: ImpDefBool
            return false;
        } else {
            return false;
        }
    }

    pub fn aarch64_permissions_overlays_applied(&self, acctype: AccessType) -> bool {
        match acctype {
            AccessType::SPE | AccessType::TRBE => false,
            _ => true,
        }
    }

    pub fn aarch64_s1indirect_base_permissions(
        &self,
        regime: Regime,
        walkstate: &TTWState,
        walkparams: &S1TTWParams,
        accdesc: &AccessDescriptor,
    ) -> S1AccessControls {
        let permissions = &walkstate.permissions;

        let (mut ur, mut uw, mut ux, mut ugcs, mut uwxn, mut u_overlay) =
            (false, false, false, false, false, false);

        let (mut pr, mut pw, mut px, mut pgcs) = match permissions.ppi {
            0b0000 => (false, false, false, false),
            0b0001 => (true, false, false, false),
            0b0010 => (false, false, true, false),
            0b0011 => (true, false, true, false),
            0b0100 => (false, false, false, false),
            0b0101 => (true, true, false, false),
            0b0110 => (true, true, true, false),
            0b0111 => (true, true, true, false),
            0b1000 => (true, false, false, false),
            0b1001 => {
                if Cpu::is_feature_implemented(Feature::GCS) {
                    (true, false, false, true)
                } else {
                    (false, false, false, false)
                }
            }
            0b1010 => (true, false, true, false),
            0b1011 => (false, false, false, false),
            0b1100 => (true, true, false, false),
            0b1101 => (false, false, false, false),
            0b1110 => (true, true, true, false),
            0b1111 => (false, false, false, false),
            _ => unreachable!(),
        };

        let p_overlay = !nth_bit(permissions.ppi, 3);
        let pwxn = permissions.ppi == 0b0110;
        let mut reserved_upi = false;

        if self.has_unprivileged(regime) {
            (ur, uw, ux, ugcs) = match permissions.upi {
                0b0000 => (false, false, false, false),
                0b0001 => (true, false, false, false),
                0b0010 => (false, false, true, false),
                0b0011 => (true, false, true, false),
                0b0100 => {
                    reserved_upi = true;
                    (false, false, false, false)
                }
                0b0101 => (true, true, false, false),
                0b0110 => (true, true, true, false),
                0b0111 => (true, true, true, false),
                0b1000 => (true, false, false, false),
                0b1001 => {
                    if Cpu::is_feature_implemented(Feature::GCS) {
                        (true, false, false, true)
                    } else {
                        reserved_upi = true;
                        (false, false, false, false)
                    }
                }
                0b1010 => (true, false, true, false),
                0b1011 => {
                    reserved_upi = true;
                    (false, false, false, false)
                }
                0b1100 => (true, true, false, false),
                0b1101 => {
                    reserved_upi = true;
                    (false, false, false, false)
                }
                0b1110 => (true, true, true, false),
                0b1111 => {
                    reserved_upi = true;
                    (false, false, false, false)
                }
                _ => unreachable!(),
            };

            u_overlay = !nth_bit(permissions.upi, 3);
            uwxn = permissions.upi == 0b0110;

            if (px || pgcs) && (uw || ugcs) {
                (pr, pw, px, pgcs) = (false, false, false, false);
                (ur, uw, ux, ugcs) = (false, false, false, false);
            }
        }

        if accdesc.ss == SecurityState::Secure
            && walkstate.baseaddress.paspace == PASpace::NonSecure
        {
            pgcs = false;
            if self.has_unprivileged(regime) {
                ugcs = false;
            }

            if walkparams.sif {
                px = false;
                if self.has_unprivileged(regime) {
                    ux = false;
                }
            }
        }

        if accdesc.ss == SecurityState::Root && walkstate.baseaddress.paspace != PASpace::Root {
            (px, pgcs) = (false, false);
            if self.has_unprivileged(regime) {
                (ux, ugcs) = (false, false);
            }
        }

        if accdesc.ss == SecurityState::Realm
            && [Regime::EL2, Regime::EL20].contains(&regime)
            && walkstate.baseaddress.paspace != PASpace::Realm
        {
            (px, pgcs) = (false, false);
            if self.has_unprivileged(regime) {
                (ux, ugcs) = (false, false);
            }
        }

        if self.has_unprivileged(regime)
            && Cpu::is_feature_implemented(Feature::PAN)
            && accdesc.pan
            && !(regime == Regime::EL10 && walkparams.nv1)
            && self.pstate.PAN
            && permissions.upi != 0
        {
            if reserved_upi {
                todo!("Imp Def Bool");
            }

            todo!("Imp Def Bool");
        }

        let (r, w, x, gcs, wxn, overlay) = if accdesc.el == EL0 {
            (ur, uw, ux, ugcs, uwxn, u_overlay)
        } else {
            (pr, pw, px, pgcs, pwxn, p_overlay)
        };

        S1AccessControls {
            r,
            w,
            x,
            gcs,
            wxn,
            overlay: self.aarch64_permissions_overlays_applied(accdesc.acctype) && overlay,
            or: false,
            ow: false,
            ox: false,
        }
    }

    pub fn aarch64_s1e0po_enabled(&self, regime: Regime, nv1: bool) -> bool {
        assert!(self.has_unprivileged(regime));

        if !Cpu::is_feature_implemented(Feature::S1POE) {
            return false;
        }

        match regime {
            Regime::EL20 => {
                self.is_tcr2el2_enabled() && E2H_TCR2_EL2Field::E0POE.fetch_bit(self.tcr2_el2)
            }
            Regime::EL10 => {
                self.is_tcr2el1_enabled() && !nv1 && TCR2_EL1Field::E0POE.fetch_bit(self.tcr2_el1)
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1po_enabled(&self, regime: Regime) -> bool {
        if !Cpu::is_feature_implemented(Feature::S1POE) {
            return false;
        }

        match regime {
            Regime::EL3 => TCR_EL3Field::POE.fetch_bit(self.tcr_el3),
            Regime::EL2 | Regime::EL20 => {
                self.is_tcr2el2_enabled() && !TCR2_EL2Field::POE.fetch_bit(self.tcr2_el2)
            }
            Regime::EL10 => {
                self.is_tcr2el1_enabled() && !TCR2_EL1Field::POE.fetch_bit(self.tcr2_el2)
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1por(&self, regime: Regime) -> S1PORType {
        match regime {
            Regime::EL3 => self.por_el3 as S1PORType,
            Regime::EL2 => self.por_el2 as S1PORType,
            Regime::EL20 => self.por_el2 as S1PORType,
            Regime::EL10 => self.por_el1 as S1PORType,
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1overlay_perms(
        &self,
        regime: Regime,
        walkstate: &TTWState,
        accdesc: &AccessDescriptor,
    ) -> S1AccessControls {
        let (mut ur, mut uw, mut ux) = (false, false, false);

        let perms = &walkstate.permissions;
        let por = self.aarch64_s1por(regime);

        let ppo = por[perms.po_index as usize];

        let (pr, pw, px) = match ppo {
            0b0000 => (false, false, false),
            0b0001 => (true, false, false),
            0b0010 => (false, false, true),
            0b0011 => (true, false, true),
            0b0100 => (false, true, false),
            0b0101 => (true, true, false),
            0b0110 => (false, true, true),
            0b0111 => (true, true, true),
            0b1000..=0b1111 => (false, false, false),
            _ => unreachable!(),
        };

        if self.has_unprivileged(regime) {
            let upo = if !(self.have_el(EL3) && !SCR_EL3Field::PIEn.fetch_bit(self.scr_el3)) {
                self.por_el0[perms.po_index as usize]
            } else {
                0
            };

            (ur, uw, ux) = match upo {
                0b0000 => (false, false, false),
                0b0001 => (true, false, false),
                0b0010 => (false, false, true),
                0b0011 => (true, false, true),
                0b0100 => (false, true, false),
                0b0101 => (true, true, false),
                0b0110 => (false, true, true),
                0b0111 => (true, true, true),
                0b1000..=0b1111 => (false, false, false),
                _ => unreachable!(),
            };
        }

        let (r, w, x) = if accdesc.el == EL0 {
            (ur, uw, ux)
        } else {
            (pr, pw, px)
        };

        S1AccessControls {
            r: false,
            w: false,
            x: false,
            gcs: false,
            overlay: false,
            or: r,
            ow: w,
            ox: x,
            wxn: false,
        }
    }

    pub fn aarch64_s1compute_permissions(
        &self,
        regime: Regime,
        walkstate: &TTWState,
        walkparams: &S1TTWParams,
        accdesc: &AccessDescriptor,
    ) -> S1AccessControls {
        let mut s1perms = if walkparams.pie {
            self.aarch64_s1indirect_base_permissions(regime, walkstate, walkparams, accdesc)
        } else {
            todo!()
        };

        if accdesc.el == EL0 && !self.aarch64_s1e0po_enabled(regime, walkparams.nv1) {
            s1perms.overlay = false;
        } else if accdesc.el != EL0 && !self.aarch64_s1po_enabled(regime) {
            s1perms.overlay = false;
        }

        if s1perms.overlay {
            let s1overlay_perms = self.aarch64_s1overlay_perms(regime, walkstate, accdesc);

            s1perms.or = s1overlay_perms.or;
            s1perms.ow = s1overlay_perms.ow;
            s1perms.ox = s1overlay_perms.ox;
        }

        if s1perms.overlay {
            s1perms.ow = s1perms.ow && !(s1perms.wxn && s1perms.ox);
        } else {
            s1perms.x = s1perms.x && !s1perms.wxn;
        }

        s1perms
    }

    pub fn aarch64_s1check_permissions(
        &self,
        fault_in: &FaultRecord,
        regime: Regime,
        walkstate: &TTWState,
        walkparams: &S1TTWParams,
        accdesc: &AccessDescriptor,
    ) -> FaultRecord {
        let mut fault = fault_in.clone();
        let permissions = &walkstate.permissions;
        let s1perms = self.aarch64_s1compute_permissions(regime, walkstate, walkparams, accdesc);

        if accdesc.acctype == AccessType::IFETCH {
            // TODO: Guarded Page

            if s1perms.overlay && !s1perms.ox {
                fault.statuscode = Fault::Permission;
                fault.overlay = true;
            } else if walkstate.memattrs.memtype == MemType::Device
                && Unpredictable::INSTRDEVICE.constrain() == Constraint::FAULT
            {
                fault.statuscode = Fault::Permission;
            } else if !s1perms.x {
                fault.statuscode = Fault::Permission;
            }
        } else if accdesc.acctype == AccessType::DC {
            if accdesc.cacheop == Some(CacheOp::Invalidate) {
                if s1perms.overlay && !s1perms.ow {
                    fault.statuscode = Fault::Permission;
                    fault.overlay = true;
                } else if !s1perms.w {
                    fault.statuscode = Fault::Permission;
                } else if !walkparams.hd && walkparams.pie && permissions.ndirty {
                    fault.statuscode = Fault::Permission;
                    fault.dirtybit = true;
                }
            } else if accdesc.el == EL0 {
                if s1perms.overlay && !s1perms.or {
                    fault.statuscode = Fault::Permission;
                    fault.overlay = true;
                } else if walkparams.cmow
                    && accdesc.cacheop == Some(CacheOp::CleanInvalidate)
                    && s1perms.overlay
                    && !s1perms.ow
                {
                    fault.statuscode = Fault::Permission;
                    fault.overlay = true;
                } else if !s1perms.r {
                    fault.statuscode = Fault::Permission;
                } else if walkparams.cmow
                    && accdesc.cacheop == Some(CacheOp::CleanInvalidate)
                    && !s1perms.w
                {
                    fault.statuscode = Fault::Permission;
                } else if walkparams.cmow
                    && !walkparams.hd
                    && walkparams.pie
                    && permissions.ndirty
                    && accdesc.cacheop == Some(CacheOp::CleanInvalidate)
                {
                    fault.statuscode = Fault::Permission;
                    fault.dirtybit = true;
                }
            }
        } else if accdesc.acctype == AccessType::IC {
            if accdesc.el == EL0 {
                if s1perms.overlay && !s1perms.or {
                    fault.statuscode = Fault::Permission;
                    fault.overlay = true;
                } else if walkparams.cmow && s1perms.overlay && !s1perms.ow {
                    fault.statuscode = Fault::Permission;
                    fault.overlay = true;
                } else if !s1perms.r {
                    fault.statuscode = Fault::Permission;
                } else if walkparams.cmow && !s1perms.w {
                    fault.statuscode = Fault::Permission;
                } else if walkparams.cmow && !walkparams.hd && walkparams.pie && permissions.ndirty
                {
                    fault.statuscode = Fault::Permission;
                    fault.dirtybit = true;
                }
            }
        } else if Cpu::is_feature_implemented(Feature::GCS) && accdesc.acctype == AccessType::GCS {
            if !s1perms.gcs {
                fault.statuscode = Fault::Permission;
            } else if accdesc.write && !walkparams.hd && permissions.ndirty {
                fault.statuscode = Fault::Permission;
                fault.dirtybit = true;
                fault.write = true;
            }
        } else if accdesc.read && s1perms.overlay && !s1perms.or {
            fault.statuscode = Fault::Permission;
            fault.overlay = true;
            fault.write = false;
        } else if accdesc.write && s1perms.overlay && !s1perms.ow {
            fault.statuscode = Fault::Permission;
            fault.overlay = true;
            fault.write = true;
        } else if accdesc.read && !s1perms.r {
            fault.statuscode = Fault::Permission;
            fault.write = false;
        } else if accdesc.write && !s1perms.w {
            fault.statuscode = Fault::Permission;
            fault.write = true;
        } else if accdesc.write
            && accdesc.tagaccess
            && walkstate.memattrs.tags == MemTagType::CanonicallyTagged
        {
            fault.statuscode = Fault::Permission;
            fault.write = true;
            fault.s1tagnotdata = true;
        } else if accdesc.write && !walkparams.hd && walkparams.pie && permissions.ndirty {
            fault.statuscode = Fault::Permission;
            fault.dirtybit = true;
            fault.write = true;
        }

        fault
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

        // let mut descipaddr: AddressDescriptor;
        // let mut walkstate: TTWState;
        // let mut descriptor: u128;

        let (descipaddr, walkstate, descriptor) = if walkparams.d128 {
            let (f, da, w, dsc) =
                self.aarch64_s1walk::<128>(fault, &walkparams, va, regime, accdesc);
            fault = f;
            (da, w, dsc.compile_u128())
        } else {
            let (f, da, w, dsc) =
                self.aarch64_s1walk::<64>(fault, &walkparams, va, regime, accdesc);
            fault = f;
            (da, w, dsc.compile_u128())
        };

        if fault.statuscode != Fault::None {
            return (fault, AddressDescriptor::arbitrary());
        }

        if self.aarch64_s1has_alignment_fault_due_to_memtype(
            regime,
            accdesc,
            aligned,
            walkparams.ntlsmd,
            &walkstate.memattrs,
        ) {
            fault.statuscode = Fault::Alignment;
        }

        todo!()
    }
}
