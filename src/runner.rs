use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{
    DataEntry, EvaluationError, GlobalDatas, Reference,
    evaluation_error::Provenance,
    parse_tool::{get_persistant_object_id, get_persistant_object_value, parse_zid_string},
    recurse_and_replace_placeholder,
};

#[derive(Default, Debug)]
pub struct RunnerOption {
    pub force_use_impl: Option<HashMap<Reference, Reference>>,
}

pub struct Runner {
    datas: Arc<GlobalDatas>,
}

impl Runner {
    pub fn new(datas: Arc<GlobalDatas>) -> Self {
        Self { datas }
    }

    pub fn get_entry_for_reference(
        &self,
        reference: &Reference,
    ) -> Result<&DataEntry, EvaluationError> {
        self.datas
            .get(reference)
            .map_or(Err(EvaluationError::MissingKey(reference.clone())), |v| {
                Ok(v)
            })
    }

    // should return a Z22 "Evaluation result"
    pub fn run_test_case(
        &self,
        test_case_persistant: &DataEntry,
        implementation_persistant: &DataEntry,
    ) -> Result<DataEntry, EvaluationError> {
        const Z14K1: Reference = Reference::from_u64s_panic(Some(14), Some(1)); // implementation->function
        const Z2K2: Reference = Reference::from_u64s_panic(Some(2), Some(2)); // persistent object->value

        let implementation_identifier = get_persistant_object_id(implementation_persistant)
            .map_err(|e| e.trace("getting the id of the implementation".to_string()))?;
        let function_identifier = parse_zid_string(
            implementation_persistant
                .get_map_entry(&Z2K2)
                .map_err(|e| e.trace("on the implementation to be tested".to_string()))?
                .get_map_entry(&Z14K1)
                .map_err(|e| {
                    e.trace("on the implementation to be tested, inside Z2K2".to_string())
                })?,
        )
        .map_err(|e| e.trace("Inside Z14K1 in the implementation to test".to_string()))?;

        let mut runner_option = RunnerOption::default();
        runner_option.force_use_impl = Some({
            let mut m = HashMap::new();
            m.insert(function_identifier, implementation_identifier);
            m
        });

        let test_case_persistant_id = get_persistant_object_id(test_case_persistant)
            .map_err(|e| e.trace("processing persistant test case".to_string()))?;
        const Z20K2: Reference = Reference::from_u64s_panic(Some(20), Some(2));
        let function_call = test_case_persistant
            .get_map_entry(&Z2K2)
            .map_err(|e| e.trace("on the test case".to_string()))?
            .get_map_entry(&Z20K2)
            .map_err(|e| e.trace("on the test case, inside Z2K2".to_string()))?;

        let test_case_provenance = Provenance::Persistant(test_case_persistant_id);
        let function_call_provenance =
            Provenance::FromOther(Box::new(test_case_provenance), vec![Z2K2, Z20K2]);

        self.run_function_call(function_call, &function_call_provenance, &runner_option)
            .map_err(|e| e.trace("running function to test".to_string()))?;
        todo!();
    }

    pub fn run_function_call(
        &self,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        const Z7K1: Reference = Reference::from_u64s_panic(Some(7), Some(1));
        let function_id = parse_zid_string(
            function_call
                .get_map_entry(&Z7K1)
                .map_err(|e| e.trace("trying to get the function to call".to_string()))?,
        )
        .map_err(|e| e.trace("trying to get the function to call".to_string()))?;
        let function_persistant = self
            .get_entry_for_reference(&function_id)
            .map_err(|e| e.trace("trying to get the function to call".to_string()))?;

        let function = get_persistant_object_value(function_persistant)
            .map_err(|e| e.trace("processing persistant function to call".to_string()))?;

        let implementation_persistant = if let Some(force_use_impl) = &option.force_use_impl
            && let Some(implementation_id) = force_use_impl.get(&function_id)
        {
            self.get_entry_for_reference(&implementation_id)
                .map_err(|e| e.trace("loading specifically specified implementation".to_string()))?
        } else {
            println!("{:?}, {:?}", function_id, option);
            todo!("pick an implementation from a function");
        };

        let implementation = get_persistant_object_value(implementation_persistant)
            .map_err(|e| e.trace("processing persistant implementation to call".to_string()))?;
        let implementation_provenance =
            Provenance::Persistant(get_persistant_object_id(implementation_persistant).map_err(
                |e| e.trace("processing persistant implementation to calll".to_string()),
            )?);

        self.run_implementation(
            implementation,
            &implementation_provenance,
            function_call,
            function_call_provenance,
            option,
        )
        .map_err(|e| {
            e.trace(format!(
                "calling implementation {:?}",
                implementation_provenance
            ))
        })?;
        todo!();
    }

