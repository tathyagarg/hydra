use crate::cpu::{
    Cpu, Feature,
    constraint::Unpredictable,
    exeception::PASpace,
    fault::{AccessDescriptor, AccessType, Fault, FaultRecord, VARange},
    memory::{
        bits::Bits,
        types::{
            AddressDescriptor, DEFAULT_MECID, MemAttr_NC, MemTagType, MemType, MemoryAttributes,
            S1TTWParams, TTWState,
        },
    },
    regs::{
        HCR_EL2Field, MECID_A0_EL2Field, MECID_A1_EL2Field, MECID_P0_EL2Field, MECID_P1_EL2Field,
        MECID_RL_A_EL3Field, RegisterField, RegisterFieldRange, VMECID_P_EL2Field,
    },
    shared::{EL0, Faultable, Regime, SecurityState},
};

impl Cpu {
    pub fn aarch64_full_translate(
        &self,
        va: u64,
        size: usize,
        accdesc: &AccessDescriptor,
        aligned: bool,
    ) -> AddressDescriptor {
        let regime = self.translation_regime(accdesc.el);
        let fault = FaultRecord::no_fault_accdecs_va(accdesc, va);
        let (fault, ipa) = self.aarch64_s1translate(&fault, regime, va, size, aligned, accdesc);

        if fault.statuscode != Fault::None {
            return AddressDescriptor::create_faulty(&fault);
        }

        if accdesc.ss == SecurityState::Realm {
            assert!(self.el2_enabled());
        }

        if regime == Regime::EL10 && self.el2_enabled() {
            todo!("S2 Translation");
        } else {
            return ipa;
        }
    }

    pub fn aarch64_translate_address(
        &self,
        va: u64,
        accdesc: &AccessDescriptor,
        aligned: bool,
        size: usize,
    ) -> AddressDescriptor {
        let mut result = self.aarch64_full_translate(va, size, accdesc, aligned);

        if !result.is_fault() && accdesc.acctype != AccessType::IFETCH {
            result.fault = self.aarch64_check_debug(va, accdesc, size);
        }

        todo!()
    }

    pub fn aarch64_s1treat_as_normal_nc(
        &self,
        walkstate: &TTWState,
        regime: Regime,
        accdesc: &AccessDescriptor,
    ) -> bool {
        (accdesc.acctype == AccessType::IFETCH
            && (walkstate.memattrs.memtype == MemType::Device
                || self.aarch64_s1icache_enabled(regime)))
            || (accdesc.acctype != AccessType::IFETCH
                && !self.aarch64_s1dcache_enabled(regime)
                && walkstate.memattrs.memtype == MemType::Normal)
    }

    pub fn is_pmte_selected(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::MTE2) {
            return false;
        }

