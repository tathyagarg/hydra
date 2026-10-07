use crate::cpu::{
    BranchType, Cpu, DecodeType, EndOfInstructionReason, ExtendType, Feature, HIGH, ShiftType,
    brbe::{BRBSRC_EL1_Type, BRBTGT_EL1_Type},
    common::*,
    constraint::{Constraint, Unpredictable},
    exeception::{Exception, ExceptionRecord},
    fault::{
        AccessDescriptor, AccessType, DEFAULT_PARTID, DEFAULT_PMG, FaultRecord, MPAMData, MPAMinfo,
        PARTIDSpaceType, PARTIDType, PMGType,
    },
    regs::{
        BRBCR_EL1Field, BRBCR_EL2Field, BRBFCR_EL0Field, BRBFCR_EL1Field, EDSCRField, HCR_EL2Field,
        MDCR_EL3Field, MPAM0_EL1Field, MPAM1_EL1Field, MPAM2_EL2Field, MPAM3_EL3Field,
        MPAMHCR_EL2Field, MPAMIDR_EL1Field, MPAMSM_EL1Field, MPAMVPMV_EL2Field, RegisterField,
        RegisterFieldRange, SCR_EL3Field, SCRField, SCTLRField, TCR_EL1Field,
    },
};

pub type EL = u8;

pub const EL0: EL = 0b00;
pub const EL1: EL = 0b01;
pub const EL2: EL = 0b10;
pub const EL3: EL = 0b11;

#[derive(PartialEq, Clone, Copy)]
pub enum SecurityState {
    NonSecure,
    Root,
    Realm,
    Secure,
}

fn replicate(val: u64, elem_bits: u32, total_bits: u32) -> u64 {
    println!("repl: {} {} {}", val, elem_bits, total_bits);
    let elem = val & ones(elem_bits);

    let mut result = 0;
    let mut pos = 0;

    while pos < total_bits {
        result |= elem << pos;
        pos += elem_bits;
    }

    println!("replicated: {:#X}", result);
    result
}

pub fn ror(val: u64, amount: u32, width: u32) -> u64 {
    let mask = ones(width);
    let val = val & mask;
    let amount = (amount % width) as u64;

    if amount == 0 {
        val
    } else {
        ((val >> amount) | (val << (width as u64 - amount))) & mask
    }
}

/// Signed Integer function: Converts unsigned int to signed int
/// Also called SignExtend function
/// The `N` is the number of used bits in the input `n`
pub(super) fn sint<const N: usize, T: Into<u64>>(n: T) -> i64 {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };
    let n = n.into();
    let sign_bit = 1u64 << (N - 1);
    (if n & sign_bit != 0 {
        n | !mask
    } else {
        n & mask
    }) as i64
}

pub(super) fn add_with_carry<const N: usize>(x: u64, y: u64, carry_in: bool) -> (u64, NZCV) {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };

    let (unsigned_sum, carry) = {
        let (temp, c1) = if N == 64 {
            x.overflowing_add(y)
        } else {
            let (res, c) = (x as u32).overflowing_add(y as u32);
            (res as u64, c)
        };

        let (temp2, c2) = if N == 64 {
            temp.overflowing_add(carry_in as u64)
        } else {
            let (res, c) = (temp as u32).overflowing_add(carry_in as u32);
            (res as u64, c)
        };
        (temp2, c1 || c2)
    };

    let signed_sum = if N == 64 {
        sint::<N, u64>(x) + sint::<N, u64>(y) + carry_in as i64
    } else {
        (sint::<N, u32>(x as u32) as i32 + sint::<N, u32>(y as u32) as i32 + carry_in as i32) as i64
    };

    let result = unsigned_sum & mask;

    let n = (result >> (N - 1)) & 1 == 1;
    let z = result == 0;
    let c = carry;
    let v = result as i64 != signed_sum;

    (result, NZCV::init(n, z, c, v))
}

fn extend(val: u64, nbits: usize, unsigned: bool) -> u64 {
    let mask = if nbits == 64 {
        u64::MAX
    } else {
        (1u64 << nbits) - 1
    };

    let value = val & mask;

    if unsigned || nbits == 64 {
        value
    } else {
        let sign_bit = 1u64 << (nbits - 1);

        if value & sign_bit != 0 {
            value | !mask
        } else {
            value
        }
    }
}

pub(super) fn extend_reg<const N: usize>(val: u64, exttype: ExtendType, shift: u8) -> u64 {
    let (unsigned, len): (bool, usize) = match exttype {
        ExtendType::SXTB => (false, 8),
        ExtendType::SXTH => (false, 16),
        ExtendType::SXTW => (false, 32),
        ExtendType::SXTX => (false, 64),
        ExtendType::UXTB => (true, 8),
        ExtendType::UXTH => (true, 16),
        ExtendType::UXTW => (true, 32),
        ExtendType::UXTX => (true, 64),
    };

    let nbits = len.min(N);
    let extval = extend(val, nbits, unsigned);

    return extval << shift;
}

