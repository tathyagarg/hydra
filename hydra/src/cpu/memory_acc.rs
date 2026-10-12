use crate::cpu::{
    Cpu, Feature,
    common::*,
    exeception::FullAddress,
    fault::{AccessDescriptor, AccessType, ErrorState, Fault, FaultRecord, VARange},
    regs::{
        HCR_EL2Field, MAIRType, RegisterField, RegisterFieldRange, S1PIRType, SCR_EL3Field,
        SCTLRField, TCR_EL3Field, mairtype_from_mair,
    },
    shared::{EL, Regime, SecurityState},
};

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

#[derive(PartialEq)]
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
        // let emec = if Cpu::is_feature_implemented(Feature::MEC) {
        //     SCT
        // }

        todo!()
    }

    pub fn aarch64_gets1ttw_params(
        &self,
        regime: Regime,
        el: EL,
        ss: SecurityState,
        va: u64,
    ) -> S1TTWParams {
        let varange = self.aarch64_get_varange(va);

        // match regime {
        //     Regime::EL3 =>
        // }

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

        todo!()
    }
}
