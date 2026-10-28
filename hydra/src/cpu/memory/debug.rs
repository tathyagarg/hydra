use crate::cpu::{
    Cpu, Feature,
    common::*,
    constraint::{Constraint, Unpredictable},
    fault::{AccessDescriptor, AccessType, FaultRecord, WatchpointInfo},
    memory::AddressSize,
    regs::{
        DBGBCR_N_EL1Field, DBGWCR_N_EL1Field, EDSCR2Field, MDCR_EL2Field, MDCR_EL3Field,
        MDSCR_EL1Field, RegisterField, RegisterFieldRange, VTCR_EL2Field,
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

    pub fn aarch64_reserved_breakpoint_type(&self, n: usize, bt_in: u8) -> (Constraint, u8) {
        let bt = bt_in;
        let mut reserved = false;
        let context_aware = self.is_context_aware_breakpoint(n);

        if !nth_bit(bt, 4) {
            if !(bt == 0b00000 || bt == 0b00001 || bt == 0b00100 || bt == 0b00101) && !context_aware
            {
                reserved = true;
            }

            if !nth_bit(bt, 5)
                && nth_bit(bt, 4)
                && !Cpu::is_feature_implemented(Feature::VHE)
                && !Cpu::is_feature_implemented(Feature::Debugv8p2)
            {
                reserved = true;
            }

            if (bt == 0b00100 || bt == 0b00101)
                && !Cpu::is_feature_implemented(Feature::BWE)
                && !self.have_aarch32_el(EL1)
            {
                reserved = true;
            }
        } else {
            if !(bt == 0b10000 || bt == 0b10001 || bt == 0b10100 || bt == 0b10101) {
                reserved = true;
            }
        }

        if reserved {
            let (c, bt) = Unpredictable::RESBPTYPE.constrain_bits();
            assert!(matches!(c, Constraint::DISABLED | Constraint::UNKNOWN));

            if c == Constraint::DISABLED {
                return (c, 0);
            }
        }

        (Constraint::NONE, bt)
    }

    pub fn debug_addr_top(&self) -> AddressSize {
        if Cpu::is_feature_implemented(Feature::LVA3) {
            55
        } else if Cpu::is_feature_implemented(Feature::LVA) {
            52
        } else {
            48
        }
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

        let mut dbgtype = if Cpu::is_feature_implemented(Feature::ABLE) {
            ((DBGBCR_N_EL1Field::BT2.fetch_bit(self.dbgbcr_el1[n]) as u8) << 1)
                | (DBGBCR_N_EL1Field::BT.fetch_bit(self.dbgbcr_el1[n]) as u8)
        } else {
            DBGBCR_N_EL1Field::BT.fetch_bit(self.dbgbcr_el1[n]) as u8
        };

        (c, dbgtype) = self.aarch64_reserved_breakpoint_type(n, dbgtype);
        if c == Constraint::DISABLED {
            return (BreakpointType::Inactive, false);
        }

        let match_addr = matches_pattern(dbgtype, 0b00000);
        let mismatch = matches_pattern(dbgtype, 0b00100);
        let match_vmid = matches_pattern(dbgtype, 0b01000);
        let match_cid = matches_pattern(dbgtype, 0b00010);
        let match_cid1 = matches_pattern(dbgtype, 0b01010) || matches_pattern(dbgtype, 0b00110);
        let match_cid2 = matches_pattern(dbgtype, 0b01100);
        let linking_enabled = matches_pattern(dbgtype, 0b00011)
            || matches_pattern(dbgtype, 0b01001)
            || matches_pattern(dbgtype, 0b10000);

        if linked_to && !linking_enabled {
            return (BreakpointType::Inactive, false);
        }

        if !linked_to && linking_enabled {
            return (BreakpointType::Inactive, false);
        }

        let linked = matches_pattern(dbgtype, 0b00001);
        if from_linking_enabled {
            assert!(linked_to && isbreakpnt);
            if linked {
                return (BreakpointType::Inactive, false);
            }
        }

        if linked_to && match_addr && isbreakpnt {
            if !Unpredictable::BPLINKEDADDRMATCH.constrain_bool() {
                return (BreakpointType::Inactive, false);
            }
        }

        if mismatch && self.using_aarch32() {
            return (BreakpointType::Inactive, false);
        }

        let mut bvr_match = false;
        let mut bxvr_match = false;
        let mut bp_type: BreakpointType;
        let mut mask: u8;

        if Cpu::is_feature_implemented(Feature::BWE) {
            mask = DBGBCR_N_EL1Field::MASK.fetch_range(self.dbgbcr_el1[n]);

            if mask == 1 || mask == 2 {
                let mut unpred_mask;
                (c, unpred_mask) = Unpredictable::RESBPMASK.constrain_integer(3, 31);
                assert!(matches!(
                    c,
                    Constraint::DISABLED | Constraint::UNKNOWN | Constraint::NONE
                ));

                match c {
                    Constraint::DISABLED => {
                        return (BreakpointType::Inactive, false);
                    }
                    Constraint::NONE => mask = 0,
                    _ => mask = unpred_mask,
                }
            }

            if mask != 0 {
                if (match_cid || match_cid1 || match_cid2)
                    || (DBGBCR_N_EL1Field::BAS.fetch_range(self.dbgbcr_el1[n]) != 0b1111
                        && self.have_aarch32())
                {
                    if !Unpredictable::BPMASK.constrain_bool() {
                        return (BreakpointType::Inactive, false);
                    }
                }
            } else {
                if mismatch {
                    return (BreakpointType::Inactive, false);
                }
            }
        } else {
            mask = 0;
        }

        if match_addr {
            let byte = get_bits(vaddress, 1, 0) as u8;
            let byte_select_match = if self.have_aarch32() {
                assert!(byte == 0 || byte == 2);
                nth_bit(
                    DBGBCR_N_EL1Field::BAS.fetch_range(self.dbgbcr_el1[n]),
                    byte as usize,
                )
            } else {
                assert!(byte == 0);
                true
            };

            let dbgtop = self.debug_addr_top();
            let unpredictable_ress = dbgtop < 55
                && !is_ones(
                    get_bits(self.dbgbvr_el1[n], 63, dbgtop as usize),
                    63 - dbgtop as u32,
                )
                && !is_zeros(
                    get_bits(self.dbgbvr_el1[n], 63, dbgtop as usize),
                    63 - dbgtop as u32,
                )
                && Unpredictable::DBGxVR_RESS.constrain_bool();

            let cmpmsb = if unpredictable_ress { 63 } else { dbgtop };
            let cmplsb = if mask > 2 { mask } else { 2 };

            bvr_match = get_bits(vaddress, cmpmsb as usize, cmplsb as usize)
                == get_bits(self.dbgbvr_el1[n], cmpmsb as usize, cmplsb as usize)
                && byte_select_match;

            if mask > 2 {
                let masktop = mask - 1;
                if bvr_match
                    && !is_zeros(
                        get_bits(self.dbgbvr_el1[n], masktop as usize, 2),
                        masktop as u32 - 2,
                    )
                {
                    bvr_match = Unpredictable::BPMASKEDBITS.constrain_bool();
                }
            }
        } else if match_cid {
            if self.is_in_host() {
                bvr_match =
                    get_bits(self.contextidr_el2, 31, 0) == get_bits(self.dbgbvr_el1[n], 31, 0);
            } else {
                bvr_match = (self.pstate.EL() == EL0 || self.pstate.EL() == EL1)
                    && get_bits(self.contextidr_el1, 31, 0) == get_bits(self.dbgbvr_el1[n], 31, 0);
            }
        } else if match_cid1 {
            bvr_match = (self.pstate.EL() == EL0 || self.pstate.EL() == EL1)
                && !self.is_in_host()
                && get_bits(self.contextidr_el1, 31, 0) == get_bits(self.dbgbvr_el1[n], 31, 0);
        }

        if match_vmid {
            let vmid = self.vmid();
            let bvr_vmid = if !Cpu::is_feature_implemented(Feature::VMID16)
                || !VTCR_EL2Field::VS.fetch_bit(self.vtcr_el2)
            {
                get_bits(self.dbgbvr_el1[n], 39, 32)
            } else {
                get_bits(self.dbgbvr_el1[n], 47, 32)
            } as BitsNumVMIDBits;

            bxvr_match = (self.pstate.EL() == EL0 || self.pstate.EL() == EL1)
                && self.el2_enabled()
                && !self.is_in_host()
                && vmid == bvr_vmid;
        } else if match_cid2 {
            bxvr_match = (self.pstate.EL() != EL3)
                && self.el2_enabled()
                && get_bits(self.dbgbvr_el1[n], 63, 32) == get_bits(self.contextidr_el2, 31, 0);
        }

        let bvr_match_valid = match_addr || match_cid || match_cid1;
        let bxvr_match_valid = match_vmid || match_cid2;

        let mut value_match = (!bxvr_match_valid || bxvr_match) && (!bvr_match_valid || bvr_match);

        if linked_to && linked {
            assert!(!isbreakpnt && match_addr && Cpu::is_feature_implemented(Feature::ABLE));
            let lbnx = if Cpu::is_feature_implemented(Feature::Debugv8p9) {
                DBGBCR_N_EL1Field::LBNX.fetch_range(self.dbgbcr_el1[n])
            } else {
                0b00
            };
            let linked_linked_n =
                (lbnx << 4) | DBGBCR_N_EL1Field::LBN.fetch_range(self.dbgbcr_el1[n]);
            let linked_vaddress = 0; // ARBITRARY
            let linked_linked_to = true;
            let linked_isbreakpnt = true;
            let linked_from_linking_enabled = true;

            let (bp_type, linked_value_match) = self.aarch64_breakpoint_value_match(
                linked_linked_n as usize,
                linked_vaddress,
                linked_linked_to,
                linked_isbreakpnt,
                linked_from_linking_enabled,
            );

            value_match = value_match && linked_value_match;
        }

        if match_addr && !mismatch {
            bp_type = BreakpointType::AddrMatch;
        } else if match_addr && mismatch {
            bp_type = BreakpointType::AddrMismatch;
        } else if match_vmid || match_cid || match_cid1 || match_cid2 {
            bp_type = BreakpointType::CtxtMatch;
        } else {
            unreachable!();
        }

        (bp_type, value_match)
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
