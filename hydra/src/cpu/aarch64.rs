use crate::cpu::{
    BranchType, Cpu, Feature,
    brbe::{BRBSRC_EL1_Type, BRBTGT_EL1_Type},
    common::*,
    exeception::{Exception, ExceptionRecord},
    regs::{
        BRBCR_EL1Field, BRBCR_EL2Field, BRBFCR_EL0Field, BRBFCR_EL1Field, HCR_EL2Field,
        HCRX_EL2Field, MDCR_EL3Field, RegisterField, RegisterFieldRange, SCR_EL3Field, SCTLRField,
    },
    shared::{EL, EL0, EL1, EL2, EL3, SecurityState},
};

impl Cpu {
    /// AArch64_TakeException()
    /// =======================
    /// Take an exception to an Exception level using AArch64.
    #[allow(warnings)]
    pub fn aarch64_take_exception(
        &self,
        target_el: EL,
        exception_in: &ExceptionRecord,
        preferred_exception_return: u64,
        vec_offset_in: isize,
    ) {
        assert!(
            self.have_el(target_el)
                && !self.el_using_aarch32(target_el)
                && target_el >= self.pstate.EL()
        );

        if self.halted() {
            unimplemented!("Take Exception in debug state");
        }

        let except = exception_in;
        let mut sync_errors = false;
        let mut iesb_req = false;

        if Cpu::is_feature_implemented(Feature::IESB) {
            todo!("IESB feature not implemented");
        } else {
            sync_errors = false;
        }

        let mut brbe_source_allowed = false;
        let mut brbe_source_address = 0u64;

        if Cpu::is_feature_implemented(Feature::BRBE) {
            brbe_source_allowed = self.branch_record_allowed(self.pstate.EL());
            brbe_source_address = preferred_exception_return;
        }

        if !Cpu::is_feature_implemented(Feature::ExS)
            || SCTLRField::EIS.fetch_bit(self.get_SCTLR_EL(target_el))
        {
            // does nothing?
            todo!("Synchronize Contexts");
        } else if !(except.exceptype == Exception::SoftwareBreakpoint
            || ([
                Exception::SupervisorCall,
                Exception::HypervisorCall,
                Exception::MonitorCall,
            ]
            .contains(&except.exceptype)
                && !except.trapped_syscall_inst))
        {
            // does nothing?
            todo!("Instruction Fetch Barrier");
        }

        todo!("More bs")
    }

    /// AArch64_Undefined()
    /// ===================
    pub fn aarch64_undefined(&self) {
        let route_to_el2 = self.pstate.EL() == EL0
            && self.el2_enabled()
            && HCR_EL2Field::TGE.fetch_bit(self.hcr_el2);
        let preferred_exception_return = self.this_instr_addr::<64>();
        let vect_offset = 0x0isize;

        let except = ExceptionRecord::syndrome(Exception::Uncategorized);

        if self.pstate.EL() > EL1 {
            self.aarch64_take_exception(
                self.pstate.EL(),
                &except,
                preferred_exception_return,
                vect_offset,
            );
        } else if route_to_el2 {
            self.aarch64_take_exception(EL2, &except, preferred_exception_return, vect_offset);
        } else {
            self.aarch64_take_exception(EL1, &except, preferred_exception_return, vect_offset);
        }
    }

    /// BranchRecordAllowed()
    /// =====================
    /// Returns TRUE if branch recording is allowed, FALSE otherwise.
    pub fn branch_record_allowed(&self, el: EL) -> bool {
        if self.el_using_aarch32(el) {
            return false;
        }

        if BRBFCR_EL0Field::PAUSED.fetch_bit(self.brbfcr_el0) {
            return false;
        }

        if el == EL3 && Cpu::is_feature_implemented(Feature::BRBEv1p1) {
            return MDCR_EL3Field::E3BREC.fetch_bit(self.mdcr_el3)
                != MDCR_EL3Field::E3BREW.fetch_bit(self.mdcr_el3);
        }

        if self.have_el(EL3)
            && (MDCR_EL3Field::SBRBE.fetch_range(self.mdcr_el3) == 0
                || (self.current_security_state() == SecurityState::Secure
                    && MDCR_EL3Field::SBRBE.fetch_range(self.mdcr_el3) == 0b01))
        {
            return false;
        }

        match el {
            EL3 => false,
            EL2 => BRBCR_EL2Field::E2BRE.fetch_bit(self.brbcr_el2),
            EL1 => BRBCR_EL1Field::E1BRE.fetch_bit(self.brbcr_el1),
            EL0 => {
                if self.el2_enabled() && HCR_EL2Field::TGE.fetch_bit(self.hcr_el2) {
                    BRBCR_EL2Field::E0HBRE.fetch_bit(self.brbcr_el2)
                } else {
                    BRBCR_EL1Field::E0BRE.fetch_bit(self.brbcr_el1)
                }
            }
            _ => unreachable!(),
        }
    }

