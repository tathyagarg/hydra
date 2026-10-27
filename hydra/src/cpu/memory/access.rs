use crate::cpu::{
    Cpu, Feature,
    common::*,
    fault::{AccessDescriptor, AccessType, Fault},
    memory::{
        bits::Bits,
        types::{
            AddressDescriptor, PhysMemRetStatus, Size, ValidMemReadSize, ValidMemReadSizeBits,
        },
    },
    shared::Faultable,
};

impl Cpu {
    /// S is the number of bytes to read, not number of bits.
    #[allow(unused_variables)]
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

        if memaddrdesc.is_fault() {
            todo!("Abort");
        }

        if memstatus.is_fault() {
            todo!("Abort")
        }

        value
    }

    #[allow(unused_variables, unused_mut)]
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

    #[allow(unused_variables)]
    pub fn phys_mem_write<const N: usize>(
        &self,
        desc: &AddressDescriptor,
        accdesc: &AccessDescriptor,
        value: Bits<N>,
    ) -> PhysMemRetStatus
    where
        Size<N>: ValidMemReadSizeBits,
    {
        todo!("Write")
    }

    #[allow(unused_variables)]
    pub fn aarch64_access_is_tag_checked(&self, vaddr: u64, accdesc: &AccessDescriptor) -> bool {
        if self.using_aarch32() {
            return false;
        }

        // TODO: More checks and stuff

        true
    }
}
