use std::{fmt::Debug, marker::PhantomData};

use crate::{DataEntry, EvaluationError, Reference, Runner};

pub fn parse_zid_string(entry: &DataEntry) -> Result<Reference, EvaluationError> {
    Ok(Reference::from_zid(entry.get_str()?).map_err(|e| EvaluationError::ParseZID(e))?)
}

pub fn parse_string_type(entry: &DataEntry) -> Result<&str, EvaluationError> {
    check_type(entry, zid!(6))?;
    Ok(entry.get_map_entry(&zid!(6, 1))?.get_str()?)
}

pub fn parse_boolean(entry: &DataEntry) -> Result<bool, EvaluationError> {
    let text = entry.get_map_entry(&zid!(40, 1))?.get_str()?;

    match text {
        "Z41" => Ok(true),
        "Z42" => Ok(false),
        _ => todo!("error handling invalid boolean"),
    }
}

/// Return an error if type does not match
pub fn check_type(entry: &DataEntry, id: Reference) -> Result<(), EvaluationError> {
    let read_type = parse_zid_string(entry.get_map_entry(&zid!(1, 1))?)
        .map_err(|e| e.trace_str("parsing the type zid"))?;
    if read_type != id {
        return Err(EvaluationError::WrongType(read_type, id));
    } else {
        return Ok(());
    }
}

pub trait WfParse<'l>: Sized + Debug + Clone {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationError>;
}

#[derive(Debug, Clone)]
pub struct PotentialReference<'l, T: WfParse<'l>> {
    entry: &'l DataEntry,
    phantom: PhantomData<T>,
}

impl<'l, T: WfParse<'l>> PotentialReference<'l, T> {
    /// Note that it only evaluate reference an not function calls or reference to argument
    pub fn evaluate(&self, runner: &'l Runner) -> Result<T, EvaluationError> {
        let entry = match self.entry {
            DataEntry::Array(_) => self.entry,
            DataEntry::String(entry) => {
                runner
                    .get_persistent_object(
                        &Reference::from_zid(entry).map_err(EvaluationError::ParseZID)?,
                    )?
                    .value
            }
            DataEntry::IdMap(entry) => {
                if Reference::from_zid(
                    entry
                        .get(&zid!(1, 1))
                        .ok_or_else(|| EvaluationError::MissingKey(zid!(1, 1)))?
                        .get_str()?,
                )
                .map_err(EvaluationError::ParseZID)?
                    == zid!(9)
                {
                    let reference_to = Reference::from_zid(
                        entry
                            .get(&zid!(9, 1))
                            .ok_or_else(|| EvaluationError::MissingKey(zid!(9, 1)))?
                            .get_str()?,
                    )
                    .map_err(EvaluationError::ParseZID)?;
                    runner.get_persistent_object(&reference_to)?.value
                } else {
                    self.entry
                }
            }
        };

        Ok(T::parse(entry)?)
    }
}

impl<'l, T: WfParse<'l>> WfParse<'l> for PotentialReference<'l, T> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationError> {
        Ok(Self {
            entry,
            phantom: PhantomData::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct WpUntyped<'l> {
    pub entry: &'l DataEntry,
}

impl<'l> WfParse<'l> for WpUntyped<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationError> {
        Ok(Self { entry })
    }
}

/// a Z2
#[derive(Clone, Debug)]
pub struct WfPersistentObject<'l> {
    // assume both id and value are not Z9/reference
    pub id: Reference,
    pub value: &'l DataEntry,
    pub labels: PotentialReference<'l, WpUntyped<'l>>,
    pub aliases: PotentialReference<'l, WpUntyped<'l>>,
    pub short_description: PotentialReference<'l, WpUntyped<'l>>,
}

impl<'l> WfParse<'l> for WfPersistentObject<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationError> {
        check_type(&entry, zid!(2))?;
        Ok(Self {
            id: Reference::from_zid(
                parse_string_type(entry.get_map_entry(&zid!(2, 1))?)
                    .map_err(|e| e.trace_str("parsing id"))?,
            )
            //TODO: Make Reference::from_zid directly return an EvaluationError
            .map_err(EvaluationError::ParseZID)
            .map_err(|e| e.trace_str("parsing id"))?,
            value: entry.get_map_entry(&zid!(2, 2))?,
            labels: entry.get_map_potential_reference(&zid!(2, 3))?,
            aliases: entry.get_map_potential_reference(&zid!(2, 4))?,
            short_description: entry.get_map_potential_reference(&zid!(2, 5))?,
        })
    }
}
