use std::collections::BTreeMap;

use crate::{DataEntry, EvaluationError, Zid};

pub fn recurse_and_replace_placeholder(
    source_entry: &DataEntry,
    to_replace: &BTreeMap<Zid, &DataEntry>, // the function call unwraped
) -> Result<DataEntry, EvaluationError> {
    const Z1K1: Zid = Zid::from_u64s_panic(Some(1), Some(1));
    const Z18K1: Zid = Zid::from_u64s_panic(Some(18), Some(1));

    match source_entry {
        DataEntry::IdMap(map) => {
            if let Some(object_type) = map.get(&Z1K1) {
                //TODO: I think in some case this might be a function call itself
                if object_type.get_str().unwrap() == "Z18" {
                    if let Some(key) = map.get(&Z18K1) {
                        let ref_to_use_to_replace = Zid::from_zid(
                            key.get_str()
                                .map_err(|e| e.trace("inside a Z18K1".to_string()))?,
                        )
                        .map_err(EvaluationError::ParseZID)
                        .map_err(|e| e.trace("inside a Z18K1".to_string()))?;
                        //TODO: what to do in this case? Report the error. How to format it? idk
                        let new_entry = to_replace.get(&ref_to_use_to_replace).unwrap();
                        return Ok((*new_entry).clone());
                    } else {
                        return Err(EvaluationError::MissingKey(Z18K1));
                    }
                }
            }

            let mut new_map = BTreeMap::new();
            for (key, value) in map.iter() {
                new_map.insert(
                    key.to_owned(),
                    recurse_and_replace_placeholder(value, to_replace)
                        .map_err(|e| e.trace(format!("Inside {}", key)))?,
                );
            }
            return Ok(DataEntry::IdMap(new_map));
        }
        DataEntry::Array(array) => {
            let mut new_array = Vec::new();
            for (pos, value) in array.iter().enumerate() {
                new_array.push(
                    recurse_and_replace_placeholder(value, to_replace)
                        .map_err(|e| e.trace(format!("Position {} in the array", pos)))?,
                );
            }
            return Ok(DataEntry::Array(new_array));
        }
        DataEntry::String(v) => Ok(DataEntry::String(v.clone())),
    }
}
