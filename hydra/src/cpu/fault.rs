use crate::cpu::{
    Cpu, Feature, Register,
    exeception::{FullAddress, PASpace},
    shared::{EL, EL1, Faultable, SecurityState},
};

#[derive(PartialEq, Clone)]
pub enum Fault {
    None,
    AccessFlag,
    Alignment,
    Background,
    Domain,
    Permission,
    Translation,
    AddressSize,
    SyncExternal,
    SyncExternalOnWalk,
    SyncParity,
    SyncParityOnWalk,
    GPCFOnWalk,
    GPCFOnOutput,
    AsyncParity,
    AsyncExternal,
    TagCheck,
    Debug,
    TLBConflict,
    BranchTarget,
    UnsupportedAtomicHWUpdate,
    Lockdown,
    Exclusive,
    ICacheMaint,
}

impl Fault {
    pub fn is_external_sync_abort(&self) -> bool {
        assert!(self != &Self::None);
        if Cpu::is_feature_implemented(Feature::RAS) {
            assert!(!matches!(self, Fault::SyncParity | Fault::SyncParityOnWalk));
        }

        matches!(
            self,
            Fault::SyncExternal
                | Fault::SyncParity
                | Fault::SyncExternalOnWalk
                | Fault::SyncParityOnWalk
        )
    }
}

