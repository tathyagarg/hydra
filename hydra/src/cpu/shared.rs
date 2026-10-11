use crate::cpu::{
    BranchType, Cpu, DecodeType, EndOfInstructionReason, ExtendType, Feature, HIGH, Register,
    ShiftType,
    common::*,
    constraint::{Constraint, Unpredictable},
    fault::{
        AccessDescriptor, AccessType, DEFAULT_PARTID, DEFAULT_PMG, FaultRecord, MPAMData, MPAMinfo,
        MemOp, PARTIDSpaceType, PARTIDType, PMGType,
    },
    regs::{
        EDSCRField, HCR_EL2Field, MPAM0_EL1Field, MPAM1_EL1Field, MPAM2_EL2Field, MPAM3_EL3Field,
        MPAMHCR_EL2Field, MPAMIDR_EL1Field, MPAMSM_EL1Field, MPAMVPMV_EL2Field, RegisterField,
        RegisterFieldRange, SCR_EL3Field, SCRField, TCR_EL1Field,
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

pub enum Regime {
    EL3,
    EL30,
    EL2,
    EL20,
    EL10,
}

pub fn replicate(val: u64, elem_bits: u32, total_bits: u32) -> u64 {
    let elem = val & ones(elem_bits);

    let mut result = 0;
    let mut pos = 0;

    while pos < total_bits {
        result |= elem << pos;
        pos += elem_bits;
    }

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
pub(super) fn sign_extend<const N: usize, T: Into<u64>>(n: T) -> i64 {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };
    let n = n.into();
    let sign_bit = 1u64 << (N - 1);
    (if n & sign_bit != 0 {
        n | !mask
    } else {
        n & mask
    }) as i64
}

/// AddWithCarry()
/// ==============
/// Integer addition with carry input, returning result and NZCV flags
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
        sign_extend::<N, u64>(x) + sign_extend::<N, u64>(y) + carry_in as i64
    } else {
        (sign_extend::<N, u32>(x as u32) as i32
            + sign_extend::<N, u32>(y as u32) as i32
            + carry_in as i32) as i64
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

/// ExtendReg()
/// ===========
/// Perform a register extension and shift
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

/// ShiftReg()
/// ==========
/// Perform shift of a register operand
pub(super) fn shift_reg<const N: usize>(val: u64, shift_type: ShiftType, amount: usize) -> u64 {
    match shift_type {
        ShiftType::LSL => val << amount,
        ShiftType::LSR => val >> amount,
        ShiftType::ASR => {
            if N == 64 {
                (val as i64 >> amount) as u64
            } else {
                (val as i32 >> amount) as u64
            }
        }
        ShiftType::ROR => {
            if N == 64 {
                val.rotate_right(amount as u32)
            } else {
                (val as u32).rotate_right(amount as u32) as u64
            }
        }
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
    /// Halt()
    /// ======
    #[allow(unused_variables)]
    pub fn halt(&mut self, reason: u8, is_async: bool, fault: FaultRecord) {
        todo!("not doing alll this")
    }

    /// Halted()
    /// ========
    pub fn halted(&self) -> bool {
        ![0b00000001, 0b00000010].contains(&EDSCRField::STATUS.fetch_range(self.edscr))
    }

    /// DoubleLockStatus()
    /// ==================
    /// Returns the state of the OS Double Lock.
    ///   FALSE if OSDLR_EL1.DLK == 0 or DBGPRCR_EL1.CORENPDRQ == 1 or the PE is in Debug state.
    ///   TRUE if OSDLR_EL1.DLK == 1 and DBGPRCR_EL1.CORENPDRQ == 0 and the PE is in Non-debug state.
    pub fn double_lock_status(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::DoubleLock) {
            return false;
        }

        todo!()
    }

    /// ExternalInvasiveDebugEnabled()
    /// ==============================
    /// The definition of this function is IMPLEMENTATION DEFINED.
    /// In the recommended interface, this function returns the state of the DBGEN signal.
    pub fn external_invasive_debug_enabled(&self) -> bool {
        self.dbgen == HIGH
    }

    /// ExternalSecureInvasiveDebugEnabled()
    /// ====================================
    /// The definition of this function is IMPLEMENTATION DEFINED.
    /// In the recommended interface, this function returns the state of the (DBGEN AND SPIDEN) signal.
    /// CoreSight allows asserting SPIDEN without also asserting DBGEN, but this is not recommended.
    pub fn external_secure_invasive_debug_enabled(&self) -> bool {
        if !self.have_secure_state() {
            return Cpu::is_feature_implemented(Feature::RME) && self.rtpiden == HIGH;
        }

        self.external_invasive_debug_enabled() && self.spiden == HIGH
    }

    /// ExternalRootInvasiveDebugEnabled()
    /// ==================================
    /// The definition of this function is IMPLEMENTATION DEFINED.
    /// In the recommended interface, this function returns the state of the
    /// (DBGEN AND RLPIDEN AND RTPIDEN AND SPIDEN) signal when FEAT_SEL2 is implemented
    /// and the (DBGEN AND RLPIDEN AND RTPIDEN) signal when FEAT_SEL2 is not implemented.
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

    /// ExternalRealmInvasiveDebugEnabled()
    /// ===================================
    /// The definition of this function is IMPLEMENTATION DEFINED.
    /// In the recommended interface, this function returns the state of the
    /// (DBGEN AND RLPIDEN) signal.
    pub fn external_realm_invasive_debug_enabled(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            return false;
        }

        self.external_invasive_debug_enabled() && self.rlpiden == HIGH
    }

    /// HaltingAllowed()
    /// ================
    /// Returns TRUE if halting is currently allowed, FALSE if halting is prohibited.
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

    /// BranchTargetCheck()
    /// ===================
    /// This function is executed checks if the current instruction is a valid target for a branch
    /// taken into, or inside, a guarded page. It is executed on every cycle once the current
    /// instruction has been decoded and the values of InGuardedPage and BTypeCompatible have been
    /// determined for the current instruction.
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

    /// EndOfInstruction()
    /// ==================
    /// Terminate processing of the current instruction.
    pub fn end_of_instruction(&self, end_reason: EndOfInstructionReason) {
        panic!("End of Instruction: {:?}", end_reason);
    }

    /// ExecuteAsNOP()
    /// ==============
    pub fn execute_as_nop(&self) {
        self.end_of_instruction(EndOfInstructionReason::Retired);
    }

    /// ThisInstrAddr()
    /// ===============
    /// Return address of the current instruction.
    pub fn this_instr_addr<const N: usize>(&self) -> u64 {
        assert!(N == 64 || (N == 32 && self.using_aarch32()));
        n_bits(self.get_PC64(), N)
    }

    /// Undefined()
    /// ===========
    pub fn undefined(&self) {
        if self.using_aarch32() {
            self.aarch32_undefined();
        } else {
            self.aarch64_undefined();
        }
    }

    /// EndOfDecode()
    /// =============
    /// This function is invoked to end the Decode phase and performs Branch target Checks
    /// before taking any UNDEFINED exceptions, NOPs, or continuing to execute.
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

    /// ImpDefInt()
    /// ===========
    /// Return an IMPLEMENTATION DEFINED integer value identified by the string parameter.
    pub fn imp_def_int(&self, name: &str) -> u64 {
        match name {
            "Number of BRBE Branch records" => 32,
            _ => unimplemented!(),
        }
    }

    /// ImpDefBool()
    /// ============
    /// Return an IMPLEMENTATION DEFINED boolean value identified by the string parameter.
    pub fn imp_def_bool(&self, name: &str) -> bool {
        match name {
            "Shared SMCU" => false,
            "MPAMSM_EL1 label precedence" => false,
            _ => unimplemented!(),
        }
    }

    /// ConditionHolds()
    /// ================
    /// Return TRUE iff COND currently holds
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

    /// BranchNotTaken()
    /// ================
    /// Called when a branch is not taken.
    #[allow(unused_variables)]
    pub fn branch_not_taken(&self, branch_type: BranchType, branch_conditional: bool) {
        if Cpu::is_feature_implemented(Feature::SPE) {
            todo!()
        }
    }

    /// BranchTo()
    /// ==========
    /// Set program counter to a new address, with a branch type.
    /// Parameter branch_conditional indicates whether the executed branch has a conditional encoding.
    /// In AArch64 state the address might include a tag in the top eight bits.
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

    /// SecurityStateAtEL()
    /// ===================
    /// Returns the effective security state at the exception level based off current settings.
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

    /// CurrentSecurityState()
    /// ======================
    /// Returns the effective security state at the exception level based off current settings.
    pub fn current_security_state(&self) -> SecurityState {
        self.security_state_at_el(self.pstate.EL())
    }

    /// PMUEvent()
    /// ==========
    /// Generate a PMU event. By default, increment by 1.
    pub fn pmu_event1(&self, pmuevent: u16) {
        self.pmu_event2(pmuevent, 1);
    }

    /// PMUEvent()
    /// ==========
    /// Accumulate a PMU Event.
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

    /// PMUEvent()
    /// ==========
    /// Accumulate a PMU Event for a specific event counter.
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

    /// UsingAArch32()
    /// ==============
    /// Return TRUE if the current Exception level is using AArch32, FALSE if using AArch64_
    pub fn using_aarch32(&self) -> bool {
        let aarch32 = self.pstate.nRW;

        return aarch32;
    }

    /// AArch64_BranchAddr()
    /// ====================
    /// Return the virtual address with tag bits removed.
    /// This is typically used when the address will be stored to the program counter.
    pub fn aarch64_branchaddr(&self, vaddress: u64, el: EL) -> u64 {
        let msbit = self.addrtop(vaddress, true, el);
        if msbit == 63 {
            return vaddress;
        } else if ((el == EL0 || el == EL1) || self.el_is_in_host(el)) && nth_bit(vaddress, msbit) {
            return sign_extend::<64, u64>(vaddress & ((1 << msbit) - 1)) as u64;
        } else {
            return vaddress & ((1 << msbit) - 1);
        }
    }

    /// HaveAArch32EL()
    /// ===============
    /// Return TRUE if Exception level 'el' supports AArch32 in this implementation
    pub fn have_aarch32_el(&self, el: EL) -> bool {
        match el {
            EL0 => Cpu::is_feature_implemented(Feature::AA32EL0),
            EL1 => Cpu::is_feature_implemented(Feature::AA32EL1),
            EL2 => Cpu::is_feature_implemented(Feature::AA32EL2),
            EL3 => Cpu::is_feature_implemented(Feature::AA32EL3),
            _ => unreachable!(),
        }
    }

    /// HaveAArch64()
    /// =============
    /// Return TRUE if the highest Exception level is using AArch64 state.
    pub fn have_aarch64(&self) -> bool {
        Cpu::is_feature_implemented(Feature::AA64EL0)
            || Cpu::is_feature_implemented(Feature::AA64EL1)
            || Cpu::is_feature_implemented(Feature::AA64EL2)
            || Cpu::is_feature_implemented(Feature::AA64EL3)
    }

    /// HaveEL()
    /// ========
    /// Return TRUE if Exception level 'el' is supported
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

    /// HaveSecureState()
    /// =================
    /// Return TRUE if Secure State is supported.
    pub fn have_secure_state(&self) -> bool {
        Cpu::is_feature_implemented(Feature::SECURE)
    }

    /// EffectiveSCR_EL3_NS()
    /// =====================
    /// Returns the Effective value of SCR_EL3().NS.
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

    /// EffectiveSCR_EL3_NSE()
    /// ======================
    /// Returns the Effective value of SCR_EL3().NSE.
    pub fn effective_scr_el3_nse(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::RME) {
            return false;
        }

        SCR_EL3Field::NSE.fetch_bit(self.scr_el3)
    }

    /// EffectiveSCR_EL3_RW()
    /// =====================
    /// Returns the Effective value of SCR_EL3().RW.
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

    /// SecureOnlyImplementation()
    /// ==========================
    /// Returns TRUE if the security state is always Secure for this implementation.
    pub fn secure_only_implementation(&self) -> bool {
        return Cpu::is_feature_implemented(Feature::SECURE) && !self.have_el(EL3);
    }

    /// IsSecureEL2Enabled()
    /// ====================
    /// Returns TRUE if Secure EL2 is enabled, FALSE otherwise.
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

    /// EL2Enabled()
    /// ============
    /// Returns TRUE if EL2 is present and executing
    /// - with the PE in Non-secure state when Non-secure EL2 is implemented, or
    /// - with the PE in Realm state when Realm EL2 is implemented, or
    /// - with the PE in Secure state when Secure EL2 is implemented and enabled, or
    /// - when EL3 is not implemented.
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

    /// EffectiveHCR_EL2_E2H()
    /// ======================
    /// Returns the Effective value of HCR_EL2().E2H.
    pub fn effective_hcr_el2_e2h(&self) -> bool {
        if !Cpu::is_feature_implemented(Feature::VHE) {
            return false;
        }

        if !Cpu::is_feature_implemented(Feature::E2H0) {
            return true;
        }

        HCR_EL2Field::E2H.fetch_bit(self.hcr_el2)
    }

    /// ELIsInHost()
    /// ============
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

    /// ELStateUsingAArch32K()
    /// ======================
    /// Returns (known, aarch32):
    ///   'known' is FALSE for EL0 if the current Exception level is not EL0 and EL1 is
    ///           using AArch64, since it cannot determine the state of EL0; TRUE otherwise.
    ///   'aarch32' is TRUE if the specified Exception level is using AArch32; FALSE otherwise.
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

    /// ELStateUsingAArch32()
    /// =====================
    pub fn el_state_using_aarch32(&self, el: EL, secure: bool) -> bool {
        let (_, aarch32) = self.el_state_using_aarch32k(el, secure);

        aarch32
    }

    /// IsSecureBelowEL3()
    /// ==================
    /// Return TRUE if an Exception level below EL3 is in Secure state
    /// or would be following an exception return to that level.
    ///
    /// That is, if at AArch64 EL3 or in AArch32 Monitor mode, whether an
    /// exception return would pass to Secure or Non-secure state.
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

    /// ELUsingAArch32()
    /// ================
    pub fn el_using_aarch32(&self, el: EL) -> bool {
        self.el_state_using_aarch32(el, self.is_secure_below_el3())
    }

    /// AddrTop()
    /// =========
    /// Return the MSB number of a virtual address in the stage 1 translation regime for "el".
    /// If EL1 is using AArch64 then addresses from EL0 using AArch32 are zero-extended to 64 bits.
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

    /// EffectiveTBI()
    /// ==============
    /// Returns the effective TBI in the AArch64 stage 1 translation regime for "el".
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

    /// S1TranslationRegime()
    /// =====================
    /// Stage 1 translation regime for the given Exception level
    pub fn s1translationregime(&self, el: EL) -> u8 {
        if el != EL0 {
            return el;
        } else if self.have_el(EL3)
            && self.el_using_aarch32(EL3)
            && SCRField::NS.fetch_bit(self.scr)
        {
            return EL3;
        } else if Cpu::is_feature_implemented(Feature::VHE) && self.el_is_in_host(el) {
            return EL2;
        } else {
            return EL1;
        }
    }

    /// HighestEL()
    /// ===========
    /// Returns the highest implemented Exception level.
    pub fn highest_el(&self) -> EL {
        if self.have_el(EL3) {
            return EL3;
        } else if self.have_el(EL2) {
            return EL2;
        } else {
            return EL1;
        }
    }

    /// MPAMIsEnabled()
    /// ===============
    /// Returns TRUE if MPAM is enabled.
    pub fn mpam_is_enabled(&self) -> bool {
        let el = self.highest_el();
        match el {
            EL3 => MPAM3_EL3Field::MPAMEN.fetch_bit(self.mpam3_el3),
            EL2 => MPAM2_EL2Field::MPAMEN.fetch_bit(self.mpam2_el2),
            EL1 => MPAM1_EL1Field::MPAMEN.fetch_bit(self.mpam1_el1),
            _ => unreachable!(),
        }
    }

    /// UsePrimarySpaceEL2()
    /// ====================
    /// Checks whether Primary space is configured in the
    /// MPAM3_EL3 and MPAM2_EL2 ALTSP control bits that affect
    /// MPAM ALTSP use at EL2.
    pub fn use_primary_space_el2(&self) -> bool {
        if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3) {
            return !MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3);
        }

        return !self.mpam_is_enabled() || !MPAM2_EL2Field::ALTSP_EL2.fetch_bit(self.mpam2_el2);
    }

    /// UsePrimarySpaceEL10()
    /// =====================
    /// Checks whether Primary space is configured in the
    /// MPAM3_EL3 and MPAM2_EL2 ALTSP control bits that affect
    /// MPAM ALTSP use at EL1 and EL0.
    pub fn use_primary_space_el10(&self) -> bool {
        if !MPAM3_EL3Field::ALTSP_HEN.fetch_bit(self.mpam3_el3) {
            return !MPAM3_EL3Field::ALTSP_HFC.fetch_bit(self.mpam3_el3);
        }

        return !self.mpam_is_enabled()
            || !self.el2_enabled()
            || !MPAM2_EL2Field::ALTSP_EL2.fetch_bit(self.mpam2_el2);
    }

    /// AltPIDSecure()
    /// ==============
    /// Compute PARTID space as either the primary PARTID space or
    /// alternative PARTID space in the Secure Security state.
    /// Helper for AltPARTIDSpace.
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

    /// AltPARTIDSpace()
    /// ================
    /// From the Security state, EL and ALTSP configuration, determine
    /// whether to primary space or the alt space is selected and which
    /// PARTID space is the alternative space. Return that alternative
    /// PARTID space if selected or the primary space if not.
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

    /// GetMPAM_PARTID()
    /// ================
    /// Returns a PARTID
    /// If mpamdata.sm is TRUE then PARTID is from MPAMSM_EL1.
    /// Otherwise, the PARTID is from MPAMn_ELx.
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

    /// GetMPAM_PARTID_MAX()
    /// ====================
    /// Returns the maximum supported PARTID value for the implemented MPAM version.
    pub fn get_mpam_partid_max(&self) -> PARTIDType {
        return MPAMIDR_EL1Field::PARTID_MAX.fetch_range(self.mpamidr_el1);
    }

    /// GetMPAM_PMG()
    /// =============
    /// Returns a PMG.
    /// if mpamdata.sm is TRUE then PMG is from MPAMSM_EL1.
    /// Otherwise, PMG is from MPAMn_ELx.
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

    /// GetMPAM_PMG_MAX()
    /// =================
    /// Returns the maximum supported PMG value for the implemented MPAM version.
    pub fn get_mpam_pmg_max(&self) -> PMGType {
        return MPAMIDR_EL1Field::PMG_MAX.fetch_range(self.mpamidr_el1);
    }

    /// MPAMIsVirtual()
    /// ===============
    /// Returns TRUE if MPAM is configured to be virtual at the given el.
    pub fn mpam_is_virtual(&self, el: EL) -> bool {
        MPAMIDR_EL1Field::HAS_HCR.fetch_bit(self.mpamidr_el1)
            && self.el2_enabled()
            && (el == EL0
                && MPAMHCR_EL2Field::EL0_VPMEN.fetch_bit(self.mpamhcr_el2)
                && !self.el_is_in_host(EL0)
                || (el == EL1 && MPAMHCR_EL2Field::EL1_VPMEN.fetch_bit(self.mpamhcr_el2)))
    }

    /// mapvpmw()
    /// =========
    /// Map a virtual PARTID into a physical PARTID using
    /// the MPAMVPMn_EL2 registers.
    /// vpartid is now assumed in-range and valid (checked by caller)
    /// returns physical PARTID from mapping entry.
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

    /// MAP_vPARTID()
    /// =============
    /// Performs conversion of virtual PARTID into physical PARTID
    /// Contains all of the error checking and implementation
    /// choices for the conversion.
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

    /// GenPARTID()
    /// ===========
    /// Returns physical PARTID and error boolean for Exception level el.
    /// If mpamdata.sm is TRUE then PARTID is from MPAMSM_EL1.
    /// Otherwise, the PARTID is from MPAMn_ELx.
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

    /// GenPMG()
    /// ========
    /// Returns PMG for Exception level el.
    /// if mpamdata.sm is TRUE then PMG is from MPAMSM_EL1.
    /// Otherwise, PMG is from MPAMn_ELx.
    /// If PARTID generation (GenPARTID) encountered an error, GenPMG() should be
    /// called with partid_err as TRUE.
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

    /// GenMPAM()
    /// =========
    /// Returns MPAMinfo for Exception level el.
    /// If mpamdata.sm is TRUE returns MPAM information using MPAMSM_EL1.
    /// Otherwise returns MPAM information using MPAMn_ELx.
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

    /// GenMPAMAtEL()
    /// =============
    /// Returns MPAMinfo for the specified EL.
    /// May be called if MPAM is not implemented (but in an version that supports
    /// MPAM), MPAM is disabled, or in AArch32_ In AArch32, convert the mode to
    /// EL if can and use that to drive MPAM information generation. If mode
    /// cannot be converted, MPAM is not implemented, or MPAM is disabled return
    /// default MPAM information for the current security state.
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

    /// GenMPAMCurEL()
    /// ==============
    /// Returns MPAMinfo for the current EL and security state.
    /// May be called if MPAM is not implemented (but in an version that supports
    /// MPAM), MPAM is disabled, or in AArch32_ In AArch32, convert the mode to
    /// EL if can and use that to drive MPAM information generation. If mode
    /// cannot be converted, MPAM is not implemented, or MPAM is disabled return
    /// default MPAM information for the current security state.
    pub fn gen_mpam_cur_el(&self, acctype: &AccessType) -> MPAMinfo {
        self.gen_mpam_at_el(acctype, self.pstate.EL())
    }

    /// NewAccDesc()
    /// ============
    /// Create a new AccessDescriptor with initialized fields
    pub fn new_accdesc_from_acctype(&self, acctype: AccessType) -> AccessDescriptor {
        let mpam = self.gen_mpam_cur_el(&acctype);
        self.new_accdesc(acctype, mpam)
    }

    /// NewAccDesc()
    /// ============
    /// Create a new AccessDescriptor with initialized fields
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

    /// CreateAccDescGPR()
    /// ==================
    /// Access descriptor for general purpose register loads/stores
    /// without exclusive or ordering semantics
    pub fn create_accdesc_gpr_pair(
        &self,
        memop: MemOp,
        nontemporal: bool,
        privileged: bool,
        tagchecked: bool,
        ispair: bool,
        Rt: Option<Register>,
        Rt2: Option<Register>,
    ) -> AccessDescriptor {
        let mut accdesc = self.new_accdesc_from_acctype(AccessType::GPR);

        accdesc.el = if !privileged { EL0 } else { self.pstate.EL() };
        accdesc.nontemporal = nontemporal;
        accdesc.read = memop == MemOp::LOAD;
        accdesc.write = memop == MemOp::STORE;
        accdesc.pan = true;
        accdesc.tagchecked = tagchecked;
        accdesc.Rt = Rt;
        accdesc.Rt2 = Rt2;
        accdesc.ispair = ispair;

        accdesc
    }

    /// CreateAccDescGPR()
    /// ==================
    /// Access descriptor for general purpose register loads/stores
    /// without exclusive or ordering semantics
    pub fn create_accdesc_gpr_rt(
        &self,
        memop: MemOp,
        nontemporal: bool,
        privileged: bool,
        tagchecked: bool,
        Rt: Option<Register>,
    ) -> AccessDescriptor {
        let ispair = false;
        let Rt2 = None;
        self.create_accdesc_gpr_pair(memop, nontemporal, privileged, tagchecked, ispair, Rt, Rt2)
    }

    /// CreateAccDescGPR()
    /// ==================
    /// Access descriptor for general purpose register loads/stores
    /// without exclusive or ordering semantics
    pub fn create_accdesc_gpr(
        &self,
        memop: MemOp,
        nontemporal: bool,
        privileged: bool,
        tagchecked: bool,
    ) -> AccessDescriptor {
        let Rt = None;
        self.create_accdesc_gpr_rt(memop, nontemporal, privileged, tagchecked, Rt)
    }

    /// PointerAddCheckAtEL()
    /// =====================
    /// Apply Checked Pointer Arithmetic for addition at the specified EL.
    #[allow(unused_variables)]
    pub fn pointer_add_check_at_el(&self, el: EL, result: u64, base: u64) -> u64 {
        todo!("bro")
    }

    /// AddressAdd()
    /// ============
    /// Add an address with an offset and return the result.
    /// If FEAT_CPA2 is implemented, the pointer arithmetic is checked.
    pub fn address_add(&self, base: u64, offset: u64, accdesc: &AccessDescriptor) -> u64 {
        self.pointer_add_check_at_el(accdesc.el, base + offset, base)
    }

    /// TranslationRegime()
    /// ===================
    /// Select the translation regime given the target EL and PE state
    pub fn translation_regime(&self, el: EL) -> Regime {
        if el == EL3 {
            if self.el_using_aarch32(EL3) {
                Regime::EL30
            } else {
                Regime::EL3
            }
        } else if el == EL2 {
            if self.el_is_in_host(EL2) {
                Regime::EL20
            } else {
                Regime::EL2
            }
        } else if el == EL1 {
            Regime::EL10
        } else if el == EL0 {
            if self.current_security_state() == SecurityState::Secure && self.el_using_aarch32(EL3)
            {
                Regime::EL30
            } else if self.el_is_in_host(EL0) {
                Regime::EL20
            } else {
                Regime::EL10
            }
        } else {
            unreachable!()
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
