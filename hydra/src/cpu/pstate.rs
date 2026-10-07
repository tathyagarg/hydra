#![allow(dead_code, non_snake_case)]

use crate::cpu::{common::NZCV, shared::EL};

pub struct PState {
    _N: bool,                // Negative condition flag
    _Z: bool,                // Zero condition flag
    _C: bool,                // Carry condition flag
    _V: bool,                // Overflow condition flag
    pub(super) D: bool,      // Debug mask bit [AArch64 only]
    pub(super) A: bool,      // SError interrupt mask bit
    pub(super) I: bool,      // IRQ mask bit
    pub(super) F: bool,      // FIQ mask bit
    pub(super) EXLOCK: bool, // Lock exception return state
    pub(super) PAN: bool,    // Privileged Access Never Bit [v8.1]
    pub(super) UAO: bool,    // User Access Override [v8.2]
    pub(super) DIT: bool,    // Data Independent Timing [v8.4]
    pub(super) TCO: bool,    // Tag Check Override [v8.5, AArch64 only]
    pub(super) PM: bool,     // PMU exception Mask
    _BTYPE: u8,              // Branch Type [v8.5]
    pub(super) PACM: bool,   // PAC instruction modifier
    pub(super) ZA: bool,     // Accumulation array enabled [SME]
    pub(super) SM: bool,     // Streaming SVE mode enabled [SME]
    pub(super) ALLINT: bool, // Interrupt mask bit
    pub(super) UINJ: bool,   // Undefined Exception Injection
    pub(super) SS: bool,     // Software step bit
    pub(super) IL: bool,     // Illegal Execution state bit
    _EL: EL,                 // Exception level
    pub(super) nRW: bool,    // Execution state: 0=AArch64, 1=AArch32
    pub(super) SP: bool,     // Stack pointer select: 0=SP0, 1=SPx [AArch64 only]
    pub(super) SSBS: bool,   // Speculative Store Bypass Safe
}

impl Default for PState {
    fn default() -> Self {
        Self {
            _N: false,
            _Z: false,
            _C: false,
            _V: false,
            D: true,
            A: true,
            I: true,
            F: true,
            EXLOCK: false,
            PAN: false,
            UAO: false,
            DIT: false,
            TCO: false,
            PM: false,
            _BTYPE: 0,
            PACM: false,
            ZA: false,
            SM: false,
            ALLINT: false,
            UINJ: false,
            SS: false,
            IL: false,
            _EL: 0,
            nRW: false,
            SP: true,
            SSBS: false,
        }
    }
}

impl PState {
    pub fn __dbg_set_C(&mut self, v: bool) {
        self._C = v;
    }

    pub fn __dbg_get_C(&self) -> bool {
        self._C
    }

    pub fn __dbg_set_Z(&mut self, v: bool) {
        self._Z = v;
    }

    pub fn __dbg_get_Z(&self) -> bool {
        self._Z
    }

    pub fn set_nzcv(&mut self, nzcv: &NZCV) {
        self._N = nzcv.N();
        self._Z = nzcv.Z();
        self._C = nzcv.C();
        self._V = nzcv.V();
    }

    pub fn nzcv(&self) -> NZCV {
        NZCV::init(self._N, self._Z, self._C, self._V)
    }

    pub fn N(&self) -> bool {
        self._N
    }

    pub fn set_N(&mut self, v: bool) {
        self._N = v;
    }

    pub fn Z(&self) -> bool {
        self._Z
    }

    pub fn set_Z(&mut self, v: bool) {
        self._Z = v;
    }

    pub fn C(&self) -> bool {
        self._C
    }

    pub fn set_C(&mut self, v: bool) {
        self._C = v;
    }

    pub fn V(&self) -> bool {
        self._V
    }

    pub fn set_V(&mut self, v: bool) {
        self._V = v;
    }

    pub fn BTYPE(&self) -> u8 {
        self._BTYPE
    }

    pub fn EL(&self) -> u8 {
        self._EL
    }

    pub fn invert_C(&mut self) {
        self._C = !self._C;
    }
}
