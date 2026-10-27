#[allow(non_camel_case_types)]
#[derive(PartialEq)]
pub enum Constraint {
    NONE,
    UNKNOWN,
    UNDEF,
    UNDEFEL0,
    NOP,
    TRUE,
    FALSE,
    DISABLED,
    UNCOND,
    COND,
    ADDITIONAL_DECODE,
    WBSUPPRESS,
    FAULT,
    LIMITED_ATOMICITY,
    NVNV1_00,
    NVNV1_01,
    NVNV1_11,
    EL1TIMESTAMP,
    EL2TIMESTAMP,
    DEFAULT_ID,
    ANY_ID,
    OSH,
    ISH,
    NSH,
    NC,
    WT,
    WB,
    RESS,
    ALLRESS,
    FORCE,
    FORCENOSLCHECK,
    MAPTOALLOCATED,
    PMSCR_PCT_VIRT,
    ANYREG,
}

#[derive(PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Unpredictable {
    /// VMSR on MVFR
    VMSR,

    /// Writeback/transfer register overlap (load)
    WBOVERLAPLD,

    /// Writeback/transfer register overlap (store)
    WBOVERLAPST,

    /// Load Pair transfer register overlap
    LDPOVERLAP,

    /// Store-exclusive base/status register overlap
    BASEOVERLAP,

    /// Store-exclusive data/status register overlap
    DATAOVERLAP,

    /// Load-store alignment checks
    DEVPAGE2,

    /// Instruction fetch from Device memory
    INSTRDEVICE,

    /// Reserved CPACR value
    RESCPACR,

    /// Reserved MAIR value
    RESMAIR,

    /// Effect of SCTLR_ELx.C on Tagged attribute
    S1CTAGGED,

    /// Reserved Stage 2 MemAttr value
    S2RESMEMATTR,

    /// Reserved TEX::C::B value
    RESTEXCB,

    /// Reserved PRRR value
    RESPRRR,

    /// Reserved DACR field
    RESDACR,

    /// Reserved VTCR.S value
    RESVTCRS,

    /// Reserved or out-of-range TCR.TnSZ value
    RESTnSZ,

    /// Reserved SCTLR_ELx.TCF value
    RESTCF,

    /// Tag stored to Device memory
    DEVICETAGSTORE,

    /// Syndrome for a known-passing conditional A32 instruction
    ESRCONDPASS,

    /// Illegal State exception: zero PSTATE.IT
    ILZEROIT,

    /// Illegal State exception: zero PSTATE.T
    ILZEROT,

    /// Debug: prioritization of Vector Catch
    BPVECTORCATCHPRI,

    /// Debug Vector Catch: match on 2nd halfword
    VCMATCHHALF,

    /// Debug Vector Catch: match on Data Abort
    /// or Prefetch abort
    VCMATCHDAPA,

    /// Debug watchpoints: nonzero MASK and non-ones BAS
    WPMASKANDBAS,

    /// Debug watchpoints: non-contiguous BAS
    WPBASCONTIGUOUS,

    /// Debug watchpoints: reserved MASK
    RESWPMASK,

    /// Debug watchpoints: nonzero MASKed bits of address
    WPMASKEDBITS,

    /// Debug breakpoints and watchpoints: reserved control bits
    RESBPWPCTRL,

    /// Debug breakpoints: not implemented
    BPNOTIMPL,

    /// Debug breakpoints: reserved type
    RESBPTYPE,

    /// Debug breakpoints and watchpoints: reserved MDSELR_EL1.BANK
    RESMDSELR,

    /// Debug breakpoints: not-context-aware breakpoint
    BPNOTCTXCMP,

    /// Debug breakpoints: match on 2nd halfword of instruction
    BPMATCHHALF,

    /// Debug breakpoints: mismatch on 2nd halfword of instruction
    BPMISMATCHHALF,

    /// Debug breakpoints: a breakpoint is linked to that is not
    /// programmed with linking enabled
    BPLINKINGDISABLED,

    /// Debug breakpoints: reserved MASK
    RESBPMASK,

    /// Debug breakpoints: MASK is set for a Context matching
    /// breakpoint or  DBGBCR_EL1(n).BAS != '1111'
    BPMASK,

    /// Debug breakpoints: nonzero MASKed bits of address
    BPMASKEDBITS,

    /// Debug breakpoints: A linked breakpoint is
    /// linked to an address matching breakpoint
    BPLINKEDADDRMATCH,

    /// Debug: restart to a misaligned AArch32 PC value
    RESTARTALIGNPC,

    /// Debug: restart to a not-zero-extended AArch32 PC value
    RESTARTZEROUPPERPC,

    /// Zero top 32 bits of X registers in AArch32 state
    ZEROUPPER,

    /// Zero top 32 bits of PC on illegal return to
    /// AArch32 state
    ERETZEROUPPERPC,

    /// Force address to be aligned  interworking
    /// branch to A32 state
    A32FORCEALIGNPC,

    /// SMC disabled
    SMD,

    /// FF speculation
    NONFAULT,

    /// Zero top bits of Z registers in EL change
    SVEZEROUPPER,

    /// Load mem data in NF loads
    SVELDNFDATA,

    /// Write zeros in NF loads
    SVELDNFZERO,

    /// SP alignment fault  predicate is all zero
    CHECKSPNONEACTIVE,

    /// Zero top bits of ZA registers in EL change
    SMEZEROUPPER,

    /// Watchpoint match of last rounded up memory access in case of
    /// 16 byte rounding
    _16BYTEROUNDEDUPACCESS,

    /// Watchpoint match of first rounded down memory access in case of
    /// 16 byte rounding
    _16BYTEROUNDEDDOWNACCESS,

    /// HCR_EL2.[NV,NV1] == '01'
    NVNV1,

    /// Upper bits of a BADDR are not RESS
    BADDR_RESS,

    /// Reserved shareability encoding
    Shareability,

    /// Access Flag Update by HW
    AFUPDATE,

    /// Dirty Bit State Update by HW
    DBUPDATE,

    /// Unsupported memory attributes for HW update
    Unsupported_Atomic_HW_Update,

    /// Consider SCTLR_ELx().IESB in Debug state
    IESBinDebug,

    /// Bad settings for PMSFCR_EL1/PMSEVFR_EL1/PMSLATFR_EL1
    BADPMSFCR,

    /// Zero saved BType value in SPSR_ELx/DPSR_EL0
    ZEROBTYPE,

    /// Timestamp constrained to virtual or physical
    EL2TIMESTAMP,

    EL1TIMESTAMP,

    /// Reserved MDCR_EL3.[NSTBE,NSTB] or MDCR_EL3.[NSPBE,NSPB] value
    RESERVEDNSxB,

    RESERVEDNSxB_Trap,

    /// WFET or WFIT instruction in Debug state
    WFxTDEBUG,

    /// Address does not support LS64 instructions
    LS64UNSUPPORTED,

    /// Unaligned exclusives, atomics, acquire/release
    /// to a region that is not to Normal inner write-back
    /// outer write-back shareable generate an Alignment fault.
    LSE2_ALIGNMENT_FAULT,

    /// 128-bit Atomic or 128-bit RCW{S} transfer register overlap
    LSE128OVERLAP,

    /// Clearing DCC/ITR sticky flags  instruction is in flight
    CLEARERRITEZERO,

    /// ALUEXCEPTIONRETURN  in user/system mode in
    /// A32 instructions
    ALUEXCEPTIONRETURN,

    /// Trap to register in debug state are ignored
    IGNORETRAPINDEBUG,

    /// Compare DBGBVR.RESS for BP/WP
    DBGxVR_RESS,

    /// Inaccessible event counter
    PMUEVENTCOUNTER,

    /// Reserved PMSCR.PCT behavior
    PMSCR_PCT,

    /// MDCR_EL2.HPMN or HDCR.HPMN is larger than PMCR.N or
    /// FEAT_HPMN0 is not implemented and HPMN is 0.
    RES_HPMN,

    /// Chained PMU counters idx and idx+1 are not in same range
    COUNT_CHAIN,

    /// PMCCR.EPMN is larger than PMCR.N
    RES_EPMN,

    /// Generate BRB_FILTRATE event on BRB injection
    BRBFILTRATE,

    /// Generate PMU_SNAPSHOT event in Debug state
    PMUSNAPSHOTEVENT,

    /// Reserved MDCR_EL3.EPMSSAD value
    RESEPMSSAD,

    /// Reserved PMECR_EL1.SSE value
    RESPMSSE,

    /// Enable for PMU Profiling exception and PMUIRQ
    RESPMEE,

    /// Enables for SPE Profiling exceptions and PMSIRQ
    RESPMSEE,

    /// Enables for TRBE Profiling exceptions and PMSIRQ
    RESTRFEE,

    /// NonZero TRBPTR_EL1[63:(OAMax)]
    TRBPTR_Fault_AddressSize,

    /// Operands for CPY*/SET* instructions overlap
    MOPSOVERLAP,

    /// Operands for CPY*/SET* instructions use 0b11111
    /// as a register specifier
    MOPS_R31,

    /// PMBPTR_EL1[63:(OAMax)] NonZero causes Address Size fault
    PMBPTR_Fault_AddressSize,

    /// Chooses which value to return in a non failed Atomic Compare and Swap
    CASRETURNOLDVALUE,

    /// Enables write of the newvalue in a failed Atomic Compare and Swap
    WRITEFAILEDCAS,

    /// Enables write of the newvalue in a failed Atomic Compare and Swap
    /// or failed RCW{S}
    WRITEFAILEDRCWCAS,

    /// Store-only Tag checking on a failed Atomic Compare and Swap
    STRONLYTAGCHECKEDCAS,

    /// Store-only Tag checking on a failed RCW(S) checks
    /// or RCW(S) Atomic Compare and Swap
    STRONLYTAGCHECKEDRCWSCAS,

    /// Reserved MDCR_EL3.ETBAD value
    RES_ETBAD,

    /// Invalid Streaming Mode filter bits
    RES_PMU_VS,

    /// Apply Checked Pointer Arithmetic on a sequential access to bytes
    /// that cross the 0xXXFF_FFFF_FFFF_FFFF boundary.
    CPACHECK,

    /// Reserved PMEVTYPER_EL0(n).[TC,TE,TLC] values
    RESTC,

    ///  FEAT_MTE is implemented, if Memory access mode is enabled
    /// and PSTATE.TCO is 0, Reads and writes to the external debug
    /// interface DTR registers are CONSTRAINED UNPREDICTABLE for tagcheck
    NODTRTAGCHK,

    /// Use the default PMG  the default PARTID is generated
    /// due to MPAM error
    USE_DEFAULT_PMG,

    /// MPAM PARTID/PMG exceeding PARTID_MAX/PMG_MAX
    OUT_OF_RANGE_ID,

    /// MPAM vPARTID exceeding (VPMR_MAX << 2) + 3
    OUT_OF_RANGE_VID,

    /// If the atomic instructions are not atomic in regard to other
    /// agents that access memory, then the instruction can have one or
    /// more of the following effects
    Atomic_SYNC_ABORT,

    Atomic_SERROR,

    Atomic_NOP,

    Atomic_MMU_IMPDEF_FAULT,

    /// Accessing DBGDSCRint via MRC in debug state
    MRC_APSR_TARGET,

    /// Accessing Banked register not accessible from the PE mode
    BankedRegister,

    /// Accessing unimplemented Banked register
    UnimplementedRegister,

    /// Reserved ERRACR.{RLRA,SRA,NSRA} values
    RES_RA,

    /// Linked target does not support linking
    BPNOTLINKING,
}

