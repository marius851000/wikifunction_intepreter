use serde::{Deserialize, de::Visitor};

use crate::{
    EvaluationErrorKind, Runner, Zid,
    parse_tool::{PotentialReference, WfParse},
};
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
        let mut result: BTreeMap<Zid, DataEntry> = BTreeMap::new();

        while let Some(entry) = map.next_entry::<Zid, DataEntry>()? {
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
    IdMap(BTreeMap<Zid, DataEntry>),
    Array(Vec<DataEntry>), // TODO: the language does not really have array, it’s a representation of a typed linked list for compresion, I guess. Decide whether I keep that as-is or I transform it to IdMap at runtime
}

impl DataEntry {
    pub fn get_map_entry(&self, reference: &Zid) -> Result<&DataEntry, EvaluationErrorKind> {
        match self.get_map()?.get(reference) {
            Some(v) => Ok(v),
            None => Err(EvaluationErrorKind::MissingKey(reference.clone())),
        }
    }

    pub fn get_map_entry_option(
        &self,
        reference: &Zid,
    ) -> Result<Option<&DataEntry>, EvaluationErrorKind> {
        Ok(self.get_map()?.get(reference))
    }

    pub fn get_map_potential_reference<'l, T: WfParse<'l>>(
        &'l self,
        reference: &'l Zid,
    ) -> Result<PotentialReference<'l, T>, EvaluationErrorKind> {
        Ok(PotentialReference::parse(self.get_map_entry(reference)?)?)
    }

    pub fn get_map_potential_reference_option<'l, T: WfParse<'l>>(
        &'l self,
        reference: &'l Zid,
    ) -> Result<Option<PotentialReference<'l, T>>, EvaluationErrorKind> {
        if let Some(v) = self.get_map_entry_option(reference)? {
            Ok(Some(PotentialReference::parse(v)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_map(&self) -> Result<&BTreeMap<Zid, DataEntry>, EvaluationErrorKind> {
        match self {
            Self::IdMap(map) => Ok(map),
            _ => Err(EvaluationErrorKind::LowLevelNotAMap),
        }
    }

    pub fn get_array(&self) -> Result<&Vec<DataEntry>, EvaluationErrorKind> {
        match self {
            Self::Array(array) => Ok(array),
            _ => Err(EvaluationErrorKind::LowLevelNotAnArray),
        }
    }

    pub fn get_str(&self) -> Result<&str, EvaluationErrorKind> {
        match self {
            Self::String(s) => Ok(s.as_ref()),
            _ => Err(EvaluationErrorKind::LowLevelNotAString),
        }
    }

    /// transform the representation into something the running code can parse. Take care of typed list, that are only vec for the json format!
    pub fn reify(&self, runner: &Runner) -> Result<DataEntry, EvaluationErrorKind> {
        //TODO: should we follow reference here? Confused...
        /*let self_pointed = PotentialReference::<WfUntyped<'_>>::new(self)
            .evaluate(runner)
            .map_err(|e| e.trace_str("looking up references"))?
            .entry;
        */

        let well_typed_pair = Self::IdMap({
            let mut map = BTreeMap::new();
            map.insert(zid!(1, 1), Self::String("Z7".to_string()));
            map.insert(zid!(7, 1), Self::String("Z882".to_string()));
            map.insert(zid!(882, 1), Self::String("Z39".to_string()));
            map.insert(zid!(882, 2), Self::String("Z2".to_string()));
            map
        });

        match self {
            // the result should be ordered. As are the BTreeMap.
            Self::String(value) => {
                // just an identity, it seems. Not sure. See Z15796
                Ok(Self::String(value.to_string()))
            }
            Self::IdMap(map) => {
                let mut result = Vec::new();
                result.push(well_typed_pair.clone());

                // typed pair are represented with Z1K1, K1 and K2
                for (k, v) in map {
                    result.push(Self::IdMap({
                        let mut map = BTreeMap::new();
                        map.insert(zid!(1, 1), well_typed_pair.clone());
                        map.insert(
                            Zid::from_u64s_panic(None, Some(1)),
                            Self::IdMap({
                                let mut map = BTreeMap::new();
                                map.insert(zid!(1, 1), Self::String("Z39".to_string()));
                                map.insert(zid!(39, 1), Self::String(k.to_string()));
                                map
                            }),
                        );
                        map.insert(
                            Zid::from_u64s_panic(None, Some(2)),
                            v.reify(runner)
                                .map_err(|e| e.trace(format!("inside {}", k)))?,
                        );
                        map
                    }));
                }

                Ok(Self::Array(result))
            }
            Self::Array(_array) => {
                // let’s have fun transforming that to linked list...
                todo!("manage list");
            }
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

    use crate::{DataEntry, Zid};

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
                    Zid::from_u64s(Some(10), Some(1)).unwrap(),
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
                m.insert(Zid::from_u64s(Some(1), None).unwrap(), {
                    let mut m2 = BTreeMap::new();
                    m2.insert(
                        Zid::from_u64s(Some(2), None).unwrap(),
                        DataEntry::String("Z3".to_string()),
                    );
                    m2.insert(
                        Zid::from_u64s(Some(4), None).unwrap(),
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
