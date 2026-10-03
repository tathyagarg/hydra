use crate::cpu::{
    BranchType, Cpu, ExtendType, ShiftType,
    brbe::{BRBSRC_EL1_Type, BRBTGT_EL1_Type},
    common::*,
    pstate::PState,
    regs::{
        brbcr_el1::BRBCR_EL1Field, brbcr_el2::BRBCR_EL2Field, brbfcr_el0::BRBFCR_EL0Field,
        brbfcr_el1::BRBFCR_EL1Field, hcr_el2::HCR_EL2Field, mdcr_el3::MDCR_EL3Field, scr::SCRField,
        scr_el3::SCR_EL3Field, tcr_el1::TCR_EL1Field,
    },
};

pub const EL0: u8 = 0b00;
pub const EL1: u8 = 0b01;
pub const EL2: u8 = 0b10;
pub const EL3: u8 = 0b11;

#[derive(PartialEq)]
pub enum SecurityState {
    NonSecure,
    Root,
    Realm,
    Secure,
}

pub struct NZCV(bool, bool, bool, bool);

impl NZCV {
    pub(super) fn set(&self, pstate: &mut PState) {
        pstate.N = self.0;
        pstate.Z = self.1;
        pstate.C = self.2;
        pstate.V = self.3;
    }
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

fn ror(val: u64, amount: u32, width: u32) -> u64 {
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
pub(super) fn sint<const N: usize>(n: u64) -> i64 {
    let mask = if N == 64 { u64::MAX } else { (1u64 << N) - 1 };
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
        let (temp, c1) = x.overflowing_add(y);
        println!("temp: {}, c1: {}", temp, c1);
        let (temp2, c2) = temp.overflowing_add(carry_in as u64);
        (temp2, c1 || c2)
    };
    let signed_sum = sint::<N>(x) + sint::<N>(y) + carry_in as i64;

    let result = unsigned_sum & mask;

    let n = (result >> (N - 1)) & 1 == 1;
    let z = result == 0;
    let c = carry;
    let v = result as i64 != signed_sum;

    println!("nzcv: {} {} {} {}", n, z, c, v);
    println!("unsigned_sum: {}, signed_sum: {}", unsigned_sum, signed_sum);

    (result, NZCV(n, z, c, v))
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
    println!("d: {}", d);

    println!("s: {}, 1 << (s + 1): {}", s, (1u64 << (s + 1)) - 1);
    let welem = (1u64 << (s + 1)) - 1;
    let telem = (1u64 << (d + 1)) - 1;
    println!("welem telem {} {}", welem, telem);
    println!("esize: {}", esize);

    let wmask = replicate(ror(welem, r as u32, esize), esize, M as u32);
    let tmask = replicate(telem, esize as u32, M as u32);

    (wmask, tmask)
}

impl Cpu {
    pub fn imp_def_int(&self, name: &str) -> u64 {
        match name {
            "Number of BRBE Branch records" => 32,
            _ => unimplemented!(),
        }
    }

    pub fn condition_holds(&self, cond: u8) -> bool {
        let result = match cond >> 1 {
            0b000 => self.pstate.Z,
            0b001 => self.pstate.C,
            0b010 => self.pstate.N,
            0b011 => self.pstate.V,
            0b100 => self.pstate.C && !self.pstate.Z,
            0b101 => self.pstate.N == self.pstate.V,
            0b110 => self.pstate.N == self.pstate.V && !self.pstate.Z,
            0b111 => true,
            _ => unreachable!(),
        };

        if cond & 1 == 1 && cond != 0b1111 {
            !result
        } else {
            result
        }
    }

    pub fn branch_not_taken(&self, branch_type: BranchType, branch_conditional: bool) {
        if Cpu::FEAT_SPE {
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
            self.pc = target;
        } else {
            let target_vaddress = self.aarch64_branchaddr(target, self.pstate.EL);
            if Cpu::FEAT_BRBE
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

            if Cpu::FEAT_SPE {
                todo!()
            }

            self.pc = target_vaddress;
        }
    }