    // FilterBranchRecord()
    // ====================
    // Returns TRUE if the branch record is not filtered out, FALSE otherwise.
    pub fn filter_branch_record(&self, br: &BranchType, cond: bool) -> bool {
        match br {
            BranchType::DIRCALL => {
                BRBFCR_EL1Field::DIRCALL.fetch_bit(self.brbfcr_el1)
                    != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
            }
            BranchType::INDCALL => {
                BRBFCR_EL1Field::INDCALL.fetch_bit(self.brbfcr_el1)
                    != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
            }
            BranchType::RET => {
                BRBFCR_EL1Field::RTN.fetch_bit(self.brbfcr_el1)
                    != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
            }
            BranchType::DIR => {
                if cond {
                    BRBFCR_EL1Field::CONDDIR.fetch_bit(self.brbfcr_el1)
                        != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
                } else {
                    BRBFCR_EL1Field::DIRECT.fetch_bit(self.brbfcr_el1)
                        != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
                }
            }
            BranchType::INDIR => {
                BRBFCR_EL1Field::INDCALL.fetch_bit(self.brbfcr_el1)
                    != BRBFCR_EL1Field::EnI.fetch_bit(self.brbfcr_el1)
            }
            _ => false,
        }
    }

    /// BranchRawCycleCount()
    /// =====================
    /// If the cycle count is known, the return result is the cycle count since the last branch.
    pub fn branch_raw_cycle_count(&self) -> u64 {
        0
    }

    /// BRBCycleCountingEnabled()
    /// =========================
    /// Returns TRUE if the recording of cycle counts is allowed,
    /// FALSE otherwise.
    pub fn brb_cycle_counting_enabled(&self) -> bool {
        if self.have_el(EL2) && !BRBCR_EL2Field::CC.fetch_bit(self.brbcr_el2) {
            return false;
        }

        if !BRBCR_EL1Field::CC.fetch_bit(self.brbcr_el1) {
            return false;
        }

        return true;
    }

    /// FirstBranchAfterProhibited()
    /// ============================
    /// Returns TRUE if branch recorded is the first branch after a prohibited region,
    /// FALSE otherwise.
    pub fn first_branch_after_prohibited(&self) -> bool {
        false
    }

    /// BranchEncCycleCount()
    /// =====================
    /// The first return result is '1' if either of the following is true, and '0' otherwise:
    /// - This is the first Branch record after the PE exited a Prohibited Region.
    /// - This is the first Branch record after cycle counting has been enabled.
    /// If the first return is '0', the second return result is the encoded cycle count
    /// since the last branch.
    /// The format of this field uses a mantissa and exponent to express the cycle count value.
    /// - bits[7:0] indicate the mantissa M.
    /// - bits[13:8] indicate the exponent E.
    /// The cycle count is expressed using the following function:
    /// cycle_count = (if IsZero(E) then UInt(M) else UInt('1'::M::Zeros{UInt(E)-1}))
    /// A value of all ones in both the mantissa and exponent indicates the cycle count value
    /// exceeded the size of the cycle counter.
    /// If the cycle count is not known, the second return result is zero.
    pub fn branch_enc_cycle_count(&self) -> (bool, u16) {
        let mut cc = self.branch_raw_cycle_count();
        if !self.brb_cycle_counting_enabled() || self.first_branch_after_prohibited() {
            return (true, 0);
        }

        let (e, m) = if cc < 2u64.pow(8) {
            (0u64, (cc & 0b1111111) as u64)
        } else if cc >= 2u64.pow(20) {
            (ones(6), ones(8))
        } else {
            let mut tempe = 1u64;
            let mut loopcount = 0;
            while cc >= 2u64.pow(9) && loopcount < 11 {
                tempe += 1;
                cc = cc.div_euclid(2);
                loopcount += 1;
            }

            (tempe, (cc & 0b1111111))
        };

        (false, (e << 6 | m) as u16)
    }

    /// BRBEMispredictAllowed()
    /// =======================
    /// Returns TRUE if the recording of branch misprediction is allowed,
    /// FALSE otherwise.
    pub fn brbe_mispredict_allowed(&self) -> bool {
        if self.have_el(EL2) && !BRBCR_EL2Field::MPRED.fetch_bit(self.brbcr_el2) {
            return false;
        }

        if !BRBCR_EL1Field::MPRED.fetch_bit(self.brbcr_el1) {
            return false;
        }

        true
    }

    /// BranchMispredict()
    /// ==================
    /// Returns TRUE if the branch being executed was mispredicted, FALSE otherwise.
    pub fn branch_mispredict(&self) -> bool {
        false
    }

