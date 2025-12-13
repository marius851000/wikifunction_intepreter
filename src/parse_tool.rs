use crate::{DataEntry, EvaluationError, Reference};

pub fn parse_zid_string(entry: &DataEntry) -> Result<Reference, EvaluationError> {
    Ok(Reference::from_zid(entry.get_str()?).map_err(|e| EvaluationError::ParseZID(e))?)
}

pub fn parse_boolean(entry: &DataEntry) -> Result<bool, EvaluationError> {
    const Z40K1: Reference = Reference::from_u64s_panic(Some(40), Some(1));
    let text = entry.get_map_entry(&Z40K1)?.get_str()?;

    match text {
        "Z41" => Ok(true),
        "Z42" => Ok(false),
        _ => todo!("error handling invalid boolean"),
    }
}

pub fn get_persistant_object_value(entry: &DataEntry) -> Result<&DataEntry, EvaluationError> {
    const Z2K2: Reference = Reference::from_u64s_panic(Some(2), Some(2));
    Ok(entry.get_map_entry(&Z2K2)?)
}

pub fn get_persistant_object_id(entry: &DataEntry) -> Result<Reference, EvaluationError> {
    const Z2K1: Reference = Reference::from_u64s_panic(Some(2), Some(1));
    const Z6K1: Reference = Reference::from_u64s_panic(Some(6), Some(1));

    let zid_entry = entry
        .get_map_entry(&Z2K1)?
        .get_map_entry(&Z6K1)
        .map_err(|e| e.trace("Inside Z2K1".to_string()))?;

    parse_zid_string(zid_entry).map_err(|e| e.trace("Inside K2K1->Z6K1".to_string()))
}