pub(super) fn shift_reg(val: u64, shift_type: ShiftType, amount: usize) -> u64 {
    match shift_type {
        ShiftType::LSL => val << amount,
        ShiftType::LSR => val >> amount,
        ShiftType::ASR => (val as i64 >> amount) as u64,
        ShiftType::ROR => val,
    }
}

/// DecodeBitMasks()
/// ================
/// Decode AArch64 bitfield and logical immediate masks which use a similar encoding structure
pub(super) fn decode_bit_masks<const M: usize>(imm_n: bool, imms: u8, immr: u8) -> (u64, u64) {
    let combined = ((imm_n as u8) << 6) | ((!imms) & 0x3f);
    let len = 63 - (combined as u64).leading_zeros() as usize;
    let levels = (1u8 << len) - 1;

    let s = imms & levels;
    let r = immr & levels;

    let diff = s.wrapping_sub(r);

    let esize = (1u64 << len) as u32;
    let d = diff & levels;

    let welem = (1u64 << (s + 1)) - 1;
    let telem = (1u64 << (d + 1)) - 1;

    let wmask = replicate(ror(welem, r as u32, esize), esize, M as u32);
    let tmask = replicate(telem, esize as u32, M as u32);

    (wmask, tmask)
}

impl Cpu {
    #[allow(unused_variables)]
    pub fn halt(&mut self, reason: u8, is_async: bool, fault: FaultRecord) {
        todo!("not doing alll this")
    }

    pub fn halted(&self) -> bool {
        ![0b00000001, 0b00000010].contains(&EDSCRField::STATUS.fetch_range(self.edscr))
    }

    pub fn double_lock_status(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::DoubleLock) {
            return false;
        }

