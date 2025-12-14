use std::{collections::BTreeMap, fmt::Debug, marker::PhantomData};

use crate::{DataEntry, EvaluationErrorKind, Runner, Zid};

pub fn parse_zid_string(entry: &DataEntry) -> Result<Zid, EvaluationErrorKind> {
    Ok(Zid::from_zid(entry.get_str()?).map_err(|e| EvaluationErrorKind::ParseZID(e))?)
}

pub fn parse_string_type(entry: &DataEntry) -> Result<&str, EvaluationErrorKind> {
    check_type(entry, zid!(6))?;
    Ok(entry.get_map_entry(&zid!(6, 1))?.get_str()?)
}

pub fn parse_string_permissive(entry: &DataEntry) -> Result<&str, EvaluationErrorKind> {
    if let Ok(v) = entry.get_str() {
        return Ok(v);
    } else {
        return parse_string_type(entry);
    }
}

pub fn raw_string_to_object_string(input: String) -> DataEntry {
    DataEntry::IdMap({
        let mut map = BTreeMap::new();
        map.insert(zid!(1, 1), DataEntry::String("Z6".to_string()));
        map.insert(zid!(6, 1), DataEntry::String(input));
        map
    })
}

pub fn parse_boolean(entry: &DataEntry) -> Result<bool, EvaluationErrorKind> {
    let text = entry.get_map_entry(&zid!(40, 1))?.get_str()?;

    match text {
        "Z41" => Ok(true),
        "Z42" => Ok(false),
        _ => todo!("error handling invalid boolean"),
    }
}

/// Return an error if type does not match
pub fn check_type(entry: &DataEntry, id: Zid) -> Result<(), EvaluationErrorKind> {
    let read_type = parse_zid_string(entry.get_map_entry(&zid!(1, 1))?)
        .map_err(|e| e.trace_str("parsing the type zid"))?;
    if read_type != id {
        return Err(EvaluationErrorKind::WrongType(read_type, id));
    } else {
        return Ok(());
    }
}

#[derive(Debug, Clone)]
pub enum MaybeOwned<'l, T> {
    Owned(T),
    Referenced(&'l T),
}

impl<'l, T> MaybeOwned<'l, T> {
    pub fn from_owned(owned: T) -> Self {
        Self::Owned(owned)
    }

    pub fn from_reference(reference: &'l T) -> Self {
        Self::Referenced(&reference)
    }

    pub fn get(&self) -> &T {
        match self {
            Self::Owned(o) => &o,
            Self::Referenced(r) => r,
        }
    }
}

pub trait WfParse<'l>: Sized + Debug + Clone {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind>;
}

#[derive(Debug, Clone)]
pub struct PotentialReference<'l, T: WfParse<'l>> {
    entry: &'l DataEntry,
    phantom: PhantomData<T>,
}

impl<'l, T: WfParse<'l>> PotentialReference<'l, T> {
    pub fn new(entry: &'l DataEntry) -> Self {
        Self {
            entry,
            phantom: PhantomData::default(),
        }
    }

    /// Note that it only evaluate reference an not function calls or reference to argument
    pub fn evaluate(&self, runner: &'l Runner) -> Result<T, EvaluationErrorKind> {
        Ok(match self.entry {
            DataEntry::Array(_) => T::parse(self.entry)?,
            DataEntry::String(entry) => {
                runner
                    .get_persistent_object(
                        &Zid::from_zid(entry).map_err(EvaluationErrorKind::ParseZID)?,
                    )?
                    .value
            }
            DataEntry::IdMap(entry) => {
                if Zid::from_zid(
                    entry
                        .get(&zid!(1, 1))
                        .ok_or_else(|| EvaluationErrorKind::MissingKey(zid!(1, 1)))?
                        .get_str()?,
                )
                .map_err(EvaluationErrorKind::ParseZID)?
                    == zid!(9)
                {
                    let reference_to = Zid::from_zid(
                        entry
                            .get(&zid!(9, 1))
                            .ok_or_else(|| EvaluationErrorKind::MissingKey(zid!(9, 1)))?
                            .get_str()?,
                    )
                    .map_err(EvaluationErrorKind::ParseZID)?;
                    runner.get_persistent_object(&reference_to)?.value
                } else {
                    T::parse(self.entry)?
                }
            }
        })
    }

    pub fn get_reference(&self) -> Result<Zid, EvaluationErrorKind> {
        Ok(match self.entry {
            DataEntry::Array(_) => return Err(EvaluationErrorKind::LowLevelNotAMap),
            DataEntry::String(entry) => {
                Zid::from_zid(entry).map_err(EvaluationErrorKind::ParseZID)?
            }
            DataEntry::IdMap(entry) => {
                check_type(self.entry, zid!(9))?;
                Zid::from_zid(
                    entry
                        .get(&zid!(9, 1))
                        .ok_or_else(|| EvaluationErrorKind::MissingKey(zid!(9, 1)))?
                        .get_str()?,
                )
                .map_err(EvaluationErrorKind::ParseZID)?
            }
        })
    }
}

