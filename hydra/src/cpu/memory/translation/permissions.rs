use crate::cpu::{
    Cpu, Feature,
    common::*,
    constraint::{Constraint, Unpredictable},
    exeception::PASpace,
    fault::{AccessDescriptor, AccessType, CacheOp, CacheType, Fault, FaultRecord},
    memory::{
        bits::Bits,
        types::{
            DeviceType, MemTagType, MemType, MemoryAttributes, Permissions, S1AccessControls,
            S1TTWParams, TTWState,
        },
    },
    regs::{
        E2H_TCR2_EL2Field, RegisterField, S1PORType, SCR_EL3Field, TCR_EL3Field, TCR2_EL1Field,
        TCR2_EL2Field,
    },
    shared::{EL0, EL3, Regime, SecurityState},
};

impl Cpu {
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

    pub fn aarch64_setting_access_flag_permitted(
        &self,
        fault: &FaultRecord,
        fault_perm: &FaultRecord,
    ) -> bool {
        if fault.statuscode == Fault::None && fault_perm.statuscode == Fault::None {
            true
        } else if fault.statuscode == Fault::Alignment || fault_perm.statuscode == Fault::Permission
        {
            Unpredictable::AFUPDATE.constrain_bool()
        } else {
            false
        }
    }

    pub fn aarch64_set_access_flag(
        &self,
        ha: bool,
        accdesc: &AccessDescriptor,
        fault: &FaultRecord,
        fault_perm: &FaultRecord,
    ) -> bool {
        if !ha || !self.aarch64_setting_access_flag_permitted(fault, fault_perm) {
            false
        } else if accdesc.acctype == AccessType::AT {
            todo!("AT updates AF");
        } else if matches!(accdesc.acctype, AccessType::DC | AccessType::IC) {
            todo!("Generate access flag fault on IC/DC operations");
        } else {
            true
        }
    }

    pub fn aarch64_setting_dirty_state_permitted(
        &self,
        fault: &FaultRecord,
        fault_perm: &FaultRecord,
    ) -> bool {
        if fault_perm.statuscode != Fault::None {
            false
        } else if fault.statuscode == Fault::None {
            true
        } else if fault.statuscode == Fault::Alignment {
            Unpredictable::DBUPDATE.constrain_bool()
        } else {
            false
        }
    }

    pub fn aarch64_set_dirty_state(
        &self,
        hd: bool,
        dbm: bool,
        accdesc: &AccessDescriptor,
        fault: &FaultRecord,
        fault_perm: &FaultRecord,
    ) -> bool {
        if !hd {
            false
        } else if !self.aarch64_setting_dirty_state_permitted(fault, fault_perm) {
            false
        } else if matches!(
            accdesc.acctype,
            AccessType::AT | AccessType::IC | AccessType::DC
        ) {
            false
        } else if !accdesc.write {
            false
        } else {
            dbm
        }
    }
}
