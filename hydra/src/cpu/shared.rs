use crate::cpu::{
    BranchType, Cpu, DecodeType, EndOfInstructionReason, ExtendType, Feature, HIGH, Register,
    ShiftType,
    common::*,
    constraint::{Constraint, Unpredictable},
    exeception::{FullAddress, PASpace},
    fault::{
        AccessDescriptor, AccessType, CacheOp, DEFAULT_PARTID, DEFAULT_PMG, ErrorState, Fault,
        FaultRecord, GPCF, GPCFRecord, MPAMData, MPAMinfo, MemAtomicOp, MemOp, PARTIDSpaceType,
        PARTIDType, PMGType, VARange,
    },
    memory_acc::{
        AddressDescriptor, AddressSize, DeviceType, MemAttr_NC, MemAttr_WB, MemAttr_WT,
        MemAttrHints, MemHint_No, MemHint_RA, MemHint_RWA, MemTagType, MemType, MemoryAttributes,
        PhysMemRetStatus, S1TTWParams, Shareability, TGx, TTWState,
    },
    regs::{
        EDSCRField, GPCBW_EL3Field, GPCCR_EL3Field, GPTBR_EL3Field, HCR_EL2Field, HCR2Field,
        HCRField, HCRX_EL2Field, MPAM0_EL1Field, MPAM1_EL1Field, MPAM2_EL2Field, MPAM3_EL3Field,
        MPAMHCR_EL2Field, MPAMIDR_EL1Field, MPAMSM_EL1Field, MPAMVPMV_EL2Field, RegisterField,
        RegisterFieldRange, SCR_EL3Field, SCRField, TCR_EL1Field,
    },
};

pub type EL = u8;

pub const EL0: EL = 0b00;
pub const EL1: EL = 0b01;
pub const EL2: EL = 0b10;
pub const EL3: EL = 0b11;

pub trait Faultable {
    fn is_fault(&self) -> bool;
}

#[allow(non_upper_case_globals)]
pub const GPTRange_4KB: AddressSize = 12;
#[allow(non_upper_case_globals)]
pub const GPTRange_16KB: AddressSize = 14;
#[allow(non_upper_case_globals)]
pub const GPTRange_64KB: AddressSize = 16;
#[allow(non_upper_case_globals)]
pub const GPTRange_2MB: AddressSize = 21;
#[allow(non_upper_case_globals)]
pub const GPTRange_32MB: AddressSize = 25;
#[allow(non_upper_case_globals)]
pub const GPTRange_512MB: AddressSize = 29;
#[allow(non_upper_case_globals)]
pub const GPTRange_1GB: AddressSize = 30;
#[allow(non_upper_case_globals)]
pub const GPTRange_16GB: AddressSize = 34;
#[allow(non_upper_case_globals)]
pub const GPTRange_64GB: AddressSize = 36;
#[allow(non_upper_case_globals)]
pub const GPTRange_512GB: AddressSize = 39;

#[allow(non_upper_case_globals)]
pub const GPT_NoAccess: u8 = 0b0000;
#[allow(non_upper_case_globals)]
pub const GPT_Table: u8 = 0b0011;
#[allow(non_upper_case_globals)]
pub const GPT_Block: u8 = 0b0001;
#[allow(non_upper_case_globals)]
pub const GPT_Contig: u8 = 0b0001;
#[allow(non_upper_case_globals)]
pub const GPT_SystemAgent: u8 = 0b0100;
#[allow(non_upper_case_globals)]
pub const GPT_NonSecureProtected: u8 = 0b0101;
#[allow(non_upper_case_globals)]
pub const GPT_NA6: u8 = 0b0110;
#[allow(non_upper_case_globals)]
pub const GPT_NA7: u8 = 0b0111;
#[allow(non_upper_case_globals)]
pub const GPT_Secure: u8 = 0b1000;
#[allow(non_upper_case_globals)]
pub const GPT_NonSecure: u8 = 0b1001;
#[allow(non_upper_case_globals)]
pub const GPT_Root: u8 = 0b1010;
#[allow(non_upper_case_globals)]
pub const GPT_Realm: u8 = 0b1011;
#[allow(non_upper_case_globals)]
pub const GPT_NonSecureOnly: u8 = 0b1101;
#[allow(non_upper_case_globals)]
pub const GPT_Any: u8 = 0b1111;

pub struct GPTTable {
    pub address: u64,
}

pub struct GPTEntry {
    pub gpi: u8,
    pub size: AddressSize,
    pub contig_size: AddressSize,
    pub istable: bool,
    pub level: u8,
    pub pa: u64,
}