impl<'l, T: WfParse<'l>> WfParse<'l> for PotentialReference<'l, T> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            entry,
            phantom: PhantomData::default(),
        })
    }
}

// Note: those only work on reference. For returning thing across function that might contain thing with not enought lifetime, directly return a cloned DataEntry.

#[derive(Debug, Clone)]
pub struct WfUntyped<'l> {
    pub entry: &'l DataEntry,
}

impl<'l> WfParse<'l> for WfUntyped<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self { entry })
    }
}

pub const ZID_PERSISTENT_OBJECT_VALUE: Zid = zid!(2, 2);

/// a Z2
#[derive(Clone, Debug)]
pub struct WfPersistentObject<'l, T: WfParse<'l>> {
    // assume both id and value are not Z9/reference (to guarantee absence of double reference)
    pub id: Zid,
    pub value: T,
    pub labels: PotentialReference<'l, WfUntyped<'l>>,
    pub aliases: PotentialReference<'l, WfUntyped<'l>>,
    pub short_description: PotentialReference<'l, WfUntyped<'l>>,
}

impl<'l, T: WfParse<'l>> WfParse<'l> for WfPersistentObject<'l, T> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        check_type(&entry, zid!(2))?;
        Ok(Self {
            id: Zid::from_zid(
                parse_string_type(entry.get_map_entry(&zid!(2, 1))?)
                    .map_err(|e| e.trace_str("parsing id"))?,
            )
            //TODO: Make Reference::from_zid directly return an EvaluationError
            .map_err(EvaluationErrorKind::ParseZID)
            .map_err(|e| e.trace_str("parsing id"))?,
            value: T::parse(entry.get_map_entry(&ZID_PERSISTENT_OBJECT_VALUE)?)
                .map_err(|e| e.trace_str("parsing value"))?,
            labels: entry.get_map_potential_reference(&zid!(2, 3))?,
            aliases: entry.get_map_potential_reference(&zid!(2, 4))?,
            short_description: entry.get_map_potential_reference(&zid!(2, 5))?,
        })
    }
}

pub const ZID_IMPLEMENTATION_FUNCTION: Zid = zid!(14, 1);

/// A Z14
#[derive(Clone, Debug)]
pub struct WfImplementation<'l> {
    pub function: PotentialReference<'l, WfFunction<'l>>,
    pub composition: Option<PotentialReference<'l, WfUntyped<'l>>>,
    pub code: Option<PotentialReference<'l, WfUntyped<'l>>>,
    pub builtin: Option<PotentialReference<'l, WfUntyped<'l>>>, //TODO: A Function?
}

impl<'l> WfParse<'l> for WfImplementation<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            function: entry.get_map_potential_reference(&zid!(14, 1))?,
            composition: entry.get_map_potential_reference_option(&zid!(14, 2))?,
            code: entry.get_map_potential_reference_option(&zid!(14, 3))?,
            builtin: entry.get_map_potential_reference_option(&zid!(14, 4))?,
        })
    }
}

pub const ZID_FUNCTION_IDENTITY: Zid = zid!(8, 5);

/// A Z8
#[derive(Clone, Debug)]
pub struct WfFunction<'l> {
    pub arguments: PotentialReference<'l, WfUntyped<'l>>,
    pub return_type: PotentialReference<'l, WfUntyped<'l>>,
    pub testers: PotentialReference<'l, WfUntyped<'l>>,
    pub implementations: PotentialReference<'l, WfUntyped<'l>>,
    pub identity: PotentialReference<'l, WfFunction<'l>>,
}

impl<'l> WfParse<'l> for WfFunction<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            arguments: entry.get_map_potential_reference(&zid!(8, 1))?,
            return_type: entry.get_map_potential_reference(&zid!(8, 2))?,
            testers: entry.get_map_potential_reference(&zid!(8, 3))?,
            implementations: entry.get_map_potential_reference(&zid!(8, 4))?,
            identity: entry.get_map_potential_reference(&ZID_FUNCTION_IDENTITY)?,
        })
    }
}

/// A Z20
pub const ZID_TEST_CASE_CALL: Zid = zid!(20, 2);
pub const ZID_TEST_CASE_RESULT_VALIDATION: Zid = zid!(20, 3);
#[derive(Clone, Debug)]
pub struct WfTestCase<'l> {
    pub function: PotentialReference<'l, WfFunction<'l>>,
    pub call: PotentialReference<'l, WfFunctionCall<'l>>,
    pub result_validation: PotentialReference<'l, WfFunctionCall<'l>>,
}

