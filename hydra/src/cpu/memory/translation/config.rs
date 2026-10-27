use crate::cpu::{
    Cpu, Feature,
    common::*,
    fault::{AccessType, VARange},
    memory::types::{AddressSize, DEFAULT_MECID, S1TTWParams, TGx},
    regs::{
        E2H0_TCR_EL2Field, E2H1_TCR_EL2Field, HCR_EL2Field, MECID_P0_EL2Field, MECID_P1_EL2Field,
        RegisterField, RegisterFieldRange, S1PIRType, SCR_EL3Field, SCTLR_EL1Field, SCTLR_EL2Field,
        SCTLR_EL3Field, SCTLR2_EL2Field, SCTLR2_EL3Field, SCTLRField, TCR_EL1Field,
        TCR_EL2CommonField, TCR_EL3Field, TCR2_EL2Field, VMECID_P_EL2Field, mairtype_from_mair,
    },
    shared::{EL, EL3, Regime, SecurityState},
};

impl Cpu {
    pub fn aarch64_s1enabled(&self, regime: Regime, acctype: AccessType) -> bool {
        // TODO: Effective TRBLIMITR yada yada
        if acctype == AccessType::TRBE {
            return false;
        }

        // TODO: Effective PMBLIMITR yada yada
        if acctype == AccessType::SPE {
            return false;
        }

        match regime {
            Regime::EL3 => SCTLRField::M.fetch_bit(self.sctlr_el3),
            Regime::EL2 | Regime::EL20 => SCTLRField::M.fetch_bit(self.sctlr_el2),
            Regime::EL10 => {
                (!self.el2_enabled()
                    || (!HCR_EL2Field::DC.fetch_bit(self.hcr_el2)
                        && !HCR_EL2Field::TGE.fetch_bit(self.hcr_el2)))
                    && SCTLRField::M.fetch_bit(self.sctlr_el1)
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_get_varange(&self, va: u64) -> VARange {
        if nth_bit(va, 55) {
            VARange::LOWER
        } else {
            VARange::UPPER
        }
    }

    pub fn aarch64_s1decode_tg0(&self, tg0_in: u8) -> TGx {
        if tg0_in == 0b11 {
            panic!("Imp def bits");
        }

        match tg0_in {
            0b00 => TGx::_4KB,
            0b01 => TGx::_64KB,
            0b10 => TGx::_16KB,
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1ttw_params_el3(&self) -> S1TTWParams {
        let tgx = self.aarch64_s1decode_tg0(TCR_EL3Field::TG0.fetch_range(self.tcr_el3));
        let txsz = TCR_EL3Field::T0SZ.fetch_range(self.tcr_el3);
        let ps = TCR_EL3Field::PS.fetch_range(self.tcr_el3);
        let irgn = TCR_EL3Field::IRGN0.fetch_range(self.tcr_el3);
        let orgn = TCR_EL3Field::ORGN0.fetch_range(self.tcr_el3);
        let sh = TCR_EL3Field::SH0.fetch_range(self.tcr_el3);
        let tbi = TCR_EL3Field::TBI.fetch_bit(self.tcr_el3);
        let mair = mairtype_from_mair(self.mair_el3);
        let skl = 0;
        let d128 = if Cpu::is_feature_implemented(Feature::D128) {
            TCR_EL3Field::D128.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let disch = if d128 {
            TCR_EL3Field::DisCH0.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pie = if d128 {
            true
        } else if Cpu::is_feature_implemented(Feature::S1PIE) {
            TCR_EL3Field::PIE.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pir = [0; 16];
        let mair2 = [0; 8];
        let aie = if Cpu::is_feature_implemented(Feature::AIE) {
            TCR_EL3Field::AIE.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let wxn = SCTLRField::WXN.fetch_bit(self.sctlr_el3);
        let ee = SCTLRField::EE.fetch_bit(self.sctlr_el3);
        let sif = if !Cpu::is_feature_implemented(Feature::RME)
            || Cpu::is_feature_implemented(Feature::SEL2)
        {
            SCR_EL3Field::SIF.fetch_bit(self.scr_el3)
        } else {
            false
        };
        let tbid = if Cpu::is_feature_implemented(Feature::PAuth) {
            TCR_EL3Field::TBID.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let mut hpd = if Cpu::is_feature_implemented(Feature::HPDS) {
            TCR_EL3Field::HPD.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        if !hpd {
            if aie {
                hpd = true;
            }
            if pie {
                hpd = true;
            }
        }
        let ha = if Cpu::is_feature_implemented(Feature::HAF) {
            TCR_EL3Field::HA.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let hd = if Cpu::is_feature_implemented(Feature::HAFDBS) && ha {
            TCR_EL3Field::HD.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let ds = if [TGx::_4KB, TGx::_16KB].contains(&tgx)
            && Cpu::is_feature_implemented(Feature::LPA2)
            && !d128
        {
            TCR_EL3Field::DS.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let mtx = if Cpu::is_feature_implemented(Feature::MTE_CANONICAL_TAGS)
            || Cpu::is_feature_implemented(Feature::MTE_NO_ADDRESS_TAGS)
        {
            TCR_EL3Field::MTX.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let pnch = if Cpu::is_feature_implemented(Feature::THE) && !d128 {
            TCR_EL3Field::PnCH.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let haft = if Cpu::is_feature_implemented(Feature::HAFT) && ha {
            TCR_EL3Field::HAFT.fetch_bit(self.tcr_el3)
        } else {
            false
        };
        let emec = if Cpu::is_feature_implemented(Feature::MEC) {
            SCTLR2_EL3Field::EMEC.fetch_bit(self.sctlr2_el3)
        } else {
            false
        };

        S1TTWParams {
            tgx,
            txsz,
            ps,
            irgn,
            orgn,
            sh,
            tbi,
            mair,
            d128,
            skl,
            disch,
            pie,
            pir,
            mair2,
            aie,
            wxn,
            ee,
            sif,
            tbid,
            hpd,
            ha,
            hd,
            ds,
            mtx,
            pnch,
            haft,
            emec,
            nfd: false,
            amec: false,
            cmow: false,
            dc: false,
            dct: false,
            e0pd: false,
            epan: false,
            nv1: false,
            fng: false,
            fngna: false,
            ntlsmd: false,
            pire0: [0; 16],
            t0sz: 0,
            t1sz: 0,
            uwxn: false,
        }
    }

    pub fn aarch64_s1ttw_params_el2(&self, ss: SecurityState) -> S1TTWParams {
        let tgx = self.aarch64_s1decode_tg0(TCR_EL2CommonField::TG0.fetch_range(self.tcr_el2));
        let txsz = TCR_EL2CommonField::T0SZ.fetch_range(self.tcr_el2);
        let ps = E2H0_TCR_EL2Field::PS.fetch_range(self.tcr_el2);
        let irgn = TCR_EL2CommonField::IRGN0.fetch_range(self.tcr_el2);
        let orgn = TCR_EL2CommonField::ORGN0.fetch_range(self.tcr_el2);
        let sh = TCR_EL2CommonField::SH0.fetch_range(self.tcr_el2);
        let tbi = E2H0_TCR_EL2Field::TBI.fetch_bit(self.tcr_el2);
        let mair = mairtype_from_mair(self.mair_el2);
        let pie = if Cpu::is_feature_implemented(Feature::S1PIE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::PIE.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let mut pir = [0u8; 16] as S1PIRType;
        if Cpu::is_feature_implemented(Feature::S1PIE) {
            if !(self.have_el(EL3) && !SCR_EL3Field::PIEn.fetch_bit(self.scr_el3)) {
                pir = self.pir_el2 as S1PIRType;
            } else {
                pir = [0u8; 16] as S1PIRType;
            }
        }

        // TODO: MAIR2_EL2
        let mair2 = [0; 8];

        let aie = if Cpu::is_feature_implemented(Feature::AIE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::AIE.fetch_bit(self.tcr2_el2)
        } else {
            false
        };

        let wxn = SCTLRField::WXN.fetch_bit(self.sctlr_el2);
        let ee = SCTLRField::EE.fetch_bit(self.sctlr_el2);
        let sif = if self.have_el(EL3)
            && (!Cpu::is_feature_implemented(Feature::RME)
                || Cpu::is_feature_implemented(Feature::SEL2))
        {
            SCR_EL3Field::SIF.fetch_bit(self.scr_el3)
        } else {
            false
        };

        let tbid = if Cpu::is_feature_implemented(Feature::PAuth) {
            E2H0_TCR_EL2Field::TBID.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let mut hpd = if Cpu::is_feature_implemented(Feature::HPDS) {
            E2H0_TCR_EL2Field::HPD.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        if !hpd {
            if aie {
                hpd = true;
            }

            if pie {
                hpd = true;
            }
        }

        let ha = if Cpu::is_feature_implemented(Feature::HAF) {
            E2H0_TCR_EL2Field::HA.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let hd = if Cpu::is_feature_implemented(Feature::HAFDBS) && ha {
            E2H0_TCR_EL2Field::HD.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let ds = if [TGx::_4KB, TGx::_16KB].contains(&tgx)
            && Cpu::is_feature_implemented(Feature::LPA2)
        {
            E2H0_TCR_EL2Field::DS.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let mtx = if Cpu::is_feature_implemented(Feature::MTE_NO_ADDRESS_TAGS)
            || Cpu::is_feature_implemented(Feature::MTE_CANONICAL_TAGS)
        {
            E2H0_TCR_EL2Field::MTX.fetch_bit(self.tcr_el2)
        } else {
            false
        };
        let pnch = if Cpu::is_feature_implemented(Feature::THE) && self.is_tcr2el2_enabled() {
            TCR2_EL2Field::PnCH.fetch_bit(self.tcr2_el2)
        } else {
            false
        };
        let haft = if Cpu::is_feature_implemented(Feature::HAFT) && ha && self.is_tcr2el2_enabled()
        {
            TCR2_EL2Field::HAFT.fetch_bit(self.tcr_el2)
        } else {
            false
        };

        let emec = if Cpu::is_feature_implemented(Feature::MEC) && self.is_sctlr2el2_enabled() {
            SCTLR2_EL2Field::EMEC.fetch_bit(self.sctlr2_el2)
        } else {
            false
        };

        let amec = if Cpu::is_feature_implemented(Feature::MEC)
            && ss == SecurityState::Realm
            && self.is_tcr2el2_enabled()
        {
            TCR2_EL2Field::AMEC0.fetch_bit(self.tcr2_el2)
        } else {
            false
        };

        S1TTWParams {
            tgx,
            txsz,
            ps,
            irgn,
            orgn,
            sh,
            tbi,
            mair,
            d128: false,
            skl: 0,
            disch: false,
            pie,
            pir,
            mair2,
            aie,
            wxn,
            ee,
            sif,
            tbid,
            hpd,
            ha,
            hd,
            ds,
            mtx,
            pnch,
            haft,
            emec,
            nfd: false,
            amec,
            cmow: false,
            dc: false,
            dct: false,
            e0pd: false,
            epan: false,
            nv1: false,
            fng: false,
            fngna: false,
            ntlsmd: false,
            pire0: [0; 16],
            t0sz: 0,
            t1sz: 0,
            uwxn: false,
        }
    }

    #[allow(unused_variables)]
    pub fn aarch64_get_s1ttw_params(
        &self,
        regime: Regime,
        el: EL,
        ss: SecurityState,
        va: u64,
    ) -> S1TTWParams {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL3 => self.aarch64_s1ttw_params_el3(),
            Regime::EL2 => self.aarch64_s1ttw_params_el2(ss),
            _ => todo!("S1TTW Params for {:?}", regime),
        }
    }

    #[allow(unused_variables)]
    pub fn aarch64_s1min_txsz(&self, regime: Regime, walkparams: &S1TTWParams) -> u8 {
        // TODO: LVA3 and LVA features
        return 16;
    }

    #[allow(unused_variables)]
    pub fn aarch64_max_txsz(&self, tgx: TGx) -> u8 {
        // TODO: TTST featuer
        return 39;
    }

    #[allow(unused_variables)]
    pub fn aarch64_s1txsz_faults(&self, regime: Regime, walkparams: &S1TTWParams) -> bool {
        // TODO: boring stuff
        return false;
    }

    pub fn aarch64_addrtop(&self, tbid: bool, acctype: AccessType, tbi: bool) -> AddressSize {
        if tbid && acctype == AccessType::IFETCH {
            return 63;
        }

        if tbi {
            return 55;
        } else {
            return 63;
        }
    }

    pub fn aarch64_iasize(&self, txsz: u8) -> AddressSize {
        (64 - txsz) as AddressSize
    }

    pub fn aarch64_va_is_out_of_range(
        &self,
        va_in: u64,
        acctype: AccessType,
        regime: Regime,
        walkparams: &S1TTWParams,
    ) -> bool {
        let mut va = va_in;

        let addrtop = self.aarch64_addrtop(walkparams.tbid, acctype, walkparams.tbi);

        if walkparams.mtx && acctype != AccessType::IFETCH {
            if self.aarch64_get_varange(va) == VARange::UPPER {
                va |= 0b1111 << 56;
            } else {
                va &= !(0b1111 << 56);
            }
        }

        let iasize = self.aarch64_iasize(walkparams.txsz);

        if addrtop < iasize {
            return false;
        }

        let width = addrtop - iasize + 1;
        let bits = (va >> iasize) & ((1u64 << width) - 1);

        if self.has_unprivileged(regime) {
            if self.aarch64_get_varange(va) == VARange::LOWER {
                bits != 0
            } else {
                bits != ((1u64 << width) - 1)
            }
        } else {
            bits != 0
        }
    }

    pub fn aarch64_s1epd(&self, regime: Regime, va: u64) -> bool {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL20 => {
                if varange == VARange::LOWER {
                    E2H1_TCR_EL2Field::EPD0.fetch_bit(self.tcr_el2)
                } else {
                    E2H1_TCR_EL2Field::EPD1.fetch_bit(self.tcr_el2)
                }
            }
            Regime::EL10 => {
                if varange == VARange::LOWER {
                    TCR_EL1Field::EPD0.fetch_bit(self.tcr_el1)
                } else {
                    TCR_EL1Field::EPD1.fetch_bit(self.tcr_el1)
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1ttbr(&self, regime: Regime, va: u64) -> u128 {
        let varange = self.aarch64_get_varange(va);

        match regime {
            Regime::EL3 => self.ttbr0_el3 as u128,
            Regime::EL2 => self.ttbr0_el2 as u128,
            Regime::EL20 => {
                if varange == VARange::LOWER {
                    self.ttbr0_el2 as u128
                } else {
                    self.ttbr1_el2 as u128
                }
            }
            Regime::EL10 => {
                if varange == VARange::LOWER {
                    self.ttbr0_el1 as u128
                } else {
                    self.ttbr1_el1 as u128
                }
            }
            _ => todo!(),
        }
    }

    pub fn aarch64_s1start_level(&self, walkparams: &S1TTWParams) -> u8 {
        let iasize = self.aarch64_iasize(walkparams.txsz);
        let granulebits = self.tgx_granulebits(walkparams.tgx);
        let descsizelog2 = 3 + (walkparams.d128 as u8);
        let stride = granulebits - descsizelog2;
        let mut s1startlevel = FINAL_LEVEL - (((iasize - 1) - granulebits) / stride);
        if walkparams.d128 {
            s1startlevel += walkparams.skl;
        }

        s1startlevel
    }

    pub fn aarch64_s1ttb_base_address<T: Copy + Into<u128>>(
        &self,
        walkparams: &S1TTWParams,
        regime: Regime,
        ttbr: T,
    ) -> u64 {
        let tablebase: u64;

        let iasize = self.aarch64_iasize(walkparams.txsz);
        let granulebits = self.tgx_granulebits(walkparams.tgx);
        let descsizelog2 = 3 + (walkparams.d128 as u8);
        let stride = granulebits - descsizelog2;
        let startlevel = self.aarch64_s1start_level(walkparams);
        let levels = FINAL_LEVEL - startlevel;

        let mut tsize = (iasize - (levels * stride + granulebits)) + descsizelog2;

        if walkparams.d128 {
            tsize = tsize.max(5);
            tablebase = if regime == Regime::EL3 {
                ((ttbr.into() >> 5) as u64) & ones(51)
            } else {
                let hi = ((ttbr.into() >> 80) & 0xFF) as u64;
                let lo = ((ttbr.into() >> 5) as u64) & ones(43);
                (hi << 43) | lo
            };
        } else if walkparams.ds
            || (walkparams.tgx == TGx::_64KB
                && walkparams.ps == 0b110
                && Cpu::is_feature_implemented(Feature::LPA))
        {
            tsize = tsize.max(6);
            tablebase = ((((ttbr.into() >> 2) & 0xF) << 42) as u64
                | ((ttbr.into() >> 6) as u64 & ones(42))) as u64;
        } else {
            tablebase = ((ttbr.into() >> 1) as u64 & ones(47)) << 1;
        }

        align_down_p2::<56>(tablebase, tsize as u32)
    }

    pub fn aarch64_s1ttwalk_mecid(&self, emec: bool, regime: Regime, ss: SecurityState) -> u16 {
        if !emec {
            return DEFAULT_MECID;
        }

        if ss != SecurityState::Realm {
            return DEFAULT_MECID;
        }

        if !self.effective_scr_el3_mecen() {
            return DEFAULT_MECID;
        }

        match regime {
            Regime::EL2 => MECID_P0_EL2Field::MECID.fetch_range(self.mecid_p0_el2),
            Regime::EL20 => {
                if !E2H1_TCR_EL2Field::A1.fetch_bit(self.tcr_el2) {
                    return MECID_P1_EL2Field::MECID.fetch_range(self.mecid_p1_el2);
                } else {
                    return MECID_P0_EL2Field::MECID.fetch_range(self.mecid_p0_el2);
                }
            }
            Regime::EL10 => VMECID_P_EL2Field::MECID.fetch_range(self.vmecid_p_el2),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1dcache_enabled(&self, regime: Regime) -> bool {
        match regime {
            Regime::EL3 => SCTLR_EL3Field::C.fetch_bit(self.sctlr_el3),
            Regime::EL2 => SCTLR_EL2Field::C.fetch_bit(self.sctlr_el2),
            Regime::EL20 => SCTLR_EL2Field::C.fetch_bit(self.sctlr_el2),
            Regime::EL10 => SCTLR_EL1Field::C.fetch_bit(self.sctlr_el1),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_s1icache_enabled(&self, regime: Regime) -> bool {
        match regime {
            Regime::EL3 => SCTLR_EL3Field::I.fetch_bit(self.sctlr_el3),
            Regime::EL2 => SCTLR_EL2Field::I.fetch_bit(self.sctlr_el2),
            Regime::EL20 => SCTLR_EL2Field::I.fetch_bit(self.sctlr_el2),
            Regime::EL10 => SCTLR_EL1Field::I.fetch_bit(self.sctlr_el1),
            _ => unreachable!(),
        }
    }

    pub fn aarch64_pamax(&self) -> AddressSize {
        self.imp_def_int("Maximum Physical Address Size") as AddressSize
    }

    pub fn aarch64_physical_address_size(
        &self,
        d128: bool,
        ds: bool,
        encoded_ps: u8,
        tgx: TGx,
    ) -> AddressSize {
        let ps = match encoded_ps {
            0b000 => 32,
            0b001 => 36,
            0b010 => 40,
            0b011 => 42,
            0b100 => 44,
            0b101 => 48,
            0b110 => 52,
            0b111 => 56,
            _ => unreachable!(),
        } as u8;

        let max_ps = if d128 {
            self.aarch64_pamax()
        } else if Cpu::is_feature_implemented(Feature::LPA) && (tgx == TGx::_64KB || ds) {
            self.aarch64_pamax().min(52)
        } else {
            self.aarch64_pamax().min(48)
        };

        ps.max(max_ps) as AddressSize
    }

    pub fn aarch64_oaout_of_range(
        &self,
        address: u64,
        d128: bool,
        ds: bool,
        ps: u8,
        tgx: TGx,
    ) -> bool {
        let oasize = self.aarch64_physical_address_size(d128, ds, ps, tgx);

        if oasize < NUM_PABITS {
            ((address >> oasize) & ones((NUM_PABITS - oasize) as u32)) != 0
        } else {
            false
        }
    }

    pub fn aarch64_s1oaout_of_range(&self, address: u64, walkparams: &S1TTWParams) -> bool {
        self.aarch64_oaout_of_range(
            address,
            walkparams.d128,
            walkparams.ds,
            walkparams.ps,
            walkparams.tgx,
        )
    }
}
