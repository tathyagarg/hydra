#![allow(dead_code, non_snake_case)]

pub struct PState {
    pub(super) N: bool,      // Negative condition flag
    pub(super) Z: bool,      // Zero condition flag
    pub(super) C: bool,      // Carry condition flag
    pub(super) V: bool,      // Overflow condition flag
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
    pub(super) BTYPE: u8,    // Branch Type [v8.5]
    pub(super) PACM: bool,   // PAC instruction modifier
    pub(super) ZA: bool,     // Accumulation array enabled [SME]
    pub(super) SM: bool,     // Streaming SVE mode enabled [SME]
    pub(super) ALLINT: bool, // Interrupt mask bit
    pub(super) UINJ: bool,   // Undefined Exception Injection
    pub(super) SS: bool,     // Software step bit
    pub(super) IL: bool,     // Illegal Execution state bit
    pub(super) EL: u8,       // Exception level
    pub(super) nRW: bool,    // Execution state: 0=AArch64, 1=AArch32
    pub(super) SP: bool,     // Stack pointer select: 0=SP0, 1=SPx [AArch64 only]
    pub(super) SSBS: bool,   // Speculative Store Bypass Safe
}

impl Default for PState {
    fn default() -> Self {
        Self {
            N: false,
            Z: false,
            C: false,
            V: false,
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
            BTYPE: 0,
            PACM: false,
            ZA: false,
            SM: false,
            ALLINT: false,
            UINJ: false,
            SS: false,
            IL: false,
            EL: 0,
            nRW: false,
            SP: true,
            SSBS: false,
        }
    }
}

impl PState {
    pub fn __dbg_set_C(&mut self, v: bool) {
        self.C = v;
    }

    pub fn __dbg_get_C(&self) -> bool {
        self.C
    }
}