    pub fn security_state_at_el(&self, el: u8) -> SecurityState {
        if Cpu::FEAT_RME {
            if el == EL3 {
                return SecurityState::Root;
            }

            let nse = self.effective_scr_el3_nse() as u8;
            let ns = self.effective_scr_el3_ns() as u8;
            let effective_nse_ns = nse << 1 | ns;

            match effective_nse_ns {
                0b00 => {
                    if !Cpu::FEAT_SEL2 {
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
        self.security_state_at_el(self.pstate.EL)
    }

    pub fn branch_record_allowed(&self, el: u8) -> bool {
        if self.el_using_aarch32(el) {
            return false;
        }

        if BRBFCR_EL0Field::PAUSED.fetch_bit(self.brbfcr_el0) {
            return false;
        }

        if el == EL3 && Cpu::FEAT_BRBEv1p1 {
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
        el: u8,
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

        if self.have_aarch64() && Cpu::FEAT_PMUv3_ICNTR && pmuevent == Cpu::PMU_EVENT_INST_RETIRED {
            todo!()
        }
    }

    pub fn pmu_event3(&self, pmuevent: u16, increment: usize, idx: usize) {
        if !Cpu::FEAT_PMUv3 {
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
        if self.branch_record_allowed(self.pstate.EL) && self.filter_branch_record(&br_type, cond) {
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
            let el = self.pstate.EL;

            let mispredict = self.brbe_mispredict_allowed() && self.branch_mispredict();

            self.update_branch_record_buffer(
                ccu,
                cc,
                branch_type,
                el,
                mispredict,
                0b11,
                self.pc,
                target_address,
            );

            self.pmu_event1(Cpu::PMU_EVENT_BRB_FILTRATE);
        }
    }

    pub fn aarch64_branchaddr(&self, vaddress: u64, el: u8) -> u64 {
        let msbit = self.addrtop(vaddress, true, el);
        if msbit == 63 {
            return vaddress;
        } else if ((el == EL0 || el == EL1) || self.el_is_in_host(el)) && nth_bit(vaddress, msbit) {
            return sint::<64>(vaddress & ((1 << msbit) - 1)) as u64;
        } else {
            return vaddress & ((1 << msbit) - 1);
        }
    }

    pub fn have_aarch32_el(&self, el: u8) -> bool {
        match el {
            EL0 => Cpu::FEAT_AA32EL0,
            EL1 => Cpu::FEAT_AA32EL1,
            EL2 => Cpu::FEAT_AA32EL2,
            EL3 => Cpu::FEAT_AA32EL3,
            _ => unreachable!(),
        }
    }

    pub fn have_aarch64(&self) -> bool {
        Cpu::FEAT_AA64EL0 || Cpu::FEAT_AA64EL1 || Cpu::FEAT_AA64EL2 || Cpu::FEAT_AA64EL3
    }

    pub fn have_el(&self, el: u8) -> bool {
        match el {
            EL0 | EL1 => true,
            EL2 => Cpu::FEAT_AA64EL2 || Cpu::FEAT_AA32EL2,
            EL3 => Cpu::FEAT_AA64EL3 || Cpu::FEAT_AA32EL3,
            _ => unreachable!(),
        }
    }

    pub fn have_secure_state(&self) -> bool {
        Cpu::FEAT_SECURE
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
        if !Cpu::FEAT_RME {
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

            if Cpu::FEAT_SEL2
                && SCR_EL3Field::EEL2.fetch_bit(self.scr_el3)
                && !self.effective_scr_el3_ns()
            {
                return true;
            }
        }

        SCR_EL3Field::RW.fetch_bit(self.scr_el3)
    }

    pub fn secure_only_implementation(&self) -> bool {
        return Cpu::FEAT_SECURE && !self.have_el(EL3);
    }

    pub fn is_secure_el2_enabled(&self) -> bool {
        if self.have_el(EL2) && Cpu::FEAT_SEL2 {
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
        if !Cpu::FEAT_VHE {
            return false;
        }

        if !Cpu::FEAT_E2H0 {
            return true;
        }

        HCR_EL2Field::E2H.fetch_bit(self.hcr_el2)
    }

    pub fn el_is_in_host(&self, el: u8) -> bool {
        if !Cpu::FEAT_VHE || self.el_using_aarch32(EL2) {
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

    pub fn el_state_using_aarch32k(&self, el: u8, secure: bool) -> (bool, bool) {
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
            && (!secure || !Cpu::FEAT_SEL2 || !SCR_EL3Field::EEL2.fetch_bit(self.scr_el3))
        {
            return (true, true);
        }

        if el == EL2 {
            return (true, false);
        }

        if self.have_el(EL2)
            && self.el_is_in_host(EL0)
            && !HCR_EL2Field::RW.fetch_bit(self.hcr_el2)
            && (!secure || (Cpu::FEAT_SEL2) && SCR_EL3Field::EEL2.fetch_bit(self.scr_el3))
        {
            return (true, true);
        }

        if el == EL1 {
            return (true, false);
        }

        if self.pstate.EL == EL0 {
            return (true, self.pstate.nRW);
        } else {
            return (false, false);
        }
    }

    pub fn el_state_using_aarch32(&self, el: u8, secure: bool) -> bool {
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
        } else if self.have_el(EL2) && (!Cpu::FEAT_SEL2 || !self.have_aarch64()) {
            return false;
        } else {
            return self.secure_only_implementation();
        }
    }

    pub fn el_using_aarch32(&self, el: u8) -> bool {
        self.el_state_using_aarch32(el, self.is_secure_below_el3())
    }

    pub fn addrtop(&self, address: u64, isinstr: bool, el: u8) -> usize {
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

    pub fn effective_tbi(&self, address: u64, isinstr: bool, el: u8) -> bool {
        let regime = self.s1translationregime(el);

        let (tbi, tbid): (bool, Option<bool>) = match regime {
            EL1 => {
                let tbi = if nth_bit(address, 55) {
                    TCR_EL1Field::TBI1.fetch_bit(self.tcr_el1)
                } else {
                    TCR_EL1Field::TBI0.fetch_bit(self.tcr_el1)
                };

                let tbid = if Cpu::FEAT_PAuth {
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

        tbi && (!Cpu::FEAT_PAuth || tbid == Some(false) || !isinstr)
    }

    pub fn s1translationregime(&self, el: u8) -> u8 {
        if el != EL0 {
            return el;
        }

        todo!("uhhh");
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