        todo!()
    }

    pub fn external_invasive_debug_enabled(&self) -> bool {
        self.dbgen == HIGH
    }

    pub fn external_secure_invasive_debug_enabled(&self) -> bool {
        if !self.have_secure_state() {
            return Cpu::is_feature_implemented(Feature::RME) && self.rtpiden == HIGH;
        }

        self.external_invasive_debug_enabled() && self.spiden == HIGH
    }

    pub fn external_root_invasive_debug_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            return false;
        }

        return self.external_invasive_debug_enabled()
            && (!Cpu::is_feature_implemented(Feature::SEL2)
                || self.external_secure_invasive_debug_enabled())
            && self.external_realm_invasive_debug_enabled()
            && self.rtpiden == HIGH;
    }

    pub fn external_realm_invasive_debug_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            return false;
        }

        self.external_invasive_debug_enabled() && self.rlpiden == HIGH
    }

    pub fn halting_allowed(&self) -> bool {
        if self.halted() || self.double_lock_status() {
            return false;
        }

        let ss = self.current_security_state();
        match ss {
            SecurityState::NonSecure => self.external_invasive_debug_enabled(),
            SecurityState::Secure => self.external_secure_invasive_debug_enabled(),
            SecurityState::Root => self.external_root_invasive_debug_enabled(),
            SecurityState::Realm => self.external_realm_invasive_debug_enabled(),
        }
    }

    pub fn branch_target_check(&self) {
        assert!(Cpu::is_feature_implemented(Feature::BTI) && !self.using_aarch32());

        if self.halted() {
            return;
        } else if self.pstate.BTYPE() == 0 {
            return;
        } else {
            unimplemented!("InGuardedPage and BTypeCompatible checks")
        }
    }

    pub fn end_of_instruction(&self, end_reason: EndOfInstructionReason) {
        panic!("End of Instruction: {:?}", end_reason);
    }

    pub fn execute_as_nop(&self) {
        self.end_of_instruction(EndOfInstructionReason::Retired);
    }

    pub fn this_instr_addr<const N: usize>(&self) -> u64 {
        assert!(N == 64 || (N == 32 && self.using_aarch32()));
        n_bits(self.get_PC64(), N)
    }

    pub fn aarch32_general_execeptions_to_aarch64(&self) -> bool {
        return (self.pstate.EL() == EL0 && !self.el_using_aarch32(EL1))
            || (self.el2_enabled()
                && !self.el_using_aarch32(EL2)
                && HCR_EL2Field::TGE.fetch_bit(self.hcr_el2));
    }

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

    pub fn aarch32_undefined(&self) {
        if self.aarch32_general_execeptions_to_aarch64() {
            self.aarch64_undefined();
        }

        unimplemented!("AArch32_TakeUndefInstrException");
    }

    pub fn undefined(&self) {
        if self.using_aarch32() {
            self.aarch32_undefined();
        } else {
            self.aarch64_undefined();
        }
    }

    pub fn end_of_decode(&self, reason: DecodeType) {
        if Cpu::is_feature_implemented(Feature::BTI) && !self.using_aarch32() {
            self.branch_target_check();
        }

        match reason {
            DecodeType::NOP => self.execute_as_nop(),
            DecodeType::UNDEF => self.undefined(),
            DecodeType::OK => {}
        }
    }

    pub fn imp_def_int(&self, name: &str) -> u64 {
        match name {
            "Number of BRBE Branch records" => 32,
            _ => unimplemented!(),
        }
    }

    pub fn imp_def_bool(&self, name: &str) -> bool {
        match name {
            "Shared SMCU" => false,
            "MPAMSM_EL1 label precedence" => false,
            _ => unimplemented!(),
        }
    }

    pub fn condition_holds(&self, cond: u8) -> bool {
        let result = match cond >> 1 {
            0b000 => self.pstate.Z(),
            0b001 => self.pstate.C(),
            0b010 => self.pstate.N(),
            0b011 => self.pstate.V(),
            0b100 => self.pstate.C() && !self.pstate.Z(),
            0b101 => self.pstate.N() == self.pstate.V(),
            0b110 => self.pstate.N() == self.pstate.V() && !self.pstate.Z(),
            0b111 => true,
            _ => unreachable!(),
        };

        if cond & 1 == 1 && cond != 0b1111 {
            !result
        } else {
            result
        }
    }

    #[allow(unused_variables)]
    pub fn branch_not_taken(&self, branch_type: BranchType, branch_conditional: bool) {
        if Cpu::is_feature_implemented(Feature::SPE) {
            todo!()
        }
    }

    pub fn branch_to<const N: usize>(
        &mut self,
        target: u64,
        branch_type: BranchType,
        branch_conditional: bool,
    ) {
        if N == 32 {
            self._pc = target;
        } else {
            let target_vaddress = self.aarch64_branchaddr(target, self.pstate.EL());
            if Cpu::is_feature_implemented(Feature::BRBE)
                && matches!(
                    branch_type,
                    BranchType::DIR
                        | BranchType::INDIR
                        | BranchType::DIRCALL
                        | BranchType::INDCALL
                        | BranchType::RET
                )
            {
                self.brbe_branch(branch_type, branch_conditional, target_vaddress);
            }

            // let branch_taken = true;

            if Cpu::is_feature_implemented(Feature::SPE) {
                todo!()
            }

            self._pc = target_vaddress;
        }
    }

    pub fn security_state_at_el(&self, el: EL) -> SecurityState {
        if Cpu::is_feature_implemented(Feature::RME) {
            if el == EL3 {
                return SecurityState::Root;
            }

            let nse = self.effective_scr_el3_nse() as u8;
            let ns = self.effective_scr_el3_ns() as u8;
            let effective_nse_ns = nse << 1 | ns;

            match effective_nse_ns {
                0b00 => {
                    if !Cpu::is_feature_implemented(Feature::SEL2) {
                        return SecurityState::Secure;
                    } else {
                        unreachable!()
                    }
                }
                0b01 => {
                    return SecurityState::NonSecure;
                }
                0b11 => {
                    return SecurityState::Realm;
                }
                _ => unreachable!(),
            }
        }

        if !self.have_el(EL3) {
            if self.secure_only_implementation() {
                return SecurityState::Secure;
            } else {
                return SecurityState::NonSecure;
            }
        } else if el == EL3 {
            return SecurityState::Secure;
        } else {
            if !self.el_using_aarch32(EL3) {
                if self.effective_scr_el3_ns() {
                    return SecurityState::NonSecure;
                } else {
                    return SecurityState::Secure;
                }
            } else {
                if SCRField::NS.fetch_bit(self.scr) {
                    return SecurityState::NonSecure;
                } else {
                    return SecurityState::Secure;
                }
            }
        }
    }

    pub fn current_security_state(&self) -> SecurityState {
        self.security_state_at_el(self.pstate.EL())
    }

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

    pub fn branch_raw_cycle_count(&self) -> u64 {
        0
    }

    pub fn brb_cycle_counting_enabled(&self) -> bool {
        if self.have_el(EL2) && !BRBCR_EL2Field::CC.fetch_bit(self.brbcr_el2) {
            return false;
        }

        if !BRBCR_EL1Field::CC.fetch_bit(self.brbcr_el1) {
            return false;
        }

        return true;
    }

    pub fn first_branch_after_prohibited(&self) -> bool {
        false
    }

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

    pub fn brbe_mispredict_allowed(&self) -> bool {
        if self.have_el(EL2) && !BRBCR_EL2Field::MPRED.fetch_bit(self.brbcr_el2) {
            return false;
        }

        if !BRBCR_EL1Field::MPRED.fetch_bit(self.brbcr_el1) {
            return false;
        }

        true
    }

    pub fn branch_mispredict(&self) -> bool {
        false
    }

    pub fn get_brbe_num_records(&self) -> u64 {
        self.imp_def_int("Number of BRBE Branch records")
    }

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

    pub fn pmu_event1(&self, pmuevent: u16) {
        self.pmu_event2(pmuevent, 1);
    }

    pub fn pmu_event2(&self, pmuevent: u16, increment: usize) {
        let counters = Cpu::NUM_PMU_COUNTERS;
        if counters != 0 {
            for idx in 0..counters {
                self.pmu_event3(pmuevent, increment, idx);
            }
        }

        if self.have_aarch64()
            && Cpu::is_feature_implemented(Feature::PMUv3_ICNTR)
            && pmuevent == Cpu::PMU_EVENT_INST_RETIRED
        {
            todo!()
        }
    }

    #[allow(unused_variables)]
    pub fn pmu_event3(&self, pmuevent: u16, increment: usize, idx: usize) {
        if !Cpu::is_feature_implemented(Feature::PMUv3) {
            return;
        }

        if self.using_aarch32() {
            todo!()
        } else {
            todo!()
        }
    }

    pub fn using_aarch32(&self) -> bool {
        let aarch32 = self.pstate.nRW;

        return aarch32;
    }

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

    pub fn aarch64_branchaddr(&self, vaddress: u64, el: EL) -> u64 {
        let msbit = self.addrtop(vaddress, true, el);
        if msbit == 63 {
            return vaddress;
        } else if ((el == EL0 || el == EL1) || self.el_is_in_host(el)) && nth_bit(vaddress, msbit) {
            return sint::<64, u64>(vaddress & ((1 << msbit) - 1)) as u64;
        } else {
            return vaddress & ((1 << msbit) - 1);
        }
    }

    pub fn have_aarch32_el(&self, el: EL) -> bool {
        match el {
            EL0 => Cpu::is_feature_implemented(Feature::AA32EL0),
            EL1 => Cpu::is_feature_implemented(Feature::AA32EL1),
            EL2 => Cpu::is_feature_implemented(Feature::AA32EL2),
            EL3 => Cpu::is_feature_implemented(Feature::AA32EL3),
            _ => unreachable!(),
        }
    }

    pub fn have_aarch64(&self) -> bool {
        Cpu::is_feature_implemented(Feature::AA64EL0)
            || Cpu::is_feature_implemented(Feature::AA64EL1)
            || Cpu::is_feature_implemented(Feature::AA64EL2)
            || Cpu::is_feature_implemented(Feature::AA64EL3)
    }

    pub fn have_el(&self, el: EL) -> bool {
        match el {
            EL0 | EL1 => true,
            EL2 => {
                Cpu::is_feature_implemented(Feature::AA64EL2)
                    || Cpu::is_feature_implemented(Feature::AA32EL2)
            }
            EL3 => {
                Cpu::is_feature_implemented(Feature::AA64EL3)
                    || Cpu::is_feature_implemented(Feature::AA32EL3)
            }
            _ => unreachable!(),
        }
    }

    pub fn have_secure_state(&self) -> bool {
        Cpu::is_feature_implemented(Feature::SECURE)
    }

    pub fn effective_scr_el3_ns(&self) -> bool {
        if !self.have_secure_state() {
            return true;
        } else if !self.have_el(EL3) {
            return false;
        } else if self.el_using_aarch32(EL3) {
            SCRField::NS.fetch_bit(self.scr)
        } else {
            SCR_EL3Field::NS.fetch_bit(self.scr_el3)
        }
    }

    pub fn effective_scr_el3_nse(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            return false;
        }

        SCR_EL3Field::NSE.fetch_bit(self.scr_el3)
    }

    pub fn effective_scr_el3_rw(&self) -> bool {
        if !self.have_aarch64() {
            return false;
        }

        if !self.have_aarch32_el(EL2) && !self.have_aarch32_el(EL1) {
            return true;
        }

        if self.have_aarch32_el(EL1) {
            if !self.have_aarch32_el(EL2) && self.effective_scr_el3_ns() {
                return true;
            }

            if Cpu::is_feature_implemented(Feature::SEL2)
                && SCR_EL3Field::EEL2.fetch_bit(self.scr_el3)
                && !self.effective_scr_el3_ns()
            {
                return true;
            }
        }

        SCR_EL3Field::RW.fetch_bit(self.scr_el3)
    }

    pub fn secure_only_implementation(&self) -> bool {
        return Cpu::is_feature_implemented(Feature::SECURE) && !self.have_el(EL3);
    }

    pub fn is_secure_el2_enabled(&self) -> bool {
        if self.have_el(EL2) && Cpu::is_feature_implemented(Feature::SEL2) {
            if self.have_el(EL3) {
                return !self.have_aarch32_el(EL3) && SCR_EL3Field::EEL2.fetch_bit(self.scr_el3);
            } else {
                return self.secure_only_implementation();
            }
        } else {
            return false;
        }
    }

    pub fn el2_enabled(&self) -> bool {
        let scr_ns = if self.have_el(EL3) {
            if self.have_aarch32_el(EL3) {
                Some(SCRField::NS.fetch_bit(self.scr))
            } else {
                Some(SCR_EL3Field::NS.fetch_bit(self.scr_el3))
            }
        } else {
            None
        };

        self.have_el(EL2)
            && (!self.have_el(EL3) || scr_ns == Some(true) || self.is_secure_el2_enabled())
    }

    pub fn effective_hcr_el2_e2h(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::VHE) {
            return false;
        }

        if !Cpu::is_feature_implemented(Feature::E2H0) {
            return true;
        }

        HCR_EL2Field::E2H.fetch_bit(self.hcr_el2)
    }

    pub fn el_is_in_host(&self, el: EL) -> bool {
        if !Cpu::is_feature_implemented(Feature::VHE) || self.el_using_aarch32(EL2) {
            return false;
        }

        match el {
            EL3 => false,
            EL2 => self.el2_enabled() && self.effective_hcr_el2_e2h(),
            EL1 => false,
            EL0 => {
                self.el2_enabled()
                    && self.effective_hcr_el2_e2h()
                    && HCR_EL2Field::TGE.fetch_bit(self.hcr_el2)
            }
            _ => unreachable!(),
        }
    }

    pub fn el_state_using_aarch32k(&self, el: EL, secure: bool) -> (bool, bool) {
        if !self.have_aarch32_el(el) {
            return (true, false);
        } else if secure && el == EL2 {
            return (true, false);
        } else if !self.have_aarch64() {
            return (true, true);
        }

        if el == EL3 {
            return (true, false);
        }

        if self.have_el(EL3)
            && !self.effective_scr_el3_rw()
            && (!secure
                || !Cpu::is_feature_implemented(Feature::SEL2)
                || !SCR_EL3Field::EEL2.fetch_bit(self.scr_el3))
        {
            return (true, true);
        }

        if el == EL2 {
            return (true, false);
        }

        if self.have_el(EL2)
            && self.el_is_in_host(EL0)
            && !HCR_EL2Field::RW.fetch_bit(self.hcr_el2)
            && (!secure
                || (Cpu::is_feature_implemented(Feature::SEL2))
                    && SCR_EL3Field::EEL2.fetch_bit(self.scr_el3))
        {
            return (true, true);
        }

        if el == EL1 {
            return (true, false);
        }

        if self.pstate.EL() == EL0 {
            return (true, self.pstate.nRW);
        } else {
            return (false, false);
        }
    }

    pub fn el_state_using_aarch32(&self, el: EL, secure: bool) -> bool {
        let (_, aarch32) = self.el_state_using_aarch32k(el, secure);

        aarch32
    }

    pub fn is_secure_below_el3(&self) -> bool {
        if self.have_el(EL3) {
            if !self.have_aarch64() {
                return !SCRField::NS.fetch_bit(self.scr);
            } else {
                return !SCR_EL3Field::NS.fetch_bit(self.scr_el3);
            }
        } else if self.have_el(EL2)
            && (!Cpu::is_feature_implemented(Feature::SEL2) || !self.have_aarch64())
        {
            return false;
        } else {
            return self.secure_only_implementation();
        }
    }

    pub fn el_using_aarch32(&self, el: EL) -> bool {
        self.el_state_using_aarch32(el, self.is_secure_below_el3())
    }

    pub fn addrtop(&self, address: u64, isinstr: bool, el: EL) -> usize {
        let regime = self.s1translationregime(el);

        if self.el_using_aarch32(regime) {
            return 31;
        } else {
            if self.effective_tbi(address, isinstr, el) {
                55
            } else {
                63
            }
        }
    }

    pub fn effective_tbi(&self, address: u64, isinstr: bool, el: EL) -> bool {
        let regime = self.s1translationregime(el);

        let (tbi, tbid): (bool, Option<bool>) = match regime {
            EL1 => {
                let tbi = if nth_bit(address, 55) {
                    TCR_EL1Field::TBI1.fetch_bit(self.tcr_el1)
                } else {
                    TCR_EL1Field::TBI0.fetch_bit(self.tcr_el1)
                };

                let tbid = if Cpu::is_feature_implemented(Feature::PAuth) {
                    Some(if nth_bit(address, 55) {
                        TCR_EL1Field::TBID1.fetch_bit(self.tcr_el1)
                    } else {
                        TCR_EL1Field::TBID0.fetch_bit(self.tcr_el1)
                    })
                } else {
                    None
                };

                (tbi, tbid)
            }
            _ => todo!("Exception Level {} not yet implemented", regime),
        };

        tbi && (!Cpu::is_feature_implemented(Feature::PAuth) || tbid == Some(false) || !isinstr)
    }

    pub fn s1translationregime(&self, el: EL) -> u8 {
        if el != EL0 {
            return el;
        }

        todo!("uhhh");
    }

    pub fn highest_el(&self) -> EL {
        if self.have_el(EL3) {
            return EL3;
        } else if self.have_el(EL2) {
            return EL2;
        } else {
            return EL1;
        }
    }

    pub fn mpam_is_enabled(&self) -> bool {
        let el = self.highest_el();
        match el {
            EL3 => MPAM3_EL3Field::MPAMEN.fetch_bit(self.mpam3_el3),
            EL2 => MPAM2_EL2Field::MPAMEN.fetch_bit(self.mpam2_el2),
            EL1 => MPAM1_EL1Field::MPAMEN.fetch_bit(self.mpam1_el1),
            _ => unreachable!(),
        }
    }

    pub fn use_primary_space_el2(&self) -> bool {
        if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3) {
            return !MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3);
        }

        return !self.mpam_is_enabled() || !MPAM2_EL2Field::ALTSP_EL2.fetch_bit(self.mpam2_el2);
    }

    pub fn use_primary_space_el10(&self) -> bool {
        if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3) {
            return !MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3);
        }

        return !self.mpam_is_enabled()
            || !self.el2_enabled()
            || !MPAM2_EL2Field::ALTSP_EL2.fetch_bit(self.mpam2_el2);
    }

    pub fn alt_pid_secure(&self, el: EL, primary_pid_space: PARTIDSpaceType) -> PARTIDSpaceType {
        let mut pidspace = primary_pid_space;
        match el {
            EL0 => {
                if self.el2_enabled() {
                    if self.el_is_in_host(EL0) {
                        if !self.use_primary_space_el2() {
                            pidspace = PARTIDSpaceType::NonSecure;
                        }
                    } else if self.use_primary_space_el10() {
                        pidspace = PARTIDSpaceType::NonSecure;
                    }
                } else if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3)
                    && MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3)
                {
                    pidspace = PARTIDSpaceType::NonSecure;
                }
            }
            EL1 => {
                if self.el2_enabled() {
                    if !self.use_primary_space_el10() {
                        pidspace = PARTIDSpaceType::NonSecure;
                    }
                } else if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3)
                    && MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3)
                {
                    pidspace = PARTIDSpaceType::NonSecure;
                }
            }
            EL2 => {
                if !self.use_primary_space_el2() {
                    pidspace = PARTIDSpaceType::NonSecure;
                }
            }
            _ => unreachable!(),
        }

        pidspace
    }

    pub fn alt_partid_space(
        &self,
        el: EL,
        security: SecurityState,
        primary_pid_space: PARTIDSpaceType,
    ) -> PARTIDSpaceType {
        match security {
            SecurityState::NonSecure => {
                assert!(el != EL3);
                return primary_pid_space;
            }
            SecurityState::Secure => {
                assert!(el != EL3);
                if primary_pid_space == PARTIDSpaceType::NonSecure {
                    return primary_pid_space;
                }

                return self.alt_pid_secure(el, primary_pid_space);
            }
            SecurityState::Root => {
                assert!(el == EL3);
                if MPAM3_EL3Field::ALTSP_EL3.fetch_bit(self.mpam3_el3) {
                    if MPAM3_EL3Field::RT_ALTSP_NS.fetch_bit(self.mpam3_el3) {
                        return PARTIDSpaceType::NonSecure;
                    } else {
                        return PARTIDSpaceType::Secure;
                    }
                } else {
                    return primary_pid_space;
                }
            }
            SecurityState::Realm => {
                assert!(el != EL3);
                todo!("Alt pid realm");
                // return self.alt_pid_realm(el, primary_pid_space);
            }
        }
    }

    pub fn get_mpam_partid_max(&self) -> PARTIDType {
        return MPAMIDR_EL1Field::PARTID_MAX.fetch_range(self.mpamidr_el1);
    }

    pub fn get_mpam_pmg_max(&self) -> PMGType {
        return MPAMIDR_EL1Field::PMG_MAX.fetch_range(self.mpamidr_el1);
    }

    pub fn get_mpam_partid(&self, mpamn: EL, mpamdata: &MPAMData) -> PARTIDType {
        if mpamdata.sm {
            return MPAMSM_EL1Field::PARTID_D.fetch_range(self.mpamsm_el1);
        }

        if mpamdata.ind {
            match mpamn {
                0b11 => MPAM3_EL3Field::PARTID_I.fetch_range(self.mpam3_el3),
                0b10 => {
                    if self.el2_enabled() {
                        MPAM2_EL2Field::PARTID_I.fetch_range(self.mpam2_el2)
                    } else {
                        DEFAULT_PARTID
                    }
                }
                0b01 => MPAM1_EL1Field::PARTID_I.fetch_range(self.mpam1_el1),
                0b00 => MPAM0_EL1Field::PARTID_I.fetch_range(self.mpam0_el1),
                _ => unreachable!(),
            }
        } else {
            match mpamn {
                0b11 => MPAM3_EL3Field::PARTID_D.fetch_range(self.mpam3_el3),
                0b10 => {
                    if self.el2_enabled() {
                        MPAM2_EL2Field::PARTID_D.fetch_range(self.mpam2_el2)
                    } else {
                        DEFAULT_PARTID
                    }
                }
                0b01 => MPAM1_EL1Field::PARTID_D.fetch_range(self.mpam1_el1),
                0b00 => MPAM0_EL1Field::PARTID_D.fetch_range(self.mpam0_el1),
                _ => unreachable!(),
            }
        }
    }

    pub fn mpam_is_virtual(&self, el: EL) -> bool {
        MPAMIDR_EL1Field::HAS_HCR.fetch_bit(self.mpamidr_el1)
            && self.el2_enabled()
            && (el == EL0
                && MPAMHCR_EL2Field::EL0_VPMEN.fetch_bit(self.mpamhcr_el2)
                && !self.el_is_in_host(EL0)
                || (el == EL1 && MPAMHCR_EL2Field::EL1_VPMEN.fetch_bit(self.mpamhcr_el2)))
    }

    pub fn mapvpmw(&self, vpartid: PARTIDType) -> PARTIDType {
        let wd = vpartid / 4;
        let vpmw = if wd < 8 {
            self.mpamvpmn_el2[wd as usize]
        } else {
            0
        };
        let vpme_lsb = (vpartid % 4) * 16;

        return ((vpmw >> vpme_lsb) & 0xFFFF) as u16;
    }

    pub fn map_vpartid(&self, vpartid: PARTIDType, partid_err: bool) -> (PARTIDType, bool) {
        let vpartid_max_raw: u8 = MPAMIDR_EL1Field::VPMR_MAX.fetch_range(self.mpamidr_el1);
        let vpartid_max = ((vpartid_max_raw << 2) + 3) as u16;
        let mut virt = vpartid;
        let mut ret: u16;
        let mut err = partid_err;

        if virt > vpartid_max {
            let (c, vid) = Unpredictable::OUT_OF_RANGE_VID.constrain_integer(0, vpartid_max);

            assert!([Constraint::DEFAULT_ID, Constraint::ANY_ID].contains(&c));

            match c {
                Constraint::DEFAULT_ID => virt = DEFAULT_PARTID,
                Constraint::ANY_ID => virt = vid,
                _ => unreachable!(),
            }
        }

        if nth_bit(self.mpamvpmv_el2, virt as usize) {
            ret = self.mapvpmw(virt);
        } else if MPAMVPMV_EL2Field::VPM_V0.fetch_bit(self.mpamvpmv_el2) {
            ret = (self.mpamvpmv_el2 & 0xFFFF) as u16;
        } else {
            ret = DEFAULT_PARTID;
            err = true;
        }

        let partid_max = self.get_mpam_partid_max();
        if ret > partid_max {
            ret = DEFAULT_PARTID;
            err = true;
        }

        (ret, err)
    }

    pub fn gen_partid(&self, el: EL, mpamdata: &MPAMData) -> (PARTIDType, bool) {
        let mut partidel = self.get_mpam_partid(el, mpamdata);
        let mut partid_err = false;

        if partidel > self.get_mpam_partid_max() {
            let (c, partid) =
                Unpredictable::OUT_OF_RANGE_ID.constrain_integer(0, self.get_mpam_partid_max());
            partid_err = true;

            assert!([Constraint::DEFAULT_ID, Constraint::ANY_ID].contains(&c));
            match c {
                Constraint::DEFAULT_ID => partidel = DEFAULT_PARTID,
                Constraint::ANY_ID => partidel = partid,
                _ => unreachable!(),
            };
        }

        if self.mpam_is_virtual(el) {
            return self.map_vpartid(partidel, partid_err);
        } else {
            (partidel, partid_err)
        }
    }

    pub fn get_mpam_pmg(&self, mpamn: EL, mpamdata: &MPAMData) -> PMGType {
        if mpamdata.sm {
            return MPAMSM_EL1Field::PMG_D.fetch_range(self.mpamsm_el1);
        }

        if mpamdata.ind {
            match mpamn {
                0b11 => MPAM3_EL3Field::PARTID_I.fetch_range(self.mpam3_el3),
                0b10 => {
                    if self.el2_enabled() {
                        MPAM2_EL2Field::PARTID_I.fetch_range(self.mpam2_el2)
                    } else {
                        DEFAULT_PMG
                    }
                }
                0b01 => MPAM1_EL1Field::PARTID_I.fetch_range(self.mpam1_el1),
                0b00 => MPAM0_EL1Field::PARTID_I.fetch_range(self.mpam0_el1),
                _ => unreachable!(),
            }
        } else {
            match mpamn {
                0b11 => MPAM3_EL3Field::PARTID_D.fetch_range(self.mpam3_el3),
                0b10 => {
                    if self.el2_enabled() {
                        MPAM2_EL2Field::PARTID_D.fetch_range(self.mpam2_el2)
                    } else {
                        DEFAULT_PMG
                    }
                }
                0b01 => MPAM1_EL1Field::PARTID_D.fetch_range(self.mpam1_el1),
                0b00 => MPAM0_EL1Field::PARTID_D.fetch_range(self.mpam0_el1),
                _ => unreachable!(),
            }
        }
    }

    pub fn gen_pmg(&self, el: EL, mpamdata: &MPAMData, partid_err: bool) -> PMGType {
        if partid_err && Unpredictable::USE_DEFAULT_PMG.constrain_bool() {
            return DEFAULT_PMG;
        }

        let groupel = self.get_mpam_pmg(el, mpamdata);
        let pmg_max = self.get_mpam_pmg_max();

        if groupel <= pmg_max {
            return groupel;
        }

        return DEFAULT_PMG;
    }

    pub fn gen_mpam(&self, el_in: EL, mpamdata: &MPAMData) -> MPAMinfo {
        let mut el = el_in;

        if el == EL0
            && self.el2_enabled()
            && MPAMHCR_EL2Field::GSTAPP_PLK.fetch_bit(self.mpamhcr_el2)
            && !HCR_EL2Field::TGE.fetch_bit(self.hcr_el2)
        {
            el = EL1;
        }

        let (partidel, perr) = self.gen_partid(el, mpamdata);
        let groupel = self.gen_pmg(el, mpamdata, perr);

        MPAMinfo {
            mpam_sp: PARTIDSpaceType::NonSecure, // placeholder
            partid: partidel,
            pmg: groupel,
        }
    }

    pub fn gen_mpam_at_el(&self, acctype: &AccessType, el: EL) -> MPAMinfo {
        let mpam_el = if acctype == &AccessType::NV2 { EL2 } else { el };
        let security = self.security_state_at_el(el);
        let mut pspace = PARTIDSpaceType::from(security);

        let mut mpam_data = MPAMData::gen_new_mpam_data();
        if Cpu::is_feature_implemented(Feature::MPAMv0p1)
            && MPAMIDR_EL1Field::HAS_FORCE_NS.fetch_bit(self.mpamidr_el1)
        {
            if MPAM3_EL3Field::FORCE_NS.fetch_bit(self.mpam3_el3)
                && security == SecurityState::Secure
            {
                pspace = PARTIDSpaceType::NonSecure;
            }
        }

        if Cpu::is_feature_implemented(Feature::RME)
            && MPAMIDR_EL1Field::HAS_ALTSP.fetch_bit(self.mpamidr_el1)
        {
            pspace = self.alt_partid_space(mpam_el, security, pspace);
        }

        if !self.mpam_is_enabled() {
            return MPAMinfo::default(pspace);
        }

        match acctype {
            AccessType::IFETCH | AccessType::IC => {
                mpam_data.ind = true;
            }
            AccessType::SME => {
                mpam_data.sm = self.imp_def_bool("Shared SMCU")
                    || self.imp_def_bool("MPAMSM_EL1 label precedence")
            }
            AccessType::FP | AccessType::ASIMD | AccessType::SVE => {
                mpam_data.sm = Cpu::is_feature_implemented(Feature::SME)
                    && self.pstate.SM
                    && (self.imp_def_bool("Shared SMCU")
                        || self.imp_def_bool("MPAMSM_EL1 label precedence"))
            }
            AccessType::TRBE => {
                todo!("assert some bs")
            }
            _ => {
                mpam_data.ind = false;
            }
        }

        if (Cpu::is_feature_implemented(Feature::MPAMv0p1)
            || Cpu::is_feature_implemented(Feature::MPAMv1p1))
            && MPAMIDR_EL1Field::HAS_SDEFLT.fetch_bit(self.mpamidr_el1)
        {
            if MPAM3_EL3Field::SDEFLT.fetch_bit(self.mpam3_el3) && security == SecurityState::Secure
            {
                return MPAMinfo::default(pspace);
            }
        }

        let mut mpaminfo = self.gen_mpam(mpam_el, &mpam_data);
        mpaminfo.mpam_sp = pspace;

        mpaminfo
    }

    pub fn gen_mpam_cur_el(&self, acctype: &AccessType) -> MPAMinfo {
        self.gen_mpam_at_el(acctype, self.pstate.EL())
    }

    pub fn new_accdesc_from_acctype(&self, acctype: AccessType) -> AccessDescriptor {
        let mpam = self.gen_mpam_cur_el(&acctype);
        self.new_accdesc(acctype, mpam)
    }

    pub fn new_accdesc(&self, acctype: AccessType, mpam: MPAMinfo) -> AccessDescriptor {
        AccessDescriptor {
            acctype,
            el: self.pstate.EL(),
            ss: self.security_state_at_el(self.pstate.EL()),
            acqsc: false,
            acqpc: false,
            relsc: false,
            limitedordered: false,
            exclusive: false,
            rcw: false,
            rcws: false,
            atomicop: false,
            nontemporal: false,
            read: false,
            write: false,
            pan: false,
            nonfault: false,
            firstfault: false,
            first: false,
            contiguous: false,
            predicated: false,
            streamingsve: false,
            ls64: false,
            withstatus: false,
            mops: false,
            a32lsmd: false,
            tagchecked: false,
            tagaccess: false,
            stzgm: false,
            mpam,
            Rs: None,
            Rs2: None,
            Rt: None,
            Rt2: None,
            ispair: false,
            highestaddressfirst: false,
            lowestaddress: true,

            cacheop: None,
            cachetype: None,
            modop: None,
            opscope: None,
            toplevel: None,
            varange: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ones_8bit_repeated() {
        assert_eq!(
            decode_bit_masks::<32>(false, 0b110100, 0),
            (0x1F1F_1F1F, 0x1F1F_1F1F)
        );
    }

    #[test]
    fn rotated_8bit_pattern() {
        let (w, _) = decode_bit_masks::<32>(false, 0b110100, 0b000010);

        assert_eq!(w, 0xC7C7_C7C7);
    }
    #[test]
    fn four_bit_element() {
        let (w, _) = decode_bit_masks::<32>(false, 0b111010, 0b000001);

        assert_eq!(w, 0xBBBB_BBBB);
    }
}
