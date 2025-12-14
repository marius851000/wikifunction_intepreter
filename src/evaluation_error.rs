use std::{
    error::Error,
    fmt::{Display, Write},
};

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
    pub frames: Vec<TraceInfo>,
}

impl EvaluationError {
    pub fn new(root_kind: EvaluationErrorKind) -> Self {
        Self {
            root_kind,
            frames: Vec::new(),
        }
    }

    pub fn add_frame_constructor(mut self, frame: TraceInfo) -> Self {
        self.add_frame(frame);
        self
    }

    pub fn add_frame(&mut self, frame: TraceInfo) {
        self.frames.push(frame);
    }

    pub fn run_with_frame_fun<T, FP: FnOnce() -> Result<T, Self>, FF: FnOnce() -> TraceInfo>(
        frame_provider: FF,
        f: FP,
    ) -> Result<T, Self> {
        f().map_err(|e| e.add_frame_constructor(frame_provider()))
    }

    pub fn run_with_frame<T, F: FnOnce() -> Result<T, Self>>(
        frame: TraceInfo,
        f: F,
    ) -> Result<T, Self> {
        f().map_err(|e| e.add_frame_constructor(frame))
    }

    /// frames are expected from top to bottom (they will be reversed before being put in the frame list)
    pub fn run_with_frame_fun_multiple<
        T,
        FP: FnOnce() -> Result<T, Self>,
        FF: FnOnce() -> Vec<TraceInfo>,
    >(
        frames_provider: FF,
        f: FP,
    ) -> Result<T, Self> {
        f().map_err(|mut e| {
            for frame in frames_provider().into_iter().rev() {
                e.add_frame(frame);
            }
            e
        })
    }
}

impl Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut has_jumped_line = false;
        for frame in self.frames.iter().rev() {
            if frame.should_start_new_section_before() && !has_jumped_line {
                f.write_char('\n')?;
            }
            has_jumped_line = false;
            std::fmt::Display::fmt(frame, f)?;
            if frame.should_start_new_section_after() {
                f.write_char('\n')?;
                has_jumped_line = true;
            }
        }
        Ok(())
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
pub enum TraceInfo {
    Reference(Zid),
    InsideMap(Zid),
    InsideArray(usize),
    ProcessingResult(DataEntry),
    InsideInput(String), //TODO: to an enum!
}

impl TraceInfo {
    pub fn should_start_new_section_before(&self) -> bool {
        matches!(self, Self::Reference(_) | Self::InsideInput(_))
    }

    pub fn should_start_new_section_after(&self) -> bool {
        matches!(self, Self::ProcessingResult(_))
    }
}

impl std::fmt::Display for TraceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reference(zid) => {
                std::fmt::Display::fmt(zid, f)?;
            }
            Self::InsideMap(zid) => {
                f.write_str("->")?;
                std::fmt::Display::fmt(zid, f)?;
            }
            Self::InsideArray(pos) => {
                f.write_char('[')?;
                std::fmt::Display::fmt(pos, f)?;
                f.write_char(']')?;
            }
            Self::InsideInput(name) => {
                f.write_str("input ")?;
                std::fmt::Debug::fmt(name, f)?;
                f.write_char(' ')?;
            }
            Self::ProcessingResult(data) => {
                f.write_str("processing result ")?;
                std::fmt::Debug::fmt(data, f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{
        EvaluationErrorKind,
        evaluation_error::{EvaluationError, TraceInfo},
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
            EvaluationError::run_with_frame(TraceInfo::InsideArray(1), || -> Result<(), _> {
                Err(EvaluationError::new(EvaluationErrorKind::LowLevelNotAMap))
            })
            .unwrap_err()
            .frames,
            vec![TraceInfo::InsideArray(1)]
        );

        assert_eq!(
            EvaluationError::run_with_frame_fun(
                || TraceInfo::InsideArray(10),
                || -> Result<(), _> {
                    Err(EvaluationError::new(EvaluationErrorKind::LowLevelNotAMap))
                }
            )
            .unwrap_err()
            .frames,
            vec![TraceInfo::InsideArray(10)]
        );
    }
}
