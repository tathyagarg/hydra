use crate::cpu::{
    Cpu, Feature,
    common::*,
    constraint::{Constraint, Unpredictable},
    fault::{AccessDescriptor, AccessType, FaultRecord, WatchpointInfo},
    regs::{
        DBGBCR_N_EL1Field, DBGWCR_N_EL1Field, EDSCR2Field, MDCR_EL2Field, MDCR_EL3Field,
        MDSCR_EL1Field, RegisterField, RegisterFieldRange,
    },
    shared::{BreakpointType, EL0, EL1, EL2, EL3, SecurityState},
};

impl Cpu {
    pub fn self_hosted_extended_bpwp_enabled(&self) -> bool {
        if self.num_breakpoints_implemented() <= 16 && self.num_watchpoints_implemented() <= 16 {
            return false;
        }

        if (self.have_el(EL3) && !MDCR_EL3Field::EBWE.fetch_bit(self.mdcr_el3))
            || (self.el2_enabled() && !MDCR_EL2Field::EBWE.fetch_bit(self.mdcr_el2))
        {
            return false;
        }

        MDSCR_EL1Field::EMBWE.fetch_bit(self.mdscr_el1)
    }

    pub fn is_watchpoint_enabled(&self, n: usize) -> bool {
        if n > 15
            && ((!self.halt_on_breakpoint_or_watchpoint()
                || !self.self_hosted_extended_bpwp_enabled())
                || (self.halt_on_breakpoint_or_watchpoint()
                    && !EDSCR2Field::EHBWE.fetch_bit(self.edscr2)))
        {
            return false;
        }

        DBGWCR_N_EL1Field::E.fetch_bit(self.dbgwcr_el1[n])
    }

    pub const fn num_address_linking_breakpoints_implemented(&self) -> usize {
        if !Cpu::is_feature_implemented(Feature::ABLE) {
            0
        } else {
            NUM_ADDRESS_LINKING_BREAKPOINTS
        }
    }

    pub fn is_address_linking_breakpoint(&self, n: usize) -> bool {
        n < self.num_address_linking_breakpoints_implemented()
    }

    pub fn is_breakpoint_enabled(&self, n: usize) -> bool {
        if n > 15
            && ((!self.halt_on_breakpoint_or_watchpoint()
                || !self.self_hosted_extended_bpwp_enabled())
                || (self.halt_on_breakpoint_or_watchpoint()
                    && !EDSCR2Field::EHBWE.fetch_bit(self.edscr2)))
        {
            return false;
        }

        DBGBCR_N_EL1Field::E.fetch_bit(self.dbgbcr_el1[n])
    }

    #[allow(unused_variables, unused_mut)]
    pub fn aarch64_breakpoint_value_match(
        &self,
        n_in: usize,
        vaddress: u64,
        linked_to: bool,
        isbreakpnt: bool,
        from_linking_enabled: bool,
    ) -> (BreakpointType, bool) {
        let mut c: Constraint;
        let mut n = n_in;

        if n >= self.num_breakpoints_implemented() {
            (c, n) = Unpredictable::BPNOTIMPL
                .constrain_integer(0, self.num_breakpoints_implemented() - 1);

            assert!(matches!(c, Constraint::DISABLED | Constraint::UNKNOWN));
            if c == Constraint::DISABLED {
                return (BreakpointType::Inactive, false);
            }
        }

        if !self.is_breakpoint_enabled(n) {
            return (BreakpointType::Inactive, false);
        }

        let dbgtype = if Cpu::is_feature_implemented(Feature::ABLE) {
            ((DBGBCR_N_EL1Field::BT2.fetch_bit(self.dbgbcr_el1[n]) as u8) << 1)
                | (DBGBCR_N_EL1Field::BT.fetch_bit(self.dbgbcr_el1[n]) as u8)
        } else {
            DBGBCR_N_EL1Field::BT.fetch_bit(self.dbgbcr_el1[n]) as u8
        };

        todo!()
    }

