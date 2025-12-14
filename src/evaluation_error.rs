use std::{error::Error, fmt::Display};

use thiserror::Error;

use crate::{DataEntry, Zid};

//TODO: error handling should be much better than that. Will do for now.
#[derive(Error, Debug)]
pub enum EvaluationErrorKind {
    #[error("low level: Not a map")]
    LowLevelNotAMap,
    #[error("low level: Not an array")]
    LowLevelNotAnArray,
    #[error("low level: Not a string")]
    LowLevelNotAString,
    #[error("low level: missing key {0}")]
    MissingKey(Zid),
    #[error("low level: parse ZID")]
    ParseZID(#[source] anyhow::Error),
    #[error("low level: validator result not true")]
    TestSuiteFailed(DataEntry),
    #[error("low level: unimplemented {0}")]
    Unimplemented(String),
    #[error("low level: wrong type {0}, expected {1}")]
    WrongType(Zid, Zid),
    #[error("info: test result: {0:?}")]
    TestResultInfo(DataEntry, #[source] Box<EvaluationErrorKind>),
    #[error("info: trace: {0}")]
    Previous(String, #[source] Box<EvaluationErrorKind>),
}

impl EvaluationErrorKind {
    pub fn trace(self, message: String) -> Self {
        Self::Previous(message, Box::new(self))
    }

    pub fn trace_str(self, message: &str) -> Self {
        self.trace(message.to_string())
    }
}

#[derive(Debug)]
pub struct EvaluationError {
    pub root_kind: EvaluationErrorKind,
    /// Frames will be added by the lower level first, so the order is reversed compared to the typical top-down view
    pub frames: Vec<FrameInfo>,
}

impl EvaluationError {
    pub fn new(root_kind: EvaluationErrorKind) -> Self {
        Self {
            root_kind,
            frames: Vec::new(),
        }
    }

    pub fn add_frame_constructor(mut self, frame: FrameInfo) -> Self {
        self.add_frame(frame);
        self
    }

    pub fn add_frame(&mut self, frame: FrameInfo) {
        self.frames.push(frame);
    }

    pub fn run_with_frame_fun<T, FP: FnOnce() -> Result<T, Self>, FF: FnOnce() -> FrameInfo>(
        frame_provider: FF,
        f: FP,
    ) -> Result<T, Self> {
        f().map_err(|e| e.add_frame_constructor(frame_provider()))
    }

    pub fn run_with_frame<T, F: FnOnce() -> Result<T, Self>>(
        frame: FrameInfo,
        f: F,
    ) -> Result<T, Self> {
        f().map_err(|e| e.add_frame_constructor(frame))
    }
}

impl Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //TODO: a nicer printer
        std::fmt::Debug::fmt(&self, f)
    }
}

impl Error for EvaluationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.root_kind)
    }
}

impl From<EvaluationErrorKind> for EvaluationError {
    fn from(value: EvaluationErrorKind) -> Self {
        Self {
            root_kind: value,
            frames: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FrameInfo {
    Reference(Zid),
    InsideMap(Zid),
    InsideArray(usize),
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{
        EvaluationErrorKind,
        evaluation_error::{EvaluationError, FrameInfo},
    };

    #[test]
    fn test_evaluation_error_info_from() {
        fn return_error_info() -> Result<(), EvaluationError> {
            Err(EvaluationErrorKind::LowLevelNotAMap)?;
            unreachable!();
        }

        return_error_info().unwrap_err().source().unwrap();
    }

    #[test]
    fn test_evaluation_error_info_run_with_frame() {
        assert_eq!(
            EvaluationError::run_with_frame(FrameInfo::InsideArray(1), || -> Result<(), _> {
                Err(EvaluationError::new(EvaluationErrorKind::LowLevelNotAMap))
            })
            .unwrap_err()
            .frames,
            vec![FrameInfo::InsideArray(1)]
        );

        assert_eq!(
            EvaluationError::run_with_frame_fun(
                || FrameInfo::InsideArray(10),
                || -> Result<(), _> {
                    Err(EvaluationError::new(EvaluationErrorKind::LowLevelNotAMap))
                }
            )
            .unwrap_err()
            .frames,
            vec![FrameInfo::InsideArray(10)]
        );
    }
}
