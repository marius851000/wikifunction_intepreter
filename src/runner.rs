use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{
    DataEntry, EvaluationError, GlobalDatas, Zid,
    evaluation_error::Provenance,
    parse_tool::{WfParse, WfPersistentObject, parse_boolean, parse_zid_string},
    recurse_and_replace_placeholder,
};

#[derive(Default, Debug)]
pub struct RunnerOption {
    pub force_use_impl: Option<HashMap<Zid, Zid>>,
}

pub struct Runner {
    datas: Arc<GlobalDatas>,
}

impl Runner {
    pub fn new(datas: Arc<GlobalDatas>) -> Self {
        Self { datas }
    }

    //TODO: check that it isn’t used outside of get_persistent_object
    fn get_entry_for_reference(&self, reference: &Zid) -> Result<&DataEntry, EvaluationError> {
        self.datas
            .get(reference)
            .map_or(Err(EvaluationError::MissingKey(reference.clone())), |v| {
                Ok(v)
            })
    }

    pub fn get_persistent_object(
        &self,
        reference: &Zid,
    ) -> Result<WfPersistentObject<'_>, EvaluationError> {
        Ok(
            WfPersistentObject::parse(self.get_entry_for_reference(reference)?)
                .map_err(|e| e.trace(format!("For object {}", reference)))?,
        )
    }

    pub fn get_true(&self) -> Result<&DataEntry, EvaluationError> {
        Ok(self.get_persistent_object(&zid!(41))?.value)
    }

    pub fn get_false(&self) -> Result<&DataEntry, EvaluationError> {
        Ok(self.get_persistent_object(&zid!(42))?.value)
    }

    pub fn get_bool(&self, b: bool) -> Result<&DataEntry, EvaluationError> {
        if b { self.get_true() } else { self.get_false() }
    }

    // return an error whether an error occur or the test result is incorrect
    pub fn run_test_case(
        &self,
        test_case_persistent: &WfPersistentObject,
        implementation_persistent: &WfPersistentObject,
    ) -> Result<(), EvaluationError> {
        let function_identifier = parse_zid_string(
            implementation_persistent
                .value
                .get_map_entry(&zid!(14, 1)) // implementation->function
                .map_err(|e| e.trace("on the implementation to be tested".to_string()))?,
        )
        .map_err(|e| e.trace("Inside Z14K1 in the implementation to test".to_string()))?;

        let mut runner_option = RunnerOption::default();
        runner_option.force_use_impl = Some({
            let mut m = HashMap::new();
            m.insert(function_identifier, implementation_persistent.id);
            m
        });

        let function_call = test_case_persistent
            .value
            .get_map_entry(&zid!(20, 2))
            .map_err(|e| e.trace("on the test case, inside Z2K2".to_string()))?;

        let test_case_provenance = Provenance::Persistant(test_case_persistent.id);
        let function_call_provenance = Provenance::FromOther(
            Box::new(test_case_provenance),
            vec![zid!(2, 2), zid!(20, 2)],
        );

        let test_fn_result = self
            .run_function_call(function_call, &function_call_provenance, &runner_option)
            .map_err(|e| e.trace("running function to test".to_string()))?;

        (|| {
            let validator = test_case_persistent
                .value
                .get_map_entry(&zid!(20, 3))
                .map_err(|e| e.trace("on the test case, inside Z2K2".to_string()))?;

            // validator is a function call. replace first parameter with the result

            let validator_function_id = parse_zid_string(
                validator
                    .get_map_entry(&zid!(7, 1))
                    .map_err(|e| e.trace_str("on the validator"))?,
            )
            .map_err(|e| e.trace_str("on the validator"))?;

            let inserted_validation_ref =
                Zid::from_u64s_panic(validator_function_id.get_z().map(|x| x.into()), Some(1));

            let mut validator_modified = validator.clone();
            match &mut validator_modified {
                DataEntry::IdMap(map) => {
                    map.insert(inserted_validation_ref, test_fn_result.clone());
                }
                _ => todo!("error handling in that case"),
            }

            let validator_result = self
                .run_function_call(
                    &validator_modified,
                    &Provenance::Runtime,
                    &RunnerOption::default(),
                )
                .map_err(|e| e.trace_str("running the validator function"))?;

            let test_result = parse_boolean(&validator_result)
                .map_err(|e| e.trace_str("parsing the validator result boolean"))?;

            if !test_result {
                return Err(EvaluationError::TestSuiteFailed(test_fn_result.clone()));
            }

            Ok(())
        })()
        .map_err(|e| EvaluationError::TestResultInfo(test_fn_result, Box::new(e)))
    }

    pub fn get_preferred_implementation(
        &self,
        function_persistent: WfPersistentObject,
        option: &RunnerOption,
    ) -> Result<WfPersistentObject<'_>, EvaluationError> {
        if let Some(force_use_impl) = &option.force_use_impl
            && let Some(implementation_id) = force_use_impl.get(&function_persistent.id)
        {
            Ok(self
                .get_persistent_object(&implementation_id)
                .map_err(|e| {
                    e.trace("loading specifically specified implementation".to_string())
                })?)
        } else {
            let implementations_raw = function_persistent
                .value
                .get_map_entry(&zid!(8, 4)) // implementations
                .map_err(|e| e.trace("getting implementations".to_string()))?;

            let implementations_ref = implementations_raw
                .get_array()
                .map_err(|e| e.trace("getting implementations".to_string()))?;

            // It appears connected functions are just function that are directly referenced by it (as opposed to inverse reference)
            // TODO: better handling of typed array
            // TODO: prioritize composition, then built-in, then finally code
            for implementation_key_text in implementations_ref.iter().skip(1) {
                let implementation_key = Zid::from_zid(
                    implementation_key_text
                        .get_str()
                        .map_err(|e| e.trace("Parsing implementation list".to_string()))?,
                )
                .map_err(EvaluationError::ParseZID)
                .map_err(|e| e.trace("processing an implementation reference".to_string()))?;

                let implementation_persistant = self
                    .get_persistent_object(&implementation_key)
                    .map_err(|e| {
                        e.trace("trying to get a referrenced implementation".to_string())
                    })?;

                let implementation_map = implementation_persistant
                    .value
                    .get_map()
                    .map_err(|e| e.trace("processing an implementation".to_string()))?;

                // check if it have a composition implementation
                if let Some(_) = implementation_map.get(&zid!(14, 2)) {
                    // composition implementation
                    return Ok(implementation_persistant);
                }

                if let Some(_) = implementation_map.get(&zid!(14, 4)) {
                    // builtin implementation
                    return Ok(implementation_persistant);
                }
            }

            // TODO: code
            return Err(EvaluationError::Unimplemented(format!(
                "code and builtins (and fail if none found) (for {})",
                function_persistent.id
            )));
        }
    }

    pub fn run_function_call(
        &self,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        const Z7K1: Zid = Zid::from_u64s_panic(Some(7), Some(1));
        let function_id = parse_zid_string(
            function_call
                .get_map_entry(&Z7K1)
                .map_err(|e| e.trace("trying to get the function to call".to_string()))?,
        )
        .map_err(|e| e.trace("trying to get the function to call".to_string()))?;
        let function_persistant = self
            .get_persistent_object(&function_id)
            .map_err(|e| e.trace("trying to get the function to call".to_string()))?;

        let implementation_persistant =
            self.get_preferred_implementation(function_persistant, option)?;

        let implementation_provenance = Provenance::Persistant(implementation_persistant.id);

        return Ok(self
            .run_implementation(
                implementation_persistant.value,
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
            })?);
    }

    pub fn run_implementation(
        &self,
        implementation: &DataEntry,
        implementation_provenance: &Provenance,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        let impl_map = implementation.get_map()?;
        if let Some(composition) = impl_map.get(&zid!(14, 2)) {
            return self.run_composition(
                composition,
                &implementation_provenance.to_other(vec![zid!(14, 2)]),
                function_call,
                function_call_provenance,
                option,
            );
        };

        if let Some(builtin) = impl_map.get(&zid!(14, 4)) {
            return self.run_builtin(builtin, function_call, function_call_provenance, option);
        }

        todo!("code implementation and error if no impl are present")
    }

    pub fn run_composition(
        &self,
        composition: &DataEntry,
        composition_provenance: &Provenance,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        // algorithm:
        // 1. replace all Z18 by their actual value
        // 2. work top-down, recursively. If entry is Z7, perform the function call. If not, recurse deeper

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
        const Z1K1: Zid = Zid::from_u64s_panic(Some(1), Some(1));

        match entry {
            DataEntry::IdMap(map) => {
                if let Some(object_type) = map.get(&Z1K1) {
                    if object_type
                        .get_str()
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

    pub fn run_builtin(
        &self,
        builtin: &DataEntry,
        function_call: &DataEntry,
        function_call_provenance: &Provenance,
        option: &RunnerOption,
    ) -> Result<DataEntry, EvaluationError> {
        let implementation_id = builtin
            .get_map_entry(&zid!(6, 1))
            .map_err(|e| e.trace("Getting the implementation id to run".to_string()))?
            .get_str()
            .map_err(|e| e.trace("Getting the implementation id to run".to_string()))?;
        // let’s force the use of composition implementation as much as posible to reduce the built-ins that needs to be implemented
        let impl_to_use = match implementation_id {
            // string equality
            "Z966" => Some(Zid::from_u64s_panic(Some(17569), None)),
            // list equality. Some weird behavior around typed list. Might be a problem in certain cases.
            "Z989" => Some(Zid::from_u64s_panic(Some(15872), None)),
            _ => None,
        };

        if let Some(impl_to_use) = impl_to_use {
            let implementation_persistant = self
                .get_persistent_object(&impl_to_use)
                .map_err(|e| e.trace("Getting the implementation to run".to_string()))?;
            let implementation_provenance = Provenance::Persistant(impl_to_use);

            return self.run_implementation(
                implementation_persistant.value,
                &implementation_provenance,
                function_call,
                function_call_provenance,
                option,
            );
        }

        let provenance_other = function_call_provenance.to_other(Vec::new());

        match implementation_id {
            // If
            "Z902" => {
                let condition = self
                    .recurse_call_function(
                        function_call.get_map_entry(&zid!(802, 1))?,
                        &provenance_other,
                        option,
                    )
                    .map_err(|e| e.trace_str("parsing condition"))?;
                let condition =
                    parse_boolean(&condition).map_err(|e| e.trace_str("parsing condition"))?;

                let entry_to_use = if condition {
                    zid!(802, 2)
                } else {
                    zid!(802, 3)
                };

                let result = self
                    .recurse_call_function(
                        function_call.get_map_entry(&entry_to_use)?,
                        &provenance_other,
                        option,
                    )
                    .map_err(|e| e.trace(format!("evaluating result for {:?}", condition)))?;

                return Ok(result);
            }
            // Is empty (typed) list
            "Z913" => {
                let list = self.recurse_call_function(
                    function_call.get_map_entry(&zid!(813, 1))?,
                    &provenance_other,
                    option,
                )?;

                // <= 1 cause typed list store the type as the first index
                return Ok(self.get_bool(list.get_array()?.len() <= 1)?.clone());
            }
            // boolean equality
            "Z944" => {
                let boolean1 = self
                    .recurse_call_function(
                        function_call.get_map_entry(&zid!(844, 1))?,
                        &provenance_other,
                        option,
                    )
                    .map_err(|e| e.trace_str("parsing first boolean"))?;
                let boolean1 =
                    parse_boolean(&boolean1).map_err(|e| e.trace_str("parsing first boolean"))?;
                let boolean2 = self
                    .recurse_call_function(
                        function_call.get_map_entry(&zid!(844, 2))?,
                        &provenance_other,
                        option,
                    )
                    .map_err(|e| e.trace_str("parsing second boolean"))?;
                let boolean2 =
                    parse_boolean(&boolean2).map_err(|e| e.trace_str("parsing first boolean"))?;

                return Ok(self.get_bool(boolean1 == boolean2)?.clone());
            }
            _ => {
                return Err(EvaluationError::Unimplemented(format!(
                    "built-in {}",
                    implementation_id
                )));
            }
        }
    }
}
