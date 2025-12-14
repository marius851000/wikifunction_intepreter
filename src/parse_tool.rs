use crate::{DataEntry, EvaluationError, Reference};

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

#[derive(Clone)]
pub struct WfPersistentObject<'l> {
    pub id: Reference,
    pub value: &'l DataEntry,
    pub labels: &'l DataEntry,
    pub aliases: &'l DataEntry,
    pub short_description: &'l DataEntry,
}

impl<'l> WfPersistentObject<'l> {
    pub fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationError> {
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
            labels: entry.get_map_entry(&zid!(2, 3))?,
            aliases: entry.get_map_entry(&zid!(2, 4))?,
            short_description: entry.get_map_entry(&zid!(2, 5))?,
        })
    }
}