impl GPTEntry {
    pub fn arbitrary() -> Self {
        Self {
            gpi: 0,
            size: 0,
            contig_size: 0,
            istable: false,
            level: 0,
            pa: 0,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum PGSe {
    _4KB,
    _16KB,
    _64KB,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SecurityState {
    NonSecure,
    Root,
    Realm,
    Secure,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// HasUnprivileged()
    /// =================
    /// Returns whether a translation regime serves EL0 as well as a higher EL
    pub fn has_unprivileged(&self, regime: Regime) -> bool {
        matches!(regime, Regime::EL30 | Regime::EL20 | Regime::EL10)
    }

    /// TGxGranuleBits()
    /// ================
    /// Retrieve the address size, in bits, of a granule
    pub fn tgx_granulebits(&self, tgx: TGx) -> AddressSize {
        match tgx {
            TGx::_4KB => 12,
            TGx::_16KB => 14,
            TGx::_64KB => 16,
        }
    }

    /// DecodeShareability()
    /// ====================
    /// Decode shareability of target memory region
    pub fn decode_shareability(&self, sh: u8) -> Shareability {
        match sh {
            0b10 => Shareability::OSH,
            0b11 => Shareability::ISH,
            0b00 => Shareability::NSH,
            0b01 => match Unpredictable::Shareability.constrain() {
                Constraint::OSH => Shareability::OSH,
                Constraint::ISH => Shareability::ISH,
                Constraint::NSH => Shareability::NSH,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    /// DecodeSDFAttr()
    /// ===============
    /// Decode memory attributes using SDF (Short Descriptor Format) mapping
    pub fn decode_sdf_attrs(&self, rgn: u8) -> MemAttrHints {
        let (attrs, hints) = match rgn {
            0b00 => (MemAttr_NC, MemHint_No),
            0b01 => (MemAttr_WB, MemHint_RWA),
            0b10 => (MemAttr_WT, MemHint_RA),
            0b11 => (MemAttr_WB, MemHint_RA),
            _ => unreachable!(),
        };

        MemAttrHints {
            attrs,
            hints,
            transient: false,
        }
    }

    /// WalkMemAttrs()
    /// ==============
    /// Retrieve memory attributes of translation table walk
    pub fn walk_mem_attrs(&self, sh: u8, irgn: u8, orgn: u8) -> MemoryAttributes {
        let inner = self.decode_sdf_attrs(irgn);
        let outer = self.decode_sdf_attrs(orgn);
        let xs = !(inner.attrs == MemAttr_WB && outer.attrs == MemAttr_WB);

        MemoryAttributes {
            memtype: MemType::Normal,
            device: DeviceType::GRE,
            shareability: self.decode_shareability(sh),
            inner,
            outer,
            tags: MemTagType::Untagged,
            xs,
            notagaccess: false,
        }
    }

    /// EffectiveSCR_EL3_MECEn()
    /// ========================
    /// Returns the Effective value of SCR_EL3().MECEn.
    pub fn effective_scr_el3_mecen(&self) -> bool {
        assert!(Cpu::is_feature_implemented(Feature::MEC));
        if !self.have_el(EL3) {
            return true;
        }

        SCR_EL3Field::MECEn.fetch_bit(self.scr_el3)
    }

    /// NormalNCMemAttr()
    /// =================
    /// Normal Non-cacheable memory attributes
    pub fn normal_nc_memattr(&self) -> MemoryAttributes {
        MemoryAttributes {
            memtype: MemType::Normal,
            device: DeviceType::GRE,
            outer: MemAttrHints {
                attrs: MemAttr_NC,
                hints: 0,
                transient: false,
            },
            inner: MemAttrHints {
                attrs: MemAttr_NC,
                hints: 0,
                transient: false,
            },
            shareability: Shareability::OSH,
            tags: MemTagType::Untagged,
            notagaccess: false,
            xs: false,
        }
    }

    /// EffectiveShareability()
    /// =======================
    /// Force Outer Shareability on Device and Normal iNCoNC memory
    pub fn effective_shareability(&self, memattrs: &MemoryAttributes) -> Shareability {
        if memattrs.memtype == MemType::Device
            || (memattrs.inner.attrs == MemAttr_NC && memattrs.outer.attrs == MemAttr_NC)
        {
            Shareability::OSH
        } else {
            memattrs.shareability
        }
    }

    /// CreateAccDescS1TTW()
    /// ====================
    /// Access descriptor for stage 1 translation table walks
    pub fn create_accdesc_s1ttw(
        &self,
        toplevel: bool,
        varange: VARange,
        accdesc_in: &AccessDescriptor,
    ) -> AccessDescriptor {
        let mut accdesc = self.new_accdesc(AccessType::TTW, accdesc_in.mpam.clone());

        accdesc.el = accdesc_in.el;
        accdesc.ss = accdesc_in.ss;
        accdesc.read = true;
        accdesc.toplevel = Some(toplevel);
        accdesc.varange = Some(varange);

        accdesc
    }

    // IsWBShareable()
    // ===============
    // Determines whether the given memory attributes are iWBoWB Shareable
    pub fn is_wb_shareable(&self, memattrs: &MemoryAttributes) -> bool {
        memattrs.memtype == MemType::Normal
            && memattrs.inner.attrs == MemAttr_WB
            && memattrs.outer.attrs == MemAttr_WB
            && matches!(memattrs.shareability, Shareability::ISH | Shareability::OSH)
    }

    /// CreateAccDescTTEUpdate()
    /// ========================
    /// Access descriptor for translation table entry HW update
    pub fn create_accdesc_tte_update(&self, accdesc_in: &AccessDescriptor) -> AccessDescriptor {
        let mut accdesc = self.new_accdesc(AccessType::TTW, accdesc_in.mpam.clone());

        accdesc.el = accdesc_in.el;
        accdesc.ss = accdesc_in.ss;
        accdesc.atomicop = true;
        accdesc.modop = Some(MemAtomicOp::CAS);
        accdesc.read = accdesc_in.read;
        accdesc.write = accdesc_in.write;

        accdesc
    }

    /// TranslationSize()
    /// =================
    /// Compute the number of bits directly mapped from the input address
    /// to the output address
    pub fn translation_size(&self, d128: bool, tgx: TGx, level: u8) -> AddressSize {
        let granulebits = self.tgx_granulebits(tgx);
        let descsizelog2 = if d128 { 4u8 } else { 3u8 };
        let blockbits = (FINAL_LEVEL - level) * (granulebits - descsizelog2);

        granulebits + blockbits
    }

    /// ContiguousSize()
    /// ================
    /// Return the number of entries log 2 marking a contiguous output range
    pub fn contiguous_size(&self, d128: bool, tgx: TGx, walkstate: &TTWState) -> u8 {
        if d128 {
            match tgx {
                TGx::_4KB => {
                    assert!([1, 2, 3].contains(&walkstate.level));
                    return if walkstate.level == 1 { 2 } else { 4 };
                }
                TGx::_16KB => {
                    assert!([1, 2, 3].contains(&walkstate.level));
                    return if walkstate.level == 1 {
                        2
                    } else if walkstate.level == 2 {
                        4
                    } else {
                        6
                    };
                }
                TGx::_64KB => {
                    assert!([2, 3].contains(&walkstate.level));
                    return if walkstate.level == 2 { 6 } else { 4 };
                }
            }
        }

        match tgx {
            TGx::_4KB => {
                assert!([1, 2, 3].contains(&walkstate.level));
                return 4;
            }
            TGx::_16KB => {
                assert!([2, 3].contains(&walkstate.level));
                return if walkstate.level == 2 { 5 } else { 7 };
            }
            TGx::_64KB => {
                assert!([2, 3].contains(&walkstate.level));
                return 5;
            }
        }
    }

    /// Stage1OA()
    /// ==========
    /// Given the final walk state (a page or block descriptor), map the untranslated
    /// input address bits to the output address
    pub fn stage1oa(&self, ia: u64, walkparams: &S1TTWParams, walkstate: &TTWState) -> FullAddress {
        let tsize = self.translation_size(walkparams.d128, walkparams.tgx, walkstate.level);
        let csize = if walkstate.contiguous {
            self.contiguous_size(walkparams.d128, walkparams.tgx, walkstate)
        } else {
            0
        };

        let ia_msb = tsize + csize;
        let address =
            (walkstate.baseaddress.address >> ia_msb << ia_msb) | (ia & ones(ia_msb as u32));

        FullAddress {
            paspace: walkstate.baseaddress.paspace,
            address,
        }
    }

    /// GPCBW_EL3BWSTRIDEValid()
    /// ========================
    /// Returns whether the current GPCBW_EL3.BWSTRIDE value is valid
    pub fn gpcbw_el3bwstride_valid(&self) -> bool {
        assert!(Cpu::is_feature_implemented(Feature::RME_GPC3));
        [
            0b00000u8, 0b00010, 0b00100, 0b00110, 0b00111, 0b01000, 0b01001, 0b01010, 0b10000,
        ]
        .contains(&GPCBW_EL3Field::BWSTRIDE.fetch_range(self.gpcbw_el3))
    }

    /// DecodePPS()
    /// ===========
    /// Size of region protected by the GPT, in bits.
    pub fn decode_pps(&self) -> AddressSize {
        let merge = ((GPCCR_EL3Field::PPS3.fetch_bit(self.gpccr_el3) as u8) << 3)
            | GPCCR_EL3Field::PPS.fetch_range(self.gpccr_el3);

        match merge {
            0b0000 => 32,
            0b0001 => 36,
            0b0010 => 40,
            0b0011 => 42,
            0b0100 => 44,
            0b0101 => 48,
            0b0110 => 52,
            0b0111 => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GPC3));
                56
            }
            0b1000 => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GPC3));
                46
            }
            0b1001 => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GPC3));
                47
            }
            _ => unreachable!(),
        }
    }

    /// GPCRegistersConsistent()
    /// ========================
    /// Returns whether the GPT registers are configured correctly.
    /// This returns false if any fields select a Reserved value.
    pub fn gpc_registers_consistent(&self) -> bool {
        if Cpu::is_feature_implemented(Feature::RME_GPC3) {
            let merge = ((GPCCR_EL3Field::PPS3.fetch_bit(self.gpccr_el3) as u8) << 3)
                | GPCCR_EL3Field::PPS.fetch_range(self.gpccr_el3);

            if !matches!(merge, 0b0000..=0b0111 | 0b1000 | 0b1001) {
                return false;
            }

            if GPCCR_EL3Field::GPCBW.fetch_bit(self.gpccr_el3) {
                if !matches!(
                    GPCBW_EL3Field::BWSIZE.fetch_range(self.gpcbw_el3),
                    0b000u8 | 0b001 | 0b100 | 0b110 | 0b010
                ) {
                    return false;
                }

                if !self.gpcbw_el3bwstride_valid() {
                    return false;
                }

                let raw_stride: u8 = GPCBW_EL3Field::BWSTRIDE.fetch_range(self.gpcbw_el3);
                let bwstride = 1u64 << (40 + raw_stride);

                let raw_addr: u32 = GPCBW_EL3Field::BWADDR.fetch_range(self.gpcbw_el3);
                let bwaddr = (raw_addr as u64) << 30;

                if bwstride <= bwaddr {
                    return false;
                }

                let size: u8 = GPCBW_EL3Field::BWSIZE.fetch_range(self.gpcbw_el3);

                if !is_aligned_size(raw_addr as u64, 1 << size) {
                    return false;
                }
            }
        } else {
            if GPCCR_EL3Field::PPS.fetch_range(self.gpccr_el3) == 0b111 {
                return false;
            }
        }

        if self.decode_pps() > self.aarch64_pamax() {
            return false;
        }

        if GPCCR_EL3Field::PGS.fetch_range(self.gpccr_el3) == 0b11 {
            return false;
        }

        if GPCCR_EL3Field::SH.fetch_range(self.gpccr_el3) == 0b01 {
            return false;
        }

        if GPCCR_EL3Field::ORGN.fetch_range(self.gpccr_el3) == 0b00
            && GPCCR_EL3Field::IRGN.fetch_range(self.gpccr_el3) == 0b00
            && GPCCR_EL3Field::SH.fetch_range(self.gpccr_el3) != 0b10
        {
            return false;
        }

        true
    }

    /// AbovePPS()
    /// ==========
    /// Returns TRUE if an address exceeds the range configured in GPCCR_EL3.PPS.
    pub fn above_pps(&self, address: u64) -> bool {
        let pps = self.decode_pps();
        if pps >= NUM_PABITS {
            return false;
        }

        return get_bits(address, (NUM_PABITS - 1) as usize, pps as usize) != 0;
    }

    /// PAWithinGPCBypassWindow()
    /// =========================
    /// Check if the supplied address is within a GPC Bypass window.
    pub fn pa_within_gpc_bypass_window(&self, pa_in: u64) -> bool {
        let pa = get_bits(pa_in, 55, 30);

        let raw_gpcbwl: u8 = GPCBW_EL3Field::BWSIZE.fetch_range(self.gpcbw_el3);
        let raw_gpcbwu: u8 = GPCBW_EL3Field::BWSTRIDE.fetch_range(self.gpcbw_el3);

        let gpcbwl = raw_gpcbwl as usize;
        let gpcbwu = 9 + raw_gpcbwu as usize;

        let addr: u32 = GPCBW_EL3Field::BWADDR.fetch_range(self.gpccr_el3);

        return get_bits(pa, gpcbwu, gpcbwl) == get_bits(addr, gpcbwu, gpcbwl);
    }

    /// DecodePGS()
    /// ===========
    pub fn decode_pgs(&self, pgs: u8) -> PGSe {
        match pgs {
            0b00 => PGSe::_4KB,
            0b01 => PGSe::_16KB,
            0b10 => PGSe::_64KB,
            _ => unreachable!(),
        }
    }

    /// GPTL0Size()
    /// ===========
    /// Returns number of bits covered by a level 0 GPT entry
    pub fn gptl0_size(&self) -> AddressSize {
        match GPCCR_EL3Field::L0GPTSZ.fetch_range(self.gpccr_el3) {
            0b0000 => GPTRange_1GB,
            0b0100 => GPTRange_16GB,
            0b0110 => GPTRange_64GB,
            0b1001 => GPTRange_512GB,
            _ => unreachable!(),
        }
    }

    /// CreateAccDescGPTW()
    /// ===================
    /// Access descriptor for Granule Protection Table walks
    pub fn create_accdesc_gptw(&self, accdesc_in: &AccessDescriptor) -> AccessDescriptor {
        let mut accdesc = self.new_accdesc(AccessType::GPTW, accdesc_in.mpam.clone());

        accdesc.el = accdesc_in.el;
        accdesc.ss = accdesc_in.ss;
        accdesc.read = true;

        accdesc
    }

    /// GPTLevel0Index()
    /// ================
    /// Compute the level 0 index based on input PA.
    pub fn gpt_level0_index(&self, pa: u64) -> u64 {
        let pps = self.decode_pps();
        let l0sz = self.gptl0_size();

        if pps <= l0sz {
            return 0;
        }

        get_bits(pa, (pps - 1) as usize, l0sz as usize) as u64
    }

    /// GPIValid()
    /// ==========
    /// Returns whether a given value is a valid encoding for a GPI value
    pub fn gpi_valid(&self, gpi: u8) -> bool {
        match gpi {
            GPT_NoAccess => true,
            GPT_NonSecureProtected => {
                Cpu::is_feature_implemented(Feature::RME_GDI)
                    && GPCCR_EL3Field::NSP.fetch_bit(self.gpccr_el3)
            }
            GPT_SystemAgent => {
                Cpu::is_feature_implemented(Feature::RME_GDI)
                    && GPCCR_EL3Field::SA.fetch_bit(self.gpccr_el3)
            }
            GPT_NA6 => {
                Cpu::is_feature_implemented(Feature::RME_GDI)
                    && GPCCR_EL3Field::NA6.fetch_bit(self.gpccr_el3)
            }
            GPT_NA7 => {
                Cpu::is_feature_implemented(Feature::RME_GDI)
                    && GPCCR_EL3Field::NA7.fetch_bit(self.gpccr_el3)
            }
            GPT_Secure => Cpu::is_feature_implemented(Feature::SEL2),
            GPT_NonSecure => true,
            GPT_Realm => true,
            GPT_Root => true,
            GPT_NonSecureOnly => {
                Cpu::is_feature_implemented(Feature::RME_GPC2)
                    && GPCCR_EL3Field::NSO.fetch_bit(self.gpccr_el3)
            }
            GPT_Any => true,
            _ => false,
        }
    }

    /// GPTBlockDescriptorValid()
    /// =========================
    /// Returns TRUE if the given GPT Block descriptor is valid, and FALSE otherwise.
    pub fn gpt_block_descriptor_valid(&self, level_0_entry: u64) -> bool {
        assert!(get_bits(level_0_entry, 3, 0) as u8 == GPT_Block);
        get_bits(level_0_entry, 63, 8) == 0 && self.gpi_valid(get_bits(level_0_entry, 7, 4) as u8)
    }

    /// DecodePGSRange()
    /// ================
    pub fn decode_pgs_range(&self, pgs: PGSe) -> AddressSize {
        match pgs {
            PGSe::_4KB => GPTRange_4KB,
            PGSe::_16KB => GPTRange_16KB,
            PGSe::_64KB => GPTRange_64KB,
        }
    }

    /// GPTTableDescriptorValid()
    /// =========================
    /// Returns TRUE if the given GPT Table descriptor is valid, and FALSE otherwise.
    pub fn gpt_table_descriptor_valid(&self, level_0_entry: u64) -> bool {
        assert!(get_bits(level_0_entry, 3, 0) as u8 == GPT_Table);

        let l0sz = self.gptl0_size();
        let pgs = self.decode_pgs(GPCCR_EL3Field::PGS.fetch_range(self.gpccr_el3));
        let p = self.decode_pgs_range(pgs);

        let top = if self.decode_pps() == 56 { 56 } else { 52 } as usize;

        get_bits(level_0_entry, 63, top) == 0
            && get_bits(level_0_entry, 11, 4) == 0
            && get_bits(level_0_entry, (l0sz - p) as usize, 12) == 0
    }

    /// GPTLevel0EntryValid()
    /// =====================
    /// Returns TRUE if the given level 0 gpt descriptor is valid, and FALSE otherwise.
    pub fn gpt_level0_entry_valid(&self, gpt_entry: u64) -> bool {
        match get_bits(gpt_entry, 3, 0) as u8 {
            GPT_Block => self.gpt_block_descriptor_valid(gpt_entry),
            GPT_Table => self.gpt_table_descriptor_valid(gpt_entry),
            _ => unreachable!(),
        }
    }

    /// DecodeGPTBlock()
    /// ================
    /// Decode a GPT Block descriptor.
    pub fn decode_gpt_block(&self, pgs: PGSe, gpt_entry: u64) -> GPTEntry {
        assert!(get_bits(gpt_entry, 3, 0) as u8 == GPT_Block);

        GPTEntry {
            gpi: get_bits(gpt_entry, 7, 4) as u8,
            level: 0,
            istable: false,
            size: match pgs {
                PGSe::_4KB => GPTRange_4KB,
                PGSe::_16KB => GPTRange_16KB,
                PGSe::_64KB => GPTRange_64KB,
            },
            contig_size: self.gptl0_size(),
            pa: 0,
        }
    }

    /// DecodeGPTTable()
    /// ================
    /// Decode a GPT Table descriptor.
    pub fn decode_gpt_table(&self, pgs: PGSe, gpt_entry: u64) -> GPTTable {
        assert!(get_bits(gpt_entry, 3, 0) as u8 == GPT_Table);

        let s = match self.gptl0_size() {
            GPTRange_1GB => 30,
            GPTRange_16GB => 34,
            GPTRange_64GB => 36,
            GPTRange_512GB => 39,
            _ => unreachable!(),
        } as u32;

        let p = match pgs {
            PGSe::_4KB => 12,
            PGSe::_16KB => 14,
            PGSe::_64KB => 16,
        } as u32;

        let align_bits = (s - p) - 1;

        GPTTable {
            address: align_down_p2::<64>(gpt_entry, align_bits),
        }
    }

    /// GPTLevel1Index()
    /// ================
    /// Compute the level 1 index based on input PA.
    pub fn gpt_level1_index(&self, pa: u64) -> u64 {
        let l0sz = self.gptl0_size();

        (match self.decode_pgs(GPCCR_EL3Field::PGS.fetch_range(self.gpccr_el3)) {
            PGSe::_4KB => get_bits(pa, (l0sz - 1) as usize, 16),
            PGSe::_16KB => get_bits(pa, (l0sz - 1) as usize, 18),
            PGSe::_64KB => get_bits(pa, (l0sz - 1) as usize, 20),
        }) as u64
    }

    /// GPTContigDescriptorValid()
    /// ==========================
    /// Returns TRUE if the given GPT Contiguous descriptor is valid, and FALSE otherwise.
    pub fn gpt_contig_descriptor_valid(&self, level_1_entry: u64) -> bool {
        assert!(get_bits(level_1_entry, 3, 0) as u8 == GPT_Contig);

        get_bits(level_1_entry, 63, 10) == 0
            && get_bits(level_1_entry, 9, 8) != 0
            && self.gpi_valid(get_bits(level_1_entry, 7, 4) as u8)
    }

    /// GPTGranulesDescriptorValid()
    /// ============================
    /// Returns TRUE if the given GPT Granules descriptor is valid, and FALSE otherwise.
    pub fn gpt_granules_descriptor_valid(&self, level_1_entry: u64) -> bool {
        for i in 0..=15 {
            if !self.gpi_valid(get_bits(level_1_entry, i * 4, i * 4 + 4) as u8) {
                return false;
            }
        }

        true
    }

    /// GPTLevel1EntryValid()
    /// =====================
    /// Returns TRUE if the given level 1 gpt descriptor is valid, and FALSE otherwise.
    pub fn gpt_level1_entry_valid(&self, gpt_entry: u64) -> bool {
        match get_bits(gpt_entry, 3, 0) as u8 {
            GPT_Contig => self.gpt_contig_descriptor_valid(gpt_entry),
            _ => self.gpt_granules_descriptor_valid(gpt_entry),
        }
    }

    /// DecodeGPTContiguous()
    /// =====================
    /// Decode a GPT Contiguous descriptor.
    pub fn decode_gpt_contiguous(&self, pgs: PGSe, gpt_entry: u64) -> GPTEntry {
        assert!(get_bits(gpt_entry, 3, 0) as u8 == GPT_Contig);

        GPTEntry {
            gpi: get_bits(gpt_entry, 7, 4) as u8,
            size: match pgs {
                PGSe::_4KB => GPTRange_4KB,
                PGSe::_16KB => GPTRange_16KB,
                PGSe::_64KB => GPTRange_64KB,
            },
            contig_size: match get_bits(gpt_entry, 9, 8) as u8 {
                0b01 => GPTRange_2MB,
                0b10 => GPTRange_32MB,
                0b11 => GPTRange_512MB,
                _ => unreachable!(),
            },
            level: 1,
            istable: false,
            pa: 0,
        }
    }

    /// GPIIndex()
    /// ==========
    pub fn gpi_index(&self, pa: u64) -> u8 {
        (match self.decode_pgs(GPCCR_EL3Field::PGS.fetch_range(self.gpccr_el3) as u8) {
            PGSe::_4KB => get_bits(pa, 15, 12),
            PGSe::_16KB => get_bits(pa, 17, 14),
            PGSe::_64KB => get_bits(pa, 19, 16),
        }) as u8
    }

    /// DecodeGPTGranules()
    /// ===================
    /// Decode a GPT Granules descriptor.
    pub fn decode_gpt_granules(&self, pgs: PGSe, index: u8, gpt_entry: u64) -> GPTEntry {
        let size = match pgs {
            PGSe::_4KB => GPTRange_4KB,
            PGSe::_16KB => GPTRange_16KB,
            PGSe::_64KB => GPTRange_64KB,
        };

        GPTEntry {
            gpi: get_bits(gpt_entry, index as usize * 4, index as usize * 4 + 4) as u8,
            size,
            contig_size: size,
            level: 1,
            istable: false,
            pa: 0,
        }
    }

    /// GPTWalk()
    /// =========
    /// Get the GPT entry for a given physical address, pa
    pub fn gpt_walk(&self, pa: u64, accdesc: &AccessDescriptor) -> (GPCFRecord, GPTEntry) {
        let pgs = self.decode_pgs(GPCCR_EL3Field::PGS.fetch_range(self.gpccr_el3));
        let mut base = GPTBR_EL3Field::BADDR.fetch_range(self.gptbr_el3) << 12;

        let pps = self.decode_pps();
        let l0sz = self.gptl0_size();
        let alignment = ((pps - l0sz) + 3).max(12) as AddressSize;

        base &= !ones(alignment as u32);

        let gptaccdesc = self.create_accdesc_gptw(accdesc);

        let memattrs = self.walk_mem_attrs(
            GPCCR_EL3Field::SH.fetch_range(self.gpccr_el3),
            GPCCR_EL3Field::IRGN.fetch_range(self.gpccr_el3),
            GPCCR_EL3Field::ORGN.fetch_range(self.gpccr_el3),
        );
        let fault = FaultRecord::no_fault_accdesc(&gptaccdesc);
        let paddress = FullAddress {
            paspace: PASpace::Root,
            address: base + self.gpt_level0_index(pa) * 8,
        };

        let mut gptaddrdesc = AddressDescriptor {
            memattrs,
            fault,
            paddress,
            s1assured: false,
            s2fs1mro: false,
            mecid: 0,
            vaddress: 0,
        };

        let (memstatus, raw_level_0_entry) = self.phys_mem_read::<64>(&gptaddrdesc, &gptaccdesc);
        let level_0_entry = raw_level_0_entry.compile_u128() as u64;

        if memstatus.is_fault() {
            return (GPCFRecord::fault(GPCF::EABT, 0), GPTEntry::arbitrary());
        }

        if !self.gpt_level0_entry_valid(level_0_entry) {
            return (GPCFRecord::fault(GPCF::Walk, 0), GPTEntry::arbitrary());
        }

        match get_bits(level_0_entry, 3, 0) as u8 {
            GPT_Block => {
                let mut result = self.decode_gpt_block(pgs, level_0_entry);
                result.pa = pa;
                (GPCFRecord::no_fault(), result)
            }
            GPT_Table => {
                let table = self.decode_gpt_table(pgs, level_0_entry);

                if self.above_pps(table.address) {
                    return (
                        GPCFRecord::fault(GPCF::AddressSize, 0),
                        GPTEntry::arbitrary(),
                    );
                }

                let offset = self.gpt_level1_index(pa) * 8;
                let level_1_entry = if Cpu::is_feature_implemented(Feature::RME_GDI) {
                    gptaddrdesc.paddress.address = align_down_size(table.address + offset, 16);
                    let (memstatus, raw_level_1_entry_lower) =
                        self.phys_mem_read::<64>(&gptaddrdesc, &gptaccdesc);
                    let level_1_entry_lower = raw_level_1_entry_lower.compile_u128() as u64;

                    if memstatus.is_fault() {
                        return (GPCFRecord::fault(GPCF::EABT, 1), GPTEntry::arbitrary());
                    }

                    gptaddrdesc.paddress.address += 8;
                    let (memstatus, raw_level_1_entry_upper) =
                        self.phys_mem_read::<64>(&gptaddrdesc, &gptaccdesc);
                    let level_1_entry_upper = raw_level_1_entry_upper.compile_u128() as u64;

                    if memstatus.is_fault() {
                        return (GPCFRecord::fault(GPCF::EABT, 1), GPTEntry::arbitrary());
                    }

                    if !self.gpt_level1_entry_valid(level_1_entry_upper)
                        || !self.gpt_level1_entry_valid(level_1_entry_lower)
                    {
                        return (GPCFRecord::fault(GPCF::Walk, 1), GPTEntry::arbitrary());
                    }

                    if nth_bit(offset, 3) {
                        level_1_entry_upper
                    } else {
                        level_1_entry_lower
                    }
                } else {
                    gptaddrdesc.paddress.address = table.address + offset;

                    let (memstatus, raw_l1e) = self.phys_mem_read::<64>(&gptaddrdesc, &gptaccdesc);
                    let l1e = raw_l1e.compile_u128() as u64;

                    if memstatus.is_fault() {
                        return (GPCFRecord::fault(GPCF::EABT, 1), GPTEntry::arbitrary());
                    }

                    if !self.gpt_level1_entry_valid(l1e) {
                        return (GPCFRecord::fault(GPCF::Walk, 1), GPTEntry::arbitrary());
                    }

                    l1e
                };

                let mut result = match get_bits(level_1_entry, 3, 0) as u8 {
                    GPT_Contig => self.decode_gpt_contiguous(pgs, level_1_entry),
                    _ => {
                        let gpi_index = self.gpi_index(pa);
                        self.decode_gpt_granules(pgs, gpi_index, level_1_entry)
                    }
                };

                result.pa = pa;
                (GPCFRecord::no_fault(), result)
            }
            _ => unreachable!(),
        }
    }

    /// GPICheck()
    /// ==========
    /// Returns whether an access to a given physical address space is permitted
    /// given the configured GPI value.
    /// paspace: Physical address space of the access
    /// gpi: Value read from GPT for the access
    /// ss: Security state of the access
    pub fn gpi_check(&self, paspace: PASpace, gpi: u8, ss: SecurityState) -> bool {
        match gpi {
            GPT_NoAccess => false,
            GPT_Secure => {
                assert!(Cpu::is_feature_implemented(Feature::SEL2));
                paspace == PASpace::Secure
            }
            GPT_NonSecure => paspace == PASpace::NonSecure,
            GPT_Root => paspace == PASpace::Root,
            GPT_Realm => paspace == PASpace::Realm,
            GPT_NonSecureOnly => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GPC2));
                paspace == PASpace::NonSecure
                    && matches!(ss, SecurityState::Root | SecurityState::NonSecure)
            }
            GPT_SystemAgent => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GDI));
                paspace == PASpace::SystemAgent
            }
            GPT_NonSecureProtected => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GDI));
                paspace == PASpace::NonSecureProtected
            }
            GPT_NA6 => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GDI));
                false
            }
            GPT_NA7 => {
                assert!(Cpu::is_feature_implemented(Feature::RME_GDI));
                false
            }
            GPT_Any => true,
            _ => unreachable!(),
        }
    }

    /// GranuleProtectionCheck()
    /// ========================
    /// Returns whether a given access is permitted, according to the
    /// granule protection check.
    /// addrdesc and accdesc describe the access to be checked.
    pub fn granule_protection_check(
        &self,
        addrdesc: &AddressDescriptor,
        accdesc: &AccessDescriptor,
    ) -> GPCFRecord {
        assert!(Cpu::is_feature_implemented(Feature::RME));

        let address = addrdesc.paddress;

        if !GPCCR_EL3Field::GPC.fetch_bit(self.gpccr_el3) {
            return GPCFRecord::no_fault();
        }

        if !self.gpc_registers_consistent() {
            return GPCFRecord::fault(GPCF::Walk, 0);
        }

        if Cpu::is_feature_implemented(Feature::RME_GPC2) {
            let access_disabled = match address.paspace {
                PASpace::Secure => GPCCR_EL3Field::SPAD.fetch_bit(self.gpccr_el3),
                PASpace::NonSecure => GPCCR_EL3Field::NSPAD.fetch_bit(self.gpccr_el3),
                PASpace::Realm => GPCCR_EL3Field::RLPAD.fetch_bit(self.gpccr_el3),
                PASpace::Root => false,
                _ => unreachable!(),
            };

            if access_disabled {
                return GPCFRecord::fault(GPCF::Fail, 0);
            }
        }

        if self.above_pps(address.address) {
            if address.paspace == PASpace::NonSecure
                || (Cpu::is_feature_implemented(Feature::RME_GPC2)
                    && GPCCR_EL3Field::APPSAA.fetch_bit(self.gpccr_el3))
            {
                return GPCFRecord::no_fault();
            } else {
                return GPCFRecord::fault(GPCF::Fail, 0);
            }
        }

        if Cpu::is_feature_implemented(Feature::RME_GPC3)
            && GPCCR_EL3Field::GPCBW.fetch_bit(self.gpccr_el3)
            && self.pa_within_gpc_bypass_window(address.address)
        {
            return GPCFRecord::no_fault();
        }

        let gpt_base = GPTBR_EL3Field::BADDR.fetch_range(self.gptbr_el3) << 12;
        if self.above_pps(gpt_base) {
            return GPCFRecord::fault(GPCF::AddressSize, 0);
        }

        let (mut gpcf, gpt_entry) = self.gpt_walk(address.address, accdesc);
        if gpcf.gpf != GPCF::None {
            return gpcf;
        }

        let permitted = self.gpi_check(address.paspace, gpt_entry.gpi, accdesc.ss);
        if !permitted {
            gpcf = GPCFRecord::fault(GPCF::Fail, gpt_entry.level);
            return gpcf;
        }

        GPCFRecord::no_fault()
    }

    /// EffectiveEA()
    /// =============
    /// Returns the Effective value of SCR_EL3().EA
    pub fn effective_ea(&self) -> bool {
        if !self.have_el(EL3) || self.halted() {
            false
        } else {
            if self.have_aarch64() {
                SCR_EL3Field::EA.fetch_bit(self.scr_el3)
            } else {
                SCRField::EA.fetch_bit(self.scr)
            }
        }
    }

    /// EffectiveHCR_TEA()
    /// ==================
    /// Return the Effective value of HCR_EL2.TEA.
    pub fn effective_hcr_tea(&self) -> bool {
        if self.el2_enabled() && Cpu::is_feature_implemented(Feature::RAS) {
            if self.el_using_aarch32(EL2) {
                HCR2Field::TEA.fetch_bit(self.hcr2)
            } else {
                HCR_EL2Field::TEA.fetch_bit(self.hcr_el2)
            }
        } else {
            false
        }
    }

    /// EffectiveTGE()
    /// ==============
    /// Returns the Effective value of TGE.
    pub fn effective_tge(&self) -> bool {
        if self.el2_enabled() {
            if self.el_using_aarch32(EL2) {
                HCRField::TGE.fetch_bit(self.hcr)
            } else {
                HCR_EL2Field::TGE.fetch_bit(self.hcr_el2)
            }
        } else {
            false
        }
    }

    /// IsHCRXEL2Enabled()
    /// ==================
    /// Returns TRUE if access to HCRX_EL2 register is enabled, and FALSE otherwise.
    /// Indirect read of HCRX_EL2 returns 0 when access is not enabled.
    pub fn is_hcrx_el2_enabled(&self) -> bool {
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

    /// EffectiveHCRX_EL2_TMEA()
    /// ========================
    /// Return the Effective value of HCRX_EL2.TMEA.
    pub fn effective_hcrx_el2_tmea(&self) -> bool {
        if Cpu::is_feature_implemented(Feature::DoubleFault2)
            && self.el2_enabled()
            && !self.el_using_aarch32(EL2)
            && self.is_hcrx_el2_enabled()
        {
            HCRX_EL2Field::TMEA.fetch_bit(self.hcrx_el2)
        } else {
            false
        }
    }

    /// EffectiveSCR_EL3_TMEA()
    /// =======================
    /// Returns the Effective value of SCR_EL3().TMEA
    pub fn effective_scr_el3_tmea(&self) -> bool {
        if Cpu::is_feature_implemented(Feature::DoubleFault2)
            && self.have_el(EL3)
            && !self.el_using_aarch32(EL3)
        {
            SCR_EL3Field::TMEA.fetch_bit(self.scr_el3)
        } else {
            false
        }
    }

    /// SyncExternalAbortTarget()
    /// =========================
    /// Returns the target Exception level for a Synchronous External Data or
    /// Instruction or Prefetch Abort.
    /// If EL3 is implemented and using AArch32, then a target Exception level of
    /// EL1 means Abort mode, and EL3 means Monitor mode, including in Secure
    /// state when Abort mode is part of EL3.
    pub fn sync_external_abort_target(&self, fault: &FaultRecord) -> EL {
        let effective_ea = self.effective_ea();
        let effective_tea = self.effective_hcr_tea();
        let effective_tge = self.effective_tge();

        if effective_ea || (self.pstate.EL() == EL3 && !self.el_using_aarch32(EL3)) {
            EL3
        } else if self.el2_enabled()
            && (self.pstate.EL() == EL1 || self.pstate.EL() == EL0)
            && (effective_tea
                || fault.is_second_stage()
                || fault.accessdesc.acctype == AccessType::NV2
                || (self.pstate.EL() == EL0 && effective_tge))
        {
            EL2
        } else if self.effective_hcrx_el2_tmea() && self.pstate.A && self.pstate.EL() == EL1 {
            EL2
        } else if self.effective_scr_el3_tmea()
            && self.pstate.A
            && (self.pstate.EL() == EL1 || self.pstate.EL() == EL2)
        {
            EL3
        } else {
            assert!(self.pstate.EL() != EL3 || self.el_using_aarch32(EL3));
            if self.pstate.EL() == EL2 { EL2 } else { EL1 }
        }
    }

    /// ExtAbortToAArch64()
    /// ===================
    /// Returns TRUE if synchronous exception is being taken to an Exception level using AArch64_
    pub fn ext_abort_to_aarch64(&self, fault: &FaultRecord) -> bool {
        assert!(fault.statuscode.is_external_sync_abort());

        !self.el_using_aarch32(self.sync_external_abort_target(fault))
    }

    /// PEErrorState()
    /// ==============
    /// Returns the error state of the PE on taking an error exception:
    /// The PE error state reported to software through the exception syndrome also
    /// depends on how the exception is taken, and so might differ from the value
    /// returned from this function.    
    pub fn peerror_state(&self, fault: &FaultRecord) -> ErrorState {
        // TODO: Impdef stuff
        ErrorState::UEO
    }

    /// HandleExternalTTWAbort()
    /// ========================
    /// Take Asynchronous abort or update FaultRecord for Translation Table Walk
    /// based on PhysMemRetStatus.
    pub fn handle_external_ttw_abort(
        &self,
        memretstatus: &PhysMemRetStatus,
        memaddrdesc: &AddressDescriptor,
        input_fault: &FaultRecord,
    ) -> FaultRecord {
        let mut output_fault = input_fault.clone();
        output_fault.extflag = memretstatus.extflag;
        output_fault.statuscode = memretstatus.statuscode.clone();

        if output_fault.is_external_sync_abort()
            && (Cpu::is_feature_implemented(Feature::RASv2)
                && self.ext_abort_to_aarch64(&output_fault)
                && matches!(
                    self.peerror_state(&output_fault),
                    ErrorState::UC | ErrorState::UEU
                ))
        {
            if output_fault.statuscode == Fault::SyncParity {
                output_fault.statuscode = Fault::AsyncParity;
            } else {
                output_fault.statuscode = Fault::AsyncExternal;
            }
        }

        if output_fault.is_external_sync_abort() {
            if output_fault.statuscode == Fault::SyncParity {
                output_fault.statuscode = Fault::SyncParityOnWalk;
            } else {
                output_fault.statuscode = Fault::SyncExternalOnWalk;
            }
        }

        if Cpu::is_feature_implemented(Feature::RAS) {
            output_fault.merrorstate = memretstatus.merrorstate.clone().unwrap();
        }

        if !output_fault.is_external_sync_abort() {
            output_fault.statuscode = Fault::None;
        }

        output_fault.paddress = memaddrdesc.paddress;
        output_fault
    }

    /// IsWatchpointableAccess()
    /// ========================
    /// Return TRUE if access should be checked for watchpoints.
    pub fn is_watchpointable_access(&self, accdesc: &AccessDescriptor) -> bool {
        // TODO: Impdef
        return !matches!(
            accdesc.acctype,
            AccessType::IFETCH
                | AccessType::TTW
                | AccessType::DC
                | AccessType::IC
                | AccessType::SPE
                | AccessType::TRBE
                | AccessType::AT
        ) || (accdesc.acctype == AccessType::DC
            && accdesc.cacheop == Some(CacheOp::Invalidate)
            && !self.using_aarch32());
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