        true
    }

    pub fn aarch64_s1output_mecid<const N: usize>(
        &self,
        walkparams: &S1TTWParams,
        regime: Regime,
        varange: VARange,
        paspace: PASpace,
        desc: &Bits<N>,
    ) -> u16 {
        if !walkparams.emec {
            return DEFAULT_MECID;
        }

        if paspace != PASpace::Realm {
            return DEFAULT_MECID;
        }

        if regime != Regime::EL3 && !self.effective_scr_el3_mecen() {
            return DEFAULT_MECID;
        }

        let descriptor_amec = if walkparams.d128 {
            desc.nth_bit(108)
        } else {
            desc.nth_bit(63)
        };

        match regime {
            Regime::EL3 => MECID_RL_A_EL3Field::MECID.fetch_range(self.mecid_rl_a_el3),
            Regime::EL2 => {
                if !descriptor_amec {
                    MECID_P0_EL2Field::MECID.fetch_range(self.mecid_p0_el2)
                } else {
                    MECID_A0_EL2Field::MECID.fetch_range(self.mecid_a0_el2)
                }
            }
            Regime::EL20 => {
                if varange == VARange::LOWER {
                    if !descriptor_amec {
                        MECID_P0_EL2Field::MECID.fetch_range(self.mecid_p0_el2)
                    } else {
                        MECID_A0_EL2Field::MECID.fetch_range(self.mecid_a0_el2)
                    }
                } else {
                    if !descriptor_amec {
                        MECID_P1_EL2Field::MECID.fetch_range(self.mecid_p1_el2)
                    } else {
                        MECID_A1_EL2Field::MECID.fetch_range(self.mecid_a1_el2)
                    }
                }
            }
            Regime::EL10 => VMECID_P_EL2Field::MECID.fetch_range(self.vmecid_p_el2),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1translate(
        &self,
        fault_in: &FaultRecord,
        regime: Regime,
        va: u64,
        size: usize,
        aligned: bool,
        accdesc: &AccessDescriptor,
    ) -> (FaultRecord, AddressDescriptor) {
        let mut fault = fault_in.clone();
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
            (da, w, dsc)
        } else {
            let (f, da, w, dsc) =
                self.aarch64_s1walk::<64>(fault, &walkparams, va, regime, accdesc);
            fault = f;
            (da, w, dsc.extend_to())
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

        let fault_perm =
            self.aarch64_s1check_permissions(&fault, regime, &walkstate, &walkparams, accdesc);

        let mem_desc: Bits<128>;
        let mut new_desc = descriptor.clone();

        if self.aarch64_set_access_flag(walkparams.ha, accdesc, &fault, &fault_perm) {
            new_desc.set_bit(10, true);
        }

        if self.aarch64_set_dirty_state(
            walkparams.hd,
            walkparams.pie || descriptor.nth_bit(51),
            accdesc,
            &fault,
            &fault_perm,
        ) {
            new_desc.set_bit(7, false);
        }

        if fault.statuscode == Fault::None && fault_perm.statuscode != Fault::None {
            fault = fault_perm;
        }

        if new_desc != descriptor {
            if !self.is_wb_shareable(&descipaddr.memattrs)
                && Unpredictable::Unsupported_Atomic_HW_Update.constrain_bool()
            {
                fault.statuscode = Fault::UnsupportedAtomicHWUpdate;
                return (fault, AddressDescriptor::arbitrary());
            }

            let descaccess = self.create_accdesc_tte_update(accdesc);
            let descpaddr = if regime == Regime::EL10 && self.el2_enabled() {
                todo!("S2 translate")
            } else {
                descipaddr
            };

            (fault, mem_desc) = if walkparams.d128 {
                self.aarch64_mem_swap_table_desc::<128>(
                    &fault,
                    &descriptor,
                    &new_desc,
                    walkparams.ee,
                    &descaccess,
                    &descpaddr,
                )
            } else {
                let (f, m) = self.aarch64_mem_swap_table_desc::<64>(
                    &fault,
                    &descriptor.clone().shrink_to(),
                    &new_desc.clone().shrink_to(),
                    walkparams.ee,
                    &descaccess,
                    &descpaddr,
                );

                (f, m.extend_to())
            };

            if fault.statuscode != Fault::None {
                if accdesc.acctype == AccessType::AT {
                    // TODO: Impdef
                    fault.statuscode = Fault::None;
                } else {
                    return (fault, AddressDescriptor::arbitrary());
                }
            } else if new_desc != descriptor && mem_desc != new_desc {
                return self.aarch64_s1translate(fault_in, regime, va, size, aligned, accdesc);
            }
        }

        if fault.statuscode != Fault::None {
            return (fault, AddressDescriptor::arbitrary());
        }

        let oa = self.stage1oa(va, &walkparams, &walkstate);
        let mut memattrs: MemoryAttributes;

        if self.aarch64_s1treat_as_normal_nc(&walkstate, regime, accdesc) {
            memattrs = self.normal_nc_memattr();
            memattrs.xs = walkstate.memattrs.xs;

            if self.is_pmte_selected()
                && walkstate.memattrs.tags == MemTagType::AllocationTagged
                && Unpredictable::S1CTAGGED.constrain_bool()
            {
                memattrs.tags = MemTagType::AllocationTagged;
            } else if Cpu::is_feature_implemented(Feature::MTE_CANONICAL_TAGS)
                && walkstate.memattrs.tags == MemTagType::CanonicallyTagged
            {
                memattrs.tags = MemTagType::CanonicallyTagged;
            }
        } else {
            memattrs = walkstate.memattrs.clone();
        }

        memattrs.shareability = if regime == Regime::EL10
            && self.el2_enabled()
            && HCR_EL2Field::VM.fetch_bit(self.hcr_el2)
        {
            // TODO: Impdef
            walkstate.memattrs.shareability
        } else {
            self.effective_shareability(&memattrs)
        };

        let mut ipa = AddressDescriptor::create(va, oa, memattrs.clone(), accdesc);
        ipa.s1assured = walkstate.s1assured;

        let varange = self.aarch64_get_varange(va);
        ipa.mecid = self.aarch64_s1output_mecid(
            &walkparams,
            regime,
            varange,
            ipa.paddress.paspace,
            &descriptor,
        );

        if accdesc.atomicop
            && !self.is_wb_shareable(&memattrs)
            && Unpredictable::Atomic_MMU_IMPDEF_FAULT.constrain_bool()
        {
            fault.statuscode = Fault::Exclusive;
            return (fault, ipa);
        }

        if accdesc.ls64 && memattrs.memtype == MemType::Normal {
            if Cpu::is_feature_implemented(Feature::LS64WB) && !accdesc.withstatus {
                if !self.is_wb_shareable(&memattrs)
                    && !(memattrs.inner.attrs == MemAttr_NC && memattrs.outer.attrs == MemAttr_NC)
                {
                    // TODO: Impdef
                    fault.statuscode = Fault::Exclusive;
                    return (fault, ipa);
                }
            } else if !(memattrs.inner.attrs == MemAttr_NC && memattrs.outer.attrs == MemAttr_NC) {
                fault.statuscode = Fault::Exclusive;
                return (fault, ipa);
            }
        }

        (fault, ipa)
    }
}
