use serde::{Deserialize, de::Visitor};

use crate::{EvaluationError, Reference};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct DataEntryVisitor {}

impl<'de> Visitor<'de> for DataEntryVisitor {
    type Value = DataEntry;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a list, a dict or a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        return self.visit_string(v.to_string());
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        return Ok(DataEntry::String(v));
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        return self.visit_string(v.to_string());
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result: BTreeMap<Reference, DataEntry> = BTreeMap::new();

        while let Some(entry) = map.next_entry::<Reference, DataEntry>()? {
            result.insert(entry.0, entry.1);
        }

        Ok(DataEntry::IdMap(result))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut result: Vec<DataEntry> = Vec::new();
        while let Some(entry) = seq.next_element::<DataEntry>()? {
            result.push(entry);
        }
        Ok(DataEntry::Array(result))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DataEntry {
    String(String),
    IdMap(BTreeMap<Reference, DataEntry>),
    Array(Vec<DataEntry>),
}

impl DataEntry {
    pub fn get_map_entry(&self, reference: &Reference) -> Result<&DataEntry, EvaluationError> {
        match self.get_map()?.get(reference) {
            Some(v) => Ok(v),
            None => Err(EvaluationError::MissingKey(reference.clone())),
        }
    }

    pub fn get_map(&self) -> Result<&BTreeMap<Reference, DataEntry>, EvaluationError> {
        match self {
            Self::IdMap(map) => Ok(map),
            _ => Err(EvaluationError::LowLevelNotAMap),
        }
    }

    pub fn get_array(&self) -> Result<&Vec<DataEntry>, EvaluationError> {
        match self {
            Self::Array(array) => Ok(array),
            _ => Err(EvaluationError::LowLevelNotAnArray),
        }
    }

    pub fn get_str(&self) -> Result<&str, EvaluationError> {
        match self {
            Self::String(s) => Ok(s.as_ref()),
            _ => Err(EvaluationError::LowLevelNotAString),
        }
    }
}

impl<'de> Deserialize<'de> for DataEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DataEntryVisitor::default())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{DataEntry, Reference};

    #[test]
    fn test_deserialize() {
        assert_eq!(
            serde_json::from_str::<DataEntry>(
                "{
                    \"Z10K1\": \"a\\nb\"
                }"
            )
            .unwrap(),
            DataEntry::IdMap({
                let mut m = BTreeMap::new();
                m.insert(
                    Reference::from_u64s(Some(10), Some(1)).unwrap(),
                    DataEntry::String("a\nb".to_string()),
                );
                m
            })
        );
        assert_eq!(
            serde_json::from_str::<DataEntry>(
                "{
                    \"Z1\": {
                        \"Z2\": \"Z3\",
                        \"Z4\": \"Z2\"
                    }
                }"
            )
            .unwrap(),
            DataEntry::IdMap({
                let mut m = BTreeMap::new();
                m.insert(Reference::from_u64s(Some(1), None).unwrap(), {
                    let mut m2 = BTreeMap::new();
                    m2.insert(
                        Reference::from_u64s(Some(2), None).unwrap(),
                        DataEntry::String("Z3".to_string()),
                    );
                    m2.insert(
                        Reference::from_u64s(Some(4), None).unwrap(),
                        DataEntry::String("Z2".to_string()),
                    );
                    DataEntry::IdMap(m2)
                });
                m
            })
        );

        assert_eq!(
            serde_json::from_str::<DataEntry>(
                "[
                    \"Z1\",
                    \"Z2\"
                ]"
            )
            .unwrap(),
            DataEntry::Array(vec![
                DataEntry::String("Z1".to_string()),
                DataEntry::String("Z2".to_string())
            ])
        );

        assert!(serde_json::from_str::<DataEntry>("{1: \"Z2\"}").is_err());
    }
}
