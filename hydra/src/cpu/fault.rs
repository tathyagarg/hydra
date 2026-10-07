use crate::cpu::{
    exeception::{FullAddress, PASpace},
    shared::{EL, EL0, EL1, SecurityState},
};

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

pub enum CacheOp {
    Clean,
    Invalidate,
    CleanInvalidate,
}

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

pub enum CacheType {
    Data,
    Tag,
    DataTag,
    Instruction,
}

pub enum VARange {
    LOWER,
    UPPER,
}

pub enum PARTIDSpaceType {
    Secure,
    Root,
    Realm,
    NonSecure,
}

pub type PARTIDType = u16;
pub type PMGType = u8;

pub struct MPAMinfo {
    mpam_sp: PARTIDSpaceType,
    partid: PARTIDType,
    pmg: PMGType,
}

impl Default for MPAMinfo {
    fn default() -> Self {
        Self {
            mpam_sp: PARTIDSpaceType::NonSecure,
            partid: 0,
            pmg: 0,
        }
    }
}

pub struct AccessDescriptor {
    acctype: AccessType,
    el: EL,
    ss: SecurityState,
    acqsc: bool,
    acqpc: bool,
    relsc: bool,
    limitedordered: bool,
    exclusive: bool,
    atomicop: bool,
    modoop: MemAtomicOp,
    nontemporal: bool,
    read: bool,
    write: bool,
    cacheop: CacheOp,
    opscope: CacheOpScope,
    cachetype: CacheType,
    pan: bool,
    nonfault: bool,
    firstfault: bool,
    first: bool,
    contiguous: bool,
    predicated: bool,
    streamingsve: bool,
    ls64: bool,
    withstatus: bool,
    mops: bool,
    rcw: bool,
    rcws: bool,
    toplevel: bool,
    varange: VARange,
    a32lsmd: bool,
    tagchecked: bool,
    tagaccess: bool,
    stzgm: bool,

    Rt: u64,
    Rt2: u64,
    Rs: u64,
    Rs2: u64,
    ispair: bool,
    highestaddressfirst: bool,
    lowestaddress: bool,

    mpam: MPAMinfo,
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
            modoop: MemAtomicOp::ADD,

            nontemporal: false,
            read: true,
            write: false,

            cacheop: CacheOp::Clean,
            opscope: CacheOpScope::SetWay,
            cachetype: CacheType::Data,

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
            toplevel: true,

            varange: VARange::LOWER,
            a32lsmd: false,
            tagchecked: false,
            tagaccess: false,
            stzgm: false,

            Rt: 0,
            Rt2: 0,
            Rs: 0,
            Rs2: 0,

            ispair: false,
            highestaddressfirst: false,
            lowestaddress: true,

            mpam: MPAMinfo::default(),
        }
    }
}

pub enum GPCF {
    None,        // No fault
    AddressSize, // GPT address size fault
    Walk,        // GPT walk fault
    EABT,        // Synchronous External abort on GPT fetch
    Fail,        // Granule protection fault
}

pub struct GPCFRecord {
    gpf: GPCF,
    level: u8,
}

impl GPCFRecord {
    pub fn no_fault() -> Self {
        Self {
            gpf: GPCF::None,
            level: 0,
        }
    }
}

pub enum ErrorState {
    UC,  // Uncontainable
    UEU, // Unrecoverable state
    UEO, // Restartable state
    UER, // Recoverable state
    CE,  // Corrected
}

pub enum WatchpointType {
    Inactive,     // Watchpoint inactive or disabled
    AddrMatch,    // Address Match watchpoint
    AddrMismatch, // Address Mismatch watchpoint
}

pub struct WatchpointInfo {
    wptype: WatchpointType,
    maybe_false_match: bool,
    watchpt_num: u64,
    value_match: bool,
    vaddress: u64,
}

pub struct FaultRecord {
    statuscode: Fault,
    accessdesc: AccessDescriptor,

    vaddress: u64,
    ipaddress: FullAddress,
    paddress: FullAddress,
    gpcf: GPCFRecord,

    gpcfs2walk: bool,
    s2fs1walk: bool,
    write: bool,
    s1tagnotdata: bool,
    tagaccess: bool,
    level: u8,
    extflag: bool,
    secondstage: bool,
    assuredonly: bool,
    toplevel: bool,
    overlay: bool,
    dirtybit: bool,

    domain: u8,
    merrorstate: ErrorState,
    hdbssf: bool,
    watchptinfo: WatchpointInfo,
    debugmoe: u8,
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
}
