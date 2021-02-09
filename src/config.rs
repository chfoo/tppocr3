use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProcessorConfig {
    pub region: Vec<Region>,
}

#[derive(Clone, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub processor: ProcessorStrategy,
}

#[derive(Clone, Deserialize)]
pub enum ProcessorStrategy {
    FixedLine,
    DialogScroll,
}
