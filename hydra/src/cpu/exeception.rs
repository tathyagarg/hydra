#[derive(PartialEq)]
pub enum Exception {
    Uncategorized,
    WFxTrap,
    CP15RTTrap,
    CP15RRTTrap,
    CP14RTTrap,
    CP14DTTrap,
    CP14RRTTrap,
    AdvSIMDFPAccessTrap,
    FPIDTrap,
    OtherTrap,
    PACTrap,
    IllegalState,
    SupervisorCall,
    HypervisorCall,
    MonitorCall,
    SystemRegisterTrap,
    ERetTrap,
    InstructionAbort,
    PCAlignment,
    DataAbort,
    NV2DataAbort,
    PACFail,
    SPAlignment,
    FPTrappedException,
    SError,
    Breakpoint,
    SoftwareStep,
    Watchpoint,
    NV2Watchpoint,
    SoftwareBreakpoint,
    VectorCatch,
    IRQ,
    SVEAccessTrap,
    SMEAccessTrap,
    GPC,
    BranchTarget,
    MemCpyMemSet,
    GCSFail,
    Profiling,
    SystemRegister128Trap,
    FIQ,
}

pub struct IssType {
    /// bits(25)
    iss: u32,
    /// bits(24)
    iss2: u32,
}

#[derive(Clone, Copy)]
pub enum PASpace {
    Root,
    SystemAgent,
    NonSecureProtected,
    NA6,
    NA7,
    Realm,
    Secure,
    NonSecure,
}

#[derive(Clone, Copy)]
pub struct FullAddress {
    pub paspace: PASpace,
    /// bits(NUM_PABITS) = u64
    pub address: u64,
}

pub struct ExceptionRecord {
    pub exceptype: Exception,
    syndrome: IssType,
    paddress: FullAddress,
    vaddress: u64,
    ipvalid: bool,
    pavalid: bool,
    NS: bool,
    /// bits(NUM_PABITS) = u64
    ipaddress: u64,
    pub trapped_syscall_inst: bool,
}

impl ExceptionRecord {
    pub fn syndrome(exceptype: Exception) -> Self {
        exception_syndrome(exceptype)
    }
}

fn exception_syndrome(exceptype: Exception) -> ExceptionRecord {
    ExceptionRecord {
        exceptype: exceptype,
        syndrome: IssType { iss: 0, iss2: 0 },
        vaddress: 0,
        ipvalid: false,
        pavalid: false,
        NS: false,
        ipaddress: 0,
        paddress: FullAddress {
            paspace: PASpace::Root,
            address: 0,
        },
        trapped_syscall_inst: false,
    }
}
