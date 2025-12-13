use thiserror::Error;

use crate::Reference;

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
    #[error("trace: {0}")]
    Previous(String, #[source] Box<EvaluationError>),
}

impl EvaluationError {
    pub fn trace(self, message: String) -> Self {
        Self::Previous(message, Box::new(self))
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