impl Unpredictable {
    pub fn constrain_integer<N>(&self, low: N, high: N) -> (Constraint, N) {
        let c = self.constrain();
        if c == Constraint::UNKNOWN {
            (c, low)
        } else {
            (c, high)
        }
    }

    pub fn constrain_bool(&self) -> bool {
        let c = self.constrain();
        assert!([Constraint::TRUE, Constraint::FALSE].contains(&c));

        c == Constraint::TRUE
    }

    pub fn constrain_bits(&self) -> (Constraint, u8) {
        let c = self.constrain();

        match c {
            Constraint::UNKNOWN => (c, 0),
            Constraint::PMSCR_PCT_VIRT => (c, 0),
            Constraint::NC if Self::S2RESMEMATTR == *self => (c, 0b0101),
            Constraint::WT if Self::S2RESMEMATTR == *self => (c, 0b1010),
            Constraint::WB if Self::S2RESMEMATTR == *self => (c, 0b1111),
            Constraint::EL1TIMESTAMP => (c, 0b01),
            Constraint::EL2TIMESTAMP => (c, 0b01),
            Constraint::MAPTOALLOCATED => (c, 0b010),
            _ => (c, 0),
        }
    }

    pub const fn constrain(&self) -> Constraint {
        match self {
            Self::VMSR => Constraint::UNDEF,
            Self::WBOVERLAPLD => Constraint::WBSUPPRESS, // return loaded value
            Self::WBOVERLAPST => Constraint::NONE,       // store pre-writeback value
            Self::LDPOVERLAP => Constraint::UNDEF,       // instruction is UNDEFINED
            Self::BASEOVERLAP => Constraint::UNKNOWN,    // use UNKNOWN address
            Self::DATAOVERLAP => Constraint::UNKNOWN,    // store UNKNOWN value
            Self::DEVPAGE2 => Constraint::FAULT,         // take an alignment fault
            Self::RESTCF => Constraint::UNKNOWN,         // Do not take a fault
            Self::DEVICETAGSTORE => Constraint::NONE,    // Do not take a fault
            Self::INSTRDEVICE => Constraint::NONE,       // Do not take a fault
            Self::RESCPACR => Constraint::TRUE,          // Map to UNKNOWN value
            Self::RESMAIR => Constraint::UNKNOWN,        // Map to UNKNOWN value
            Self::S1CTAGGED => Constraint::FALSE, // SCTLR_ELx.C == '0' marks address as untagged
            Self::S2RESMEMATTR => Constraint::NC, // Map to Noncacheable value
            Self::RESTEXCB => Constraint::UNKNOWN, // Map to UNKNOWN value
            Self::RESDACR => Constraint::UNKNOWN, // Map to UNKNOWN value
            Self::RESPRRR => Constraint::UNKNOWN, // Map to UNKNOWN value
            Self::RESVTCRS => Constraint::UNKNOWN, // Map to UNKNOWN value
            Self::RESTnSZ => Constraint::FORCE,   // Map to the limit value
            Self::ESRCONDPASS => Constraint::FALSE, // Report as "AL"
            Self::ILZEROIT => Constraint::FALSE,  // Do not zero PSTATE.IT
            Self::ILZEROT => Constraint::FALSE,   // Do not zero PSTATE.T
            Self::BPVECTORCATCHPRI => Constraint::TRUE, // Debug Vector Catch: match on 2nd halfword
            Self::VCMATCHHALF => Constraint::FALSE, // No match
            Self::VCMATCHDAPA => Constraint::FALSE, // No match on Data Abort or Prefetch abort
            Self::WPMASKANDBAS => Constraint::FALSE, // Watchpoint disabled
            Self::WPBASCONTIGUOUS => Constraint::FALSE, // Watchpoint disabled
            Self::RESWPMASK => Constraint::DISABLED, // Watchpoint disabled
            Self::WPMASKEDBITS => Constraint::FALSE, // Watchpoint disabled
            Self::RESBPWPCTRL => Constraint::DISABLED, // Breakpoint/watchpoint disabled
            Self::BPNOTIMPL => Constraint::DISABLED, // Breakpoint disabled
            Self::RESBPTYPE => Constraint::DISABLED, // Breakpoint disabled
            Self::BPNOTCTXCMP => Constraint::DISABLED, // Breakpoint disabled
            Self::RESMDSELR => Constraint::UNKNOWN, // Map to UNKNOWN value
            Self::BPMATCHHALF => Constraint::FALSE, // No match
            Self::BPMISMATCHHALF => Constraint::FALSE, // No match
            Self::BPLINKINGDISABLED => Constraint::FALSE, // Breakpoint does not match
            Self::RESTARTALIGNPC => Constraint::FALSE, // Do not force alignment
            Self::RESTARTZEROUPPERPC => Constraint::TRUE, // Force zero extension
            Self::ZEROUPPER => Constraint::TRUE,  // zero top halves of X registers
            Self::ERETZEROUPPERPC => Constraint::TRUE, // zero top half of PC
            Self::A32FORCEALIGNPC => Constraint::FALSE, // Do not force alignment
            Self::SMD => Constraint::UNDEF,       // disabled SMC is Unallocated
            Self::NONFAULT => Constraint::FALSE,  // Speculation enabled
            Self::SVEZEROUPPER => Constraint::TRUE, // zero top bits of Z registers
            Self::SVELDNFDATA => Constraint::TRUE, // Load mem data in NF loads
            Self::SVELDNFZERO => Constraint::TRUE, // Write zeros in NF loads
            Self::CHECKSPNONEACTIVE => Constraint::TRUE, // Check SP alignment
            Self::SMEZEROUPPER => Constraint::TRUE, // zero top bits of ZA registers
            Self::NVNV1 => Constraint::NVNV1_00, // Map unpredictable configuration of HCR_EL2[NV,NV1]
            // to NV = 0 and NV1 = 0
            Self::BADDR_RESS => Constraint::RESS, // Values behave as RESS
            Self::Shareability => Constraint::OSH, // Map reserved encoding of shareability to Outer Shareable
            Self::AFUPDATE =>
            // AF update for alignment or Permission fault
            {
                Constraint::TRUE
            }
            Self::DBUPDATE =>
            // DB State update for alignment fault
            {
                Constraint::TRUE
            }
            Self::Unsupported_Atomic_HW_Update => Constraint::FALSE, // Do not raise Unsupported Atomic HW Update fault
            //  encountering unsupported memory attributes
            Self::IESBinDebug =>
            // Use SCTLR_ELx[].IESB in Debug state
            {
                Constraint::TRUE
            }
            Self::BADPMSFCR =>
            // Bad settings for PMSFCR_EL1/PMSEVFR_EL1/PMSLATFR_EL1
            {
                Constraint::TRUE
            }
            Self::PMBPTR_Fault_AddressSize => Constraint::FALSE,
            Self::RES_PMU_VS =>
            // Bad setting for SVE Streaming mode filter
            {
                Constraint::FALSE
            }
            Self::COUNT_CHAIN =>
            // Chained PMU counters idx, idx+1 are not in same range
            {
                Constraint::FALSE
            }
            Self::EL1TIMESTAMP =>
            // Bad settings for TRFCR_EL1
            {
                Constraint::EL1TIMESTAMP
            }
            Self::EL2TIMESTAMP =>
            // Bad settings for TRFCR_EL2
            {
                Constraint::EL2TIMESTAMP
            }
            Self::ZEROBTYPE => Constraint::TRUE, // Save BTYPE in SPSR_ELx/DPSR_EL0 as '00'
            Self::CLEARERRITEZERO =>
            // Clearing sticky errors when instruction in flight
            {
                Constraint::FALSE
            }
            Self::ALUEXCEPTIONRETURN => Constraint::UNDEF,
            Self::DBGxVR_RESS => Constraint::FALSE,
            Self::PMSCR_PCT => Constraint::PMSCR_PCT_VIRT,
            Self::WFxTDEBUG => Constraint::FALSE, // WFxT in Debug state does not execute as a NOP
            // Accesses are not single-copy atomic above the byte level
            Self::LS64UNSUPPORTED => Constraint::LIMITED_ATOMICITY,
            // Unaligned exclusives, atomics, acquire/release
            // to a region that is not to Normal inner write-back
            // outer write-back shareable generate an Alignment fault.
            Self::LSE2_ALIGNMENT_FAULT => Constraint::FALSE,
            Self::LSE128OVERLAP => Constraint::UNDEF, // instruction is UNDEFINED
            Self::IGNORETRAPINDEBUG => Constraint::FALSE, // Trap to register access in debug state is enabled
            Self::PMUEVENTCOUNTER => Constraint::UNDEF,   // Accesses to the register are UNDEFINED
            Self::RES_HPMN => Constraint::UNKNOWN,        // The counter is reserved for EL2 use
            Self::RES_EPMN => Constraint::UNKNOWN, // The counter is reserved for external use
            Self::BRBFILTRATE => Constraint::FALSE, // BRB_FILTRATE event not generated on BRB injection
            Self::PMUSNAPSHOTEVENT =>
            // PMU_SNAPSHOT event not generated in Debug state
            {
                Constraint::FALSE
            }
            Self::RESEPMSSAD =>
            // External access to PMUv3 Snapshot extension allowed in Secure state
            //  FEAT_RME is not implemented or Root state otherwise
            {
                Constraint::UNKNOWN
            }
            Self::RESPMSSE => Constraint::DISABLED, // PMU capture events disabled
            Self::RESPMEE => Constraint::DISABLED, // PMU Profiling exception disabled, PMUIRQ enabled
            Self::RESPMSEE => Constraint::DISABLED, // SPE Profiling exception disabled
            Self::RESTRFEE => Constraint::DISABLED, // TRBE Profiling exception disabled
            Self::TRBPTR_Fault_AddressSize => Constraint::TRUE, // TRBPTR_EL1[63:(OAMax)] NonZero causes Address Size fault
            Self::RESTC => Constraint::DISABLED,                // Threshold features disabled
            Self::MOPSOVERLAP =>
            // Instruction is UNDEFINED
            {
                Constraint::UNDEF
            }
            Self::MOPS_R31 =>
            // Instruction is UNDEFINED
            {
                Constraint::UNDEF
            }
            Self::CASRETURNOLDVALUE => Constraint::TRUE,
            Self::WRITEFAILEDCAS => Constraint::FALSE,
            Self::WRITEFAILEDRCWCAS => Constraint::FALSE,
            Self::STRONLYTAGCHECKEDCAS => Constraint::TRUE, // CAS with compare fail does a Tag Check
            Self::STRONLYTAGCHECKEDRCWSCAS => Constraint::TRUE, // RCW(S)CAS with RCW(S) fail/compare fail does a Tag Check
            Self::RESERVEDNSxB => Constraint::MAPTOALLOCATED,
            Self::RESERVEDNSxB_Trap => Constraint::TRUE,
            Self::RES_ETBAD =>
            // ExternalTracebufferaccess disabled for res values
            {
                Constraint::DISABLED
            }
            Self::RESBPMASK => Constraint::DISABLED, // Mask set to 0
            Self::BPMASK => Constraint::FALSE,       // Breakpoint match will fail
            Self::BPMASKEDBITS => Constraint::FALSE, // Breakpoint match will fail
            Self::BPLINKEDADDRMATCH => Constraint::FALSE, // Breakpoint match will fail
            Self::_16BYTEROUNDEDUPACCESS =>
            // Watchpoint match of 16 byte rounded range in case of SVE access
            {
                Constraint::TRUE
            }
            Self::_16BYTEROUNDEDDOWNACCESS =>
            // Watchpoint match of 16 byte rounded range in case of SVE access
            {
                Constraint::TRUE
            }
            Self::NODTRTAGCHK =>
            // The load or store operation does not perform the Tag check in Debug state
            {
                Constraint::TRUE
            }
            Self::CPACHECK =>
            // Do not perform Checked Pointer Arithmetic
            {
                Constraint::FALSE
            }
            Self::Atomic_SYNC_ABORT => Constraint::FALSE,
            Self::Atomic_SERROR => Constraint::FALSE,
            Self::Atomic_NOP => Constraint::FALSE,
            Self::Atomic_MMU_IMPDEF_FAULT => Constraint::FALSE,
            Self::USE_DEFAULT_PMG => Constraint::TRUE,
            Self::OUT_OF_RANGE_ID => Constraint::DEFAULT_ID,
            Self::OUT_OF_RANGE_VID => Constraint::DEFAULT_ID,
            Self::BankedRegister => Constraint::NOP,
            Self::UnimplementedRegister => Constraint::NOP,
            Self::RES_RA => Constraint::TRUE,
            Self::BPNOTLINKING => Constraint::FALSE,
            _ => unreachable!(),
        }
    }
}
