use crate::cpu::{
    Cpu, Feature,
    common::*,
    exeception::FullAddress,
    fault::{AccessDescriptor, AccessType, ErrorState, Fault, FaultRecord},
    regs::{HCR_EL2Field, RegisterField, SCTLRField},
    shared::Regime,
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