impl Faultable for Fault {
    fn is_fault(&self) -> bool {
        self != &Fault::None
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum AccessType {
    IFETCH, // Instruction FETCH
    GPR,    // Software load/store to a General Purpose Register
    FP,     // Software load/store to an FP register
    ASIMD,  // Software ASIMD extension load/store instructions
    SVE,    // Software SVE load/store instructions
    SME,    // Software SME load/store instructions
    IC,     // Sysop IC
    DC,     // Sysop DC (not DC {Z,G,GZ}VA)
    DCZero, // Sysop DC {Z,G,GZ}VA
    AT,     // Sysop AT
    NV2,    // NV2 memory redirected access
    SPE,    // Statistical Profiling buffer access
    GCS,    // Guarded Control Stack access
    TRBE,   // Trace Buffer access
    GPTW,   // Granule Protection Table Walk
    HACDBS, // Access to the HACDBS structure
    HDBSS,  // Access to entries in HDBSS
    TTW,    // Translation Table Walk
}

#[derive(Clone)]
pub enum MemAtomicOp {
    GCSSS1,
    ADD,
    BIC,
    EOR,
    ORR,
    SMAX,
    SMIN,
    UMAX,
    UMIN,
    SWP,
    CAS,
    FPADD,
    FPMAX,
    FPMIN,
    FPMAXNM,
    FPMINNM,
    BFADD,
    BFMAX,
    BFMIN,
    BFMAXNM,
    BFMINNM,
}

#[derive(PartialEq)]
pub enum MemOp {
    LOAD,
    STORE,
    PREFETCH,
}

#[derive(PartialEq, Clone)]
pub enum CacheOp {
    Clean,
    Invalidate,
    CleanInvalidate,
}

#[derive(Clone)]
pub enum CacheOpScope {
    SetWay,
    PoU,
    PoC,
    PoE,
    PoP,
    PoDP,
    PoPA,
    PoPS,
    OuterCache,
    ALLU,
    ALLUIS,
}

#[derive(PartialEq, Clone)]
pub enum CacheType {
    Data,
    Tag,
    DataTag,
    Instruction,
}

#[derive(PartialEq, Clone, Copy)]
pub enum VARange {
    LOWER,
    UPPER,
}

#[derive(PartialEq, Clone)]
pub enum PARTIDSpaceType {
    Secure,
    Root,
    Realm,
    NonSecure,
}

impl From<SecurityState> for PARTIDSpaceType {
    fn from(value: SecurityState) -> Self {
        match value {
            SecurityState::NonSecure => PARTIDSpaceType::NonSecure,
            SecurityState::Root => PARTIDSpaceType::Root,
            SecurityState::Realm => PARTIDSpaceType::Realm,
            SecurityState::Secure => PARTIDSpaceType::Secure,
        }
    }
}

pub type PARTIDType = u16;
pub type PMGType = u8;

#[derive(Clone)]
pub struct MPAMinfo {
    pub mpam_sp: PARTIDSpaceType,
    pub partid: PARTIDType,
    pub pmg: PMGType,
}

pub const DEFAULT_PARTID: PARTIDType = 0;
pub const DEFAULT_PMG: PMGType = 0;

impl MPAMinfo {
    pub fn default(partidspace: PARTIDSpaceType) -> Self {
        Self {
            mpam_sp: partidspace,
            partid: DEFAULT_PARTID,
            pmg: DEFAULT_PMG,
        }
    }
}

// impl Default for MPAMinfo {
//     fn default() -> Self {
//         Self {
//             mpam_sp: PARTIDSpaceType::NonSecure,
//             partid: 0,
//             pmg: 0,
//         }
//     }
// }

pub struct MPAMData {
    pub(crate) sm: bool,
    pub(crate) ind: bool,
}

impl MPAMData {
    pub fn gen_new_mpam_data() -> Self {
        Self {
            ind: false,
            sm: false,
        }
    }
}

#[derive(Clone)]
pub struct AccessDescriptor {
    pub(crate) acctype: AccessType,
    pub(crate) el: EL,
    pub(crate) ss: SecurityState,
    pub(crate) acqsc: bool,
    pub(crate) acqpc: bool,
    pub(crate) relsc: bool,
    pub(crate) limitedordered: bool,
    pub(crate) exclusive: bool,
    pub(crate) atomicop: bool,
    pub(crate) modop: Option<MemAtomicOp>,
    pub(crate) nontemporal: bool,
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) cacheop: Option<CacheOp>,
    pub(crate) opscope: Option<CacheOpScope>,
    pub(crate) cachetype: Option<CacheType>,
    pub(crate) pan: bool,
    pub(crate) nonfault: bool,
    pub(crate) firstfault: bool,
    pub(crate) first: bool,
    pub(crate) contiguous: bool,
    pub(crate) predicated: bool,
    pub(crate) streamingsve: bool,
    pub(crate) ls64: bool,
    pub(crate) withstatus: bool,
    pub(crate) mops: bool,
    pub(crate) rcw: bool,
    pub(crate) rcws: bool,
    pub(crate) toplevel: Option<bool>,
    pub(crate) varange: Option<VARange>,
    pub(crate) a32lsmd: bool,
    pub(crate) tagchecked: bool,
    pub(crate) tagaccess: bool,
    pub(crate) stzgm: bool,

    pub(crate) Rt: Option<Register>,
    pub(crate) Rt2: Option<Register>,
    pub(crate) Rs: Option<Register>,
    pub(crate) Rs2: Option<Register>,
    pub(crate) ispair: bool,
    pub(crate) highestaddressfirst: bool,
    pub(crate) lowestaddress: bool,

    pub(crate) mpam: MPAMinfo,
}

impl Default for AccessDescriptor {
    fn default() -> Self {
        Self {
            acctype: AccessType::IFETCH,
            el: EL1,
            ss: SecurityState::NonSecure,

            acqsc: false,
            acqpc: false,
            relsc: false,
            limitedordered: false,
            exclusive: false,
            atomicop: false,
            modop: None,

            nontemporal: false,
            read: true,
            write: false,

            cacheop: None,
            opscope: None,
            cachetype: None,

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
            rcw: false,
            rcws: false,
            toplevel: None,

            varange: None,
            a32lsmd: false,
            tagchecked: false,
            tagaccess: false,
            stzgm: false,

            Rt: None,
            Rt2: None,
            Rs: None,
            Rs2: None,

            ispair: false,
            highestaddressfirst: false,
            lowestaddress: true,

            mpam: MPAMinfo::default(PARTIDSpaceType::NonSecure),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum GPCF {
    None,        // No fault
    AddressSize, // GPT address size fault
    Walk,        // GPT walk fault
    EABT,        // Synchronous External abort on GPT fetch
    Fail,        // Granule protection fault
}

#[derive(Clone)]
pub struct GPCFRecord {
    pub gpf: GPCF,
    pub level: u8,
}

impl GPCFRecord {
    pub fn no_fault() -> Self {
        Self {
            gpf: GPCF::None,
            level: 0,
        }
    }

    pub fn fault(gpf: GPCF, level: u8) -> Self {
        Self { gpf, level }
    }
}

#[derive(Clone)]
pub enum ErrorState {
    UC,  // Uncontainable
    UEU, // Unrecoverable state
    UEO, // Restartable state
    UER, // Recoverable state
    CE,  // Corrected
}

#[derive(PartialEq, Clone, Copy)]
pub enum WatchpointType {
    Inactive,     // Watchpoint inactive or disabled
    AddrMatch,    // Address Match watchpoint
    AddrMismatch, // Address Mismatch watchpoint
}

#[derive(Clone)]
pub struct WatchpointInfo {
    pub(crate) wptype: WatchpointType,
    pub(crate) maybe_false_match: bool,
    pub(crate) watchpt_num: usize,
    pub(crate) value_match: bool,
    pub(crate) vaddress: u64,
}

#[derive(Clone)]
pub struct FaultRecord {
    pub statuscode: Fault,
    pub accessdesc: AccessDescriptor,

    pub vaddress: u64,
    pub ipaddress: FullAddress,
    pub paddress: FullAddress,
    pub gpcf: GPCFRecord,

    pub gpcfs2walk: bool,
    pub s2fs1walk: bool,
    pub write: bool,
    pub s1tagnotdata: bool,
    pub tagaccess: bool,
    pub level: u8,
    pub extflag: bool,
    pub secondstage: bool,
    pub assuredonly: bool,
    pub toplevel: bool,
    pub overlay: bool,
    pub dirtybit: bool,

    pub domain: u8,
    pub merrorstate: ErrorState,
    pub hdbssf: bool,
    pub watchptinfo: WatchpointInfo,
    pub debugmoe: u8,
}

impl FaultRecord {
    pub fn no_fault() -> Self {
        Self {
            statuscode: Fault::None,
            accessdesc: AccessDescriptor::default(),
            secondstage: false,
            s2fs1walk: false,
            dirtybit: false,
            overlay: false,
            toplevel: false,
            assuredonly: false,
            s1tagnotdata: false,
            tagaccess: false,
            gpcfs2walk: false,
            gpcf: GPCFRecord::no_fault(),
            hdbssf: false,
            vaddress: 0,
            paddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            ipaddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            debugmoe: 0,
            domain: 0,
            extflag: false,
            level: 0,
            merrorstate: ErrorState::CE,
            watchptinfo: WatchpointInfo {
                wptype: WatchpointType::Inactive,
                maybe_false_match: false,
                watchpt_num: 0,
                value_match: false,
                vaddress: 0,
            },
            write: false,
        }
    }

    pub fn no_fault_accdesc(accdesc: &AccessDescriptor) -> Self {
        Self {
            statuscode: Fault::None,
            accessdesc: accdesc.clone(),
            secondstage: false,
            s2fs1walk: false,
            dirtybit: false,
            overlay: false,
            toplevel: false,
            assuredonly: false,
            s1tagnotdata: false,
            tagaccess: false,
            gpcfs2walk: false,
            gpcf: GPCFRecord::no_fault(),
            hdbssf: false,
            vaddress: 0,
            paddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            ipaddress: FullAddress {
                paspace: PASpace::NonSecure,
                address: 0,
            },
            debugmoe: 0,
            domain: 0,
            extflag: false,
            level: 0,
            merrorstate: ErrorState::CE,
            watchptinfo: WatchpointInfo {
                wptype: WatchpointType::Inactive,
                maybe_false_match: false,
                watchpt_num: 0,
                value_match: false,
                vaddress: 0,
            },
            write: !accdesc.read && accdesc.write,
        }
    }

    pub fn no_fault_accdecs_va(accdesc: &AccessDescriptor, va: u64) -> Self {
        let mut fault = Self::no_fault();
        fault.accessdesc = accdesc.clone();
        fault.write = !accdesc.read && accdesc.write;
        fault.vaddress = va;

        fault
    }

    pub fn is_external_sync_abort(&self) -> bool {
        self.statuscode.is_external_sync_abort() || self.gpcf.gpf == GPCF::EABT
    }

    pub fn is_second_stage(&self) -> bool {
        assert!(self.statuscode != Fault::None);
        self.secondstage
    }
}
