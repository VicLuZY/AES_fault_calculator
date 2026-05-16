//! FaultCalc core engine.
//!
//! This crate is intentionally UI agnostic. It owns the case model, validation,
//! symmetrical component network solution, fault calculations, and report export.

pub mod complex;
pub mod domain;
pub mod model;
pub mod sample;
pub mod solver;

pub use domain::*;
pub use model::*;
pub use sample::*;
pub use solver::*;

pub const VERSION: &str = "0.3.0-rust-wasm";

pub type Result<T> = std::result::Result<T, FaultCalcError>;

#[derive(Debug, Clone)]
pub struct FaultCalcError {
    pub message: String,
}

impl FaultCalcError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FaultCalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FaultCalcError {}

impl From<serde_json::Error> for FaultCalcError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<std::io::Error> for FaultCalcError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}
