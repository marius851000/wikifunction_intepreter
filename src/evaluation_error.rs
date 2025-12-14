use thiserror::Error;

use crate::{DataEntry, Reference};

//TODO: error handling should be much better than that. Will do for now.
#[derive(Error, Debug)]
pub enum EvaluationError {
    #[error("low level: Not a map")]
    LowLevelNotAMap,
    #[error("low level: Not an array")]
    LowLevelNotAnArray,
    #[error("low level: Not a string")]
    LowLevelNotAString,
    #[error("low level: missing key {0}")]
    MissingKey(Reference),
    #[error("low level: parse ZID")]
    ParseZID(#[source] anyhow::Error),
    #[error("low level: validator result not true")]
    TestSuiteFailed(DataEntry),
    #[error("low level: unimplemented {0}")]
    Unimplemented(String),
    #[error("info: test result: {0:?}")]
    TestResultInfo(DataEntry, #[source] Box<EvaluationError>),
    #[error("info: trace: {0}")]
    Previous(String, #[source] Box<EvaluationError>),
}

impl EvaluationError {
    pub fn trace(self, message: String) -> Self {
        Self::Previous(message, Box::new(self))
    }

    pub fn trace_str(self, message: &str) -> Self {
        self.trace(message.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum Provenance {
    Persistant(Reference),
    //TODO: manage array
    FromOther(Box<Provenance>, Vec<Reference>),
    Runtime,
}

impl Provenance {
    pub fn to_other(&self, path: Vec<Reference>) -> Self {
        Self::FromOther(Box::new(self.clone()), path)
    }
}