    /// GetBRBENumRecords()
    /// ===================
    /// Returns the number of branch records implemented.
    pub fn get_brbe_num_records(&self) -> u64 {
        self.imp_def_int("Number of BRBE Branch records")
    }

    /// UpdateBranchRecordBuffer()
    /// ==========================
    /// Add a new Branch record to the buffer.
    pub fn update_branch_record_buffer(
        &mut self,
        ccu: bool,
        cc: u16,
        branch_type: u8,
        el: EL,
        mispredict: bool,
        valid: u8,
        source_address: u64,
        target_address: u64,
    ) {
        let nbrbe_records = self.get_brbe_num_records() as usize;

        for i in (1..nbrbe_records).rev() {
            self.brbe.records_src[i] = self.brbe.records_src[i - 1];
            self.brbe.records_tgt[i] = self.brbe.records_tgt[i - 1];
            self.brbe.records_inf[i] = self.brbe.records_inf[i - 1];
        }

        self.brbe.records_inf[0].ccu = ccu;
        self.brbe.records_inf[0].cc = cc;
        self.brbe.records_inf[0].el = el;
        self.brbe.records_inf[0].valid = valid;
        self.brbe.records_inf[0].mpred = mispredict;
        self.brbe.records_inf[0].type_ = branch_type;

        self.brbe.records_src[0] = BRBSRC_EL1_Type(source_address);
        self.brbe.records_tgt[0] = BRBTGT_EL1_Type(target_address);
    }

    /// BRBEBranch()
    /// ============
    /// Called to write branch record for the following branches when BRBE is active:
    /// direct branches,
    /// indirect branches,
    /// direct branches with link,
    /// indirect branches with link,
    /// returns from subroutines.
    pub fn brbe_branch(&mut self, br_type: BranchType, cond: bool, target_address: u64) {
        if self.branch_record_allowed(self.pstate.EL()) && self.filter_branch_record(&br_type, cond)
        {
            let branch_type: u8 = match br_type {
                BranchType::DIR => {
                    if cond {
                        0b001000
                    } else {
                        0b000000
                    }
                }
                BranchType::INDIR => 0b000001,
                BranchType::DIRCALL => 0b000010,
                BranchType::INDCALL => 0b000011,
                BranchType::RET => 0b000101,
                _ => unreachable!(),
            };

            let (ccu, cc) = self.branch_enc_cycle_count();
            let el = self.pstate.EL();

            let mispredict = self.brbe_mispredict_allowed() && self.branch_mispredict();

            self.update_branch_record_buffer(
                ccu,
                cc,
                branch_type,
                el,
                mispredict,
                0b11,
                self.get_PC64(),
                target_address,
            );

            self.pmu_event1(Cpu::PMU_EVENT_BRB_FILTRATE);
        }
    }

    /// IsTCR2EL2Enabled()
    /// ==================
    /// Returns TRUE if access to TCR2_EL2 register is enabled, and FALSE otherwise.
    /// Indirect read of TCR2_EL2 returns 0 when access is not enabled.
    pub fn is_tcr2el2_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::TCR2) {
            return false;
        }

        if self.have_el(EL3) && !SCR_EL3Field::TCR2En.fetch_bit(self.scr_el3) {
            return false;
        }

        self.el2_enabled()
    }

    /// IsHCRXEL2Enabled()
    /// ==================
    /// Returns TRUE if access to HCRX_EL2 register is enabled, and FALSE otherwise.
    /// Indirect read of HCRX_EL2 returns 0 when access is not enabled.
    pub fn is_hcrxel2_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::HCX) {
            return false;
        }

        if self.el_using_aarch32(EL2) {
            return false;
        }

        if self.have_el(EL3) && !SCR_EL3Field::HXEn.fetch_bit(self.scr_el3) {
            return false;
        }

        self.el2_enabled()
    }

    // IsTCR2EL1Enabled()
    // ==================
    // Returns TRUE if access to TCR2_EL1 register is enabled, and FALSE otherwise.
    // Indirect read of TCR2_EL1 returns 0 when access is not enabled.
    pub fn is_tcr2el1_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::TCR2) {
            return false;
        }

        if self.have_el(EL3) && !SCR_EL3Field::TCR2En.fetch_bit(self.scr_el3) {
            return false;
        } else if self.el2_enabled()
            && (self.is_hcrxel2_enabled() && !HCRX_EL2Field::SRMASKEn.fetch_bit(self.hcrx_el2))
        {
            return false;
        } else {
            return true;
        }
    }

    pub fn is_sctlr2el2_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::SCTLR2) {
            return false;
        }

        if self.have_el(EL3) && !SCR_EL3Field::SCTLR2En.fetch_bit(self.scr_el3) {
            return false;
        }

        self.el2_enabled()
    }
}