impl<'l> WfParse<'l> for WfTestCase<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            function: entry.get_map_potential_reference(&zid!(20, 1))?,
            call: entry.get_map_potential_reference(&ZID_TEST_CASE_CALL)?,
            result_validation: entry
                .get_map_potential_reference(&ZID_TEST_CASE_RESULT_VALIDATION)?,
        })
    }
}

pub const ZID_FUNCTION_CALL_FUNCTION: Zid = zid!(7, 1);
/// A Z7
#[derive(Debug, Clone)]
pub struct WfFunctionCall<'l> {
    pub function: PotentialReference<'l, WfFunction<'l>>,
    pub args: BTreeMap<Zid, &'l DataEntry>,
}

impl<'l> WfFunctionCall<'l> {
    pub fn get_arg(&self, key: &Zid) -> Result<&'l DataEntry, EvaluationErrorKind> {
        Ok(self
            .args
            .get(key)
            .ok_or_else(|| EvaluationErrorKind::MissingKey(key.to_owned()))?)
    }
}
impl<'l> WfParse<'l> for WfFunctionCall<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        let function = entry.get_map_potential_reference(&ZID_FUNCTION_CALL_FUNCTION)?;
        let mut args = BTreeMap::new();
        for (k, v) in entry.get_map()? {
            if k == &zid!(1, 1) || k == &zid!(7, 1) {
                continue;
            }
            args.insert(k.clone(), v);
        }
        Ok(Self { function, args })
    }
}

/// A Z4
#[derive(Debug, Clone)]
pub struct WfType<'l> {
    pub identity: PotentialReference<'l, WfType<'l>>,
    pub keys: PotentialReference<'l, WfUntyped<'l>>, //TODO: typed list
    pub validator: PotentialReference<'l, WfFunction<'l>>,
    pub equality: PotentialReference<'l, WfFunction<'l>>,
    pub display_function: PotentialReference<'l, WfFunction<'l>>,
    pub reading_function: PotentialReference<'l, WfFunction<'l>>,
    pub type_converters_to_code: PotentialReference<'l, WfUntyped<'l>>,
    pub type_converters_from_code: PotentialReference<'l, WfUntyped<'l>>,
}

impl<'l> WfParse<'l> for WfType<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            identity: entry.get_map_potential_reference(&zid!(4, 1))?,
            keys: entry.get_map_potential_reference(&zid!(4, 2))?,
            validator: entry.get_map_potential_reference(&zid!(4, 3))?,
            equality: entry.get_map_potential_reference(&zid!(4, 4))?,
            display_function: entry.get_map_potential_reference(&zid!(4, 5))?,
            reading_function: entry.get_map_potential_reference(&zid!(4, 6))?,
            type_converters_to_code: entry.get_map_potential_reference(&zid!(4, 7))?,
            type_converters_from_code: entry.get_map_potential_reference(&zid!(4, 8))?,
        })
    }
}

/// meant as a high level representation of a typed list whose type is known in advance
#[derive(Debug, Clone)]
pub struct WfTypedList<'l, T: WfParse<'l>> {
    pub elements: Vec<T>,
    phantom: PhantomData<WfFunction<'l>>,
}

impl<'l, T: WfParse<'l>> WfParse<'l> for WfTypedList<'l, T> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        let mut result = Vec::new();
        for (pos, value) in entry.get_array()?.iter().enumerate() {
            result
                .push(T::parse(value).map_err(|e| e.trace(format!("at array position {}", pos)))?);
        }
        Ok(Self {
            elements: result,
            phantom: PhantomData::default(),
        })
    }
}

/// A Z3
#[derive(Debug, Clone)]
pub struct WfKey<'l> {
    pub value_type: PotentialReference<'l, WfType<'l>>,
    pub key_id: &'l str,
    pub label: PotentialReference<'l, WfUntyped<'l>>,
    pub is_identity: PotentialReference<'l, WfUntyped<'l>>,
}

impl<'l> WfParse<'l> for WfKey<'l> {
    fn parse(entry: &'l DataEntry) -> Result<Self, EvaluationErrorKind> {
        Ok(Self {
            value_type: entry.get_map_potential_reference(&zid!(3, 1))?,
            key_id: parse_string_permissive(entry.get_map_entry(&zid!(3, 2))?)?,
            label: entry.get_map_potential_reference(&zid!(3, 3))?,
            is_identity: entry.get_map_potential_reference(&zid!(3, 4))?,
        })
    }
}
