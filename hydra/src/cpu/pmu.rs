pub struct Pmu {
    pub pmcr_el0: u64,
    pub pmcntenset_el0: u64,
    pub pmcntenclr_el0: u64,
    pub pmccntr_el0: u64,
    pub counters: Vec<u64>,
    pub event_types: Vec<u16>,
}