    pub fn run_implementation(
        &self,
        implementation: &DataEntry,
        implementation_provenance: &Provenance,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        const Z14K2: Reference = Reference::from_u64s_panic(Some(14), Some(2));
        match implementation.get_map_entry(&Z14K2) {
            Ok(composition) => {
                return self.run_composition(
                    composition,
                    &implementation_provenance.to_other(vec![Z14K2]),
                    function_call,
                    function_call_provenance,
                    option,
                );
            }
            Err(EvaluationError::MissingKey(_)) => (),
            Err(err) => return Err(err),
        };

        todo!("code implementation and error if neither are present")
    }

    pub fn run_composition(
        &self,
        composition: &DataEntry,
        composition_provenance: &Provenance,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        const Z7K1: Reference = Reference::from_u64s_panic(Some(7), Some(1));

        // algorithm:
        // 1. replace all Z18 by their actual value
        // 2. work top-down, recursively. If entry is Z7, perform the function call. If not, recurse deeper

        let function_id = function_call.get_map_entry(&Z7K1).map_err(|e| {
            e.trace(format!(
                "inside function call from {:?}",
                function_call_provenance
            ))
        })?;

        let composition_with_substitution_done = match function_call {
            DataEntry::IdMap(to_replace) => {
                //TODO: remove unwrap
                recurse_and_replace_placeholder(composition, to_replace).unwrap()
            }
            _ => return Err(EvaluationError::LowLevelNotAMap),
        };

        Ok(self
            .recurse_call_function(
                &composition_with_substitution_done,
                composition_provenance,
                option,
            )
            .map_err(|e| {
                e.trace(format!(
                    "Calling the composition from {:?}",
                    function_call_provenance
                ))
            })?)
    }

    pub fn recurse_call_function(
        &self,
        entry: &DataEntry,
        //TODO: make provenance trace all inside stuff. That should make it quite deep.
        provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        const Z1K1: Reference = Reference::from_u64s_panic(Some(1), Some(1));

        match entry {
            DataEntry::IdMap(map) => {
                if let Some(object_type) = map.get(&Z1K1) {
                    if object_type
                        .get_string()
                        .map_err(|e| e.trace("Inside Z1K1".to_string()))?
                        == "Z7"
                    {
                        return self.run_function_call(&entry, provenance, option);
                    }
                }

                let mut new_map = BTreeMap::new();
                for (key, value) in map.iter() {
                    new_map.insert(
                        key.to_owned(),
                        self.recurse_call_function(value, provenance, option)
                            .map_err(|e| e.trace(format!("Inside {}", key)))?,
                    );
                }
                return Ok(DataEntry::IdMap(new_map));
            }
            DataEntry::Array(array) => {
                let mut new_array = Vec::new();

                for (pos, entry) in array.iter().enumerate() {
                    new_array.push(
                        self.recurse_call_function(entry, provenance, option)
                            .map_err(|e| e.trace(format!("At array position {}", pos)))?,
                    )
                }

                Ok(DataEntry::Array(new_array))
            }
            DataEntry::String(s) => Ok(DataEntry::String(s.to_string())),
        }
    }
}