    #[allow(unused_variables, unused_mut)]
    pub fn aarch64_state_match(
        &self,
        ssc_in: u8,
        ssce_in: bool,
        hmc_in: bool,
        pxc_in: u8,
        linked_in: bool,
        linked_n_in: u8,
        isbreakpnt: bool,
        vaddress: u64,
        accdesc: &AccessDescriptor,
    ) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            assert!(!ssce_in);
        }

        let mut ssc = ssc_in;
        let mut ssce = ssce_in;
        let mut hmc = hmc_in;
        let mut pxc = pxc_in;

        let linked = linked_in;
        let linked_n = linked_n_in;
        let mut c: Constraint;

        (c, ssc, ssce, hmc, pxc) = self.check_valid_state_match(ssc, ssce, hmc, pxc, isbreakpnt);
        if c == Constraint::DISABLED {
            return false;
        }

        let el3_match = self.have_el(EL3) && hmc && !nth_bit(ssc, 0);
        let el2_match =
            self.have_el(EL2) && ((hmc && !(ssc == 0b10 && pxc == 0b00)) || ssc == 0b11);
        let el1_match = nth_bit(pxc, 0);
        let el0_match = nth_bit(pxc, 1);

        let priv_match = match accdesc.el {
            EL3 => el3_match,
            EL2 => el2_match,
            EL1 => el1_match,
            EL0 => el0_match,
            _ => unreachable!(),
        };

        let ss_match = match (ssce, ssc) {
            (false, 0b00) => hmc || accdesc.ss != SecurityState::Root,
            (false, 0b01) => accdesc.ss == SecurityState::NonSecure,
            (false, 0b10) => {
                (hmc && accdesc.ss == SecurityState::Root) || accdesc.ss == SecurityState::Secure
            }
            (false, 0b11) => {
                (hmc && accdesc.ss != SecurityState::Root) || accdesc.ss == SecurityState::Secure
            }
            (true, 0b01) => accdesc.ss == SecurityState::Realm,
            _ => unreachable!(),
        };

        let mut linked_match = false;

        if linked {
            if !self.is_context_aware_breakpoint(linked_n as usize)
                && (isbreakpnt || self.is_address_linking_breakpoint(linked_n as usize))
            {
                return Unpredictable::BPNOTLINKING.constrain_bool();
            }
        }

        if linked {
            let mut bp_type: BreakpointType;
        }

        todo!()
    }

    #[allow(unused_variables, unused_mut)]
    pub fn aarch64_watchpoint_match(
        &self,
        n: usize,
        vaddress: u64,
        size: u64,
        accdesc: &AccessDescriptor,
    ) -> WatchpointInfo {
        assert!(!self.el_using_aarch32(self.s1translationregime_raw()));
        assert!(n < self.num_watchpoints_implemented());

        let enabled = self.is_watchpoint_enabled(n);
        let linked = DBGWCR_N_EL1Field::WT.fetch_bit(self.dbgwcr_el1[n]);
        let isbreakpoint = false;
        let lbnx = if Cpu::is_feature_implemented(Feature::Debugv8p9) {
            DBGWCR_N_EL1Field::LBNX.fetch_range(self.dbgwcr_el1[n])
        } else {
            0b00
        };
        let linked_n = (lbnx << 4) | DBGWCR_N_EL1Field::LBN.fetch_range(self.dbgwcr_el1[n]);
        let ssce = if Cpu::is_feature_implemented(Feature::RME) {
            DBGWCR_N_EL1Field::SSCE.fetch_bit(self.dbgwcr_el1[n])
        } else {
            false
        };
        let mismatch = Cpu::is_feature_implemented(Feature::BWE2)
            && DBGWCR_N_EL1Field::WT2.fetch_bit(self.dbgwcr_el1[n]);
        // let state_match = self.aarch64_state_match(ssc_in, ssce_in, hmc_in, pxc_in, linked_in, linked_n_in, isbreakpnt, vaddress, accdesc)

        todo!()
    }

    #[allow(unused_variables, unused_mut, unused_assignments)]
    pub fn aarch64_check_watchpoint(
        &self,
        fault_in: &FaultRecord,
        vaddress_in: u64,
        accdesc: &AccessDescriptor,
        size_in: u64,
    ) -> MDSCR_EL1Field {
        assert!(!self.el_using_aarch32(self.s1translationregime_raw()));

        let mut fault = fault_in.clone();
        let mut fault_match = fault_in.clone();
        let mut fault_mismatch = fault_in.clone();

        let mut vaddress = vaddress_in;
        let mut size = size_in;

        let mut rounded_match = false;

        let original_vaddress = vaddress_in;
        let original_size = size_in;

        let mut addr_match_wp = false;
        let mut addr_mismatch_wp = false;
        let mut addr_match = false;
        let mut addr_mismatch = true;

        if self.is_relaxed_watchpoint_access(accdesc) {
            let mut upper_vaddress = original_vaddress + original_size;
            if Unpredictable::_16BYTEROUNDEDDOWNACCESS.constrain_bool() {
                vaddress = align_down_size(vaddress, 16);
                rounded_match = true;
            }

            if Unpredictable::_16BYTEROUNDEDUPACCESS.constrain_bool() {
                upper_vaddress = align_up_size(upper_vaddress, 16);
                rounded_match = true;
            }

            size = upper_vaddress - vaddress;
        }

        for i in 0..self.num_watchpoints_implemented() {
            // let watchptinfo = self.aarch64_watch
        }

        todo!()
    }

    #[allow(unused_variables, unused_mut, unused_assignments, unreachable_code)]
    pub fn aarch64_check_debug(
        &self,
        vaddress: u64,
        accdesc: &AccessDescriptor,
        size: usize,
    ) -> FaultRecord {
        let mut fault = FaultRecord::no_fault_accdecs_va(accdesc, vaddress);

        let d_side = self.is_watchpointable_access(accdesc);
        let i_side = accdesc.acctype == AccessType::IFETCH;

        let generate_exceptions = if accdesc.acctype == AccessType::NV2 {
            let mask = false;
            let ss = self.current_security_state();
            self.aarch64_generate_debug_exceptions_from(EL2, ss, mask)
        } else {
            self.aarch64_generate_debug_exceptions()
        } && MDSCR_EL1Field::MDE.fetch_bit(self.mdscr_el1);

        let halt = self.halt_on_breakpoint_or_watchpoint();

        if generate_exceptions || halt {
            fault = if d_side {
                // self.aarch64_check_wa
                todo!()
            } else {
                todo!()
            };
        }

        todo!()
    }
}
