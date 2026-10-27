use crate::cpu::{
    Cpu, Feature,
    common::*,
    constraint::Unpredictable,
    exeception::{FullAddress, PASpace},
    fault::{AccessDescriptor, AccessType, Fault, FaultRecord, GPCF, VARange},
    memory::{
        bits::Bits,
        types::{
            AddressDescriptor, AddressSize, DescriptorType, Permissions, S1TTWParams, SDFType,
            Size, TGx, TTWState, ValidMemReadSizeBits, ValidWalkSize,
        },
    },
    regs::{HCR_EL2Field, RegisterField, VTCR_EL2Field},
    shared::{Faultable, Regime, SecurityState},
};

impl Cpu {
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
        if memstatus.is_fault() {
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

    #[allow(unused_variables, unused_mut, unused_assignments)]
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

    pub fn aarch64_mem_swap_table_desc<const S: usize>(
        &self,
        fault_in: &FaultRecord,
        prev_desc: &Bits<S>,
        new_desc: &Bits<S>,
        ee: bool,
        descaccess: &AccessDescriptor,
        descpaddr: &AddressDescriptor,
    ) -> (FaultRecord, Bits<S>)
    where
        Size<S>: ValidWalkSize + ValidMemReadSizeBits,
    {
        assert!(descaccess.acctype == AccessType::TTW);
        let mut fault = fault_in.clone();

        if Cpu::is_feature_implemented(Feature::RME) {
            fault.gpcf = self.granule_protection_check(descpaddr, descaccess);
            if fault.gpcf.gpf != GPCF::None {
                fault.statuscode = Fault::GPCFOnWalk;
                fault.paddress = descpaddr.paddress;
                fault.gpcfs2walk = fault.secondstage;
                return (fault, Bits::zero());
            }
        }

        let (mut memstatus, mut mem_desc) = self.phys_mem_read::<S>(descpaddr, descaccess);

        if ee {
            mem_desc = mem_desc.reverse();
        }

        if memstatus.is_fault() {
            fault = self.handle_external_ttw_abort(&memstatus, &descpaddr, &fault);
            if fault.statuscode.is_fault() {
                return (fault, Bits::zero());
            }
        }

        if mem_desc == *prev_desc {
            let ordered_new_desc = if ee {
                new_desc.reverse()
            } else {
                new_desc.clone()
            };
            memstatus = self.phys_mem_write(descpaddr, descaccess, ordered_new_desc);

            if memstatus.is_fault() {
                fault = self.handle_external_ttw_abort(&memstatus, &descpaddr, &fault);

                if fault.statuscode.is_fault() {
                    return (fault, Bits::zero());
                }
            }

            mem_desc = new_desc.clone();
        }

        (fault, mem_desc)
    }
}
