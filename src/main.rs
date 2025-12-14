use std::{fs::File, io::BufReader, sync::Arc};

use wikifunctions_interpreter::{
    GlobalDatas, Reference, Runner, RunnerOption, parse_tool::get_persistant_object_value,
};

fn main() -> anyhow::Result<()> {
    let file =
        BufReader::new(File::open("./wikifunctionswiki-20251201-pages-meta-current.xml").unwrap());
    let mut gb = GlobalDatas::default();
    for result in parse_mediawiki_dump_reboot::parse(file) {
        let result = result.unwrap();
        if result.model.unwrap() == "zobject" {
            gb.add_entry(&result.title, &result.text).unwrap();
        }
    }

    let runner = Runner::new(Arc::new(gb));

    for test_to_run in [
        // is empty list
        "Z8130", "Z8131",

        // other stuff
        // "Z10071"
    ] {
        let test_case_persistant = runner
            .get_entry_for_reference(&Reference::from_zid(test_to_run).unwrap())
            .unwrap();
        let function_id_string = get_persistant_object_value(test_case_persistant)
            .unwrap()
            .get_map_entry(&Reference::from_u64s_panic(Some(20), Some(1)))
            .unwrap()
            .get_str()
            .unwrap();
        let function_persistant = runner
            .get_entry_for_reference(&Reference::from_zid(function_id_string).unwrap())
            .unwrap();
        let implementation_persistant = runner
            .get_preferred_persistant_implementation(function_persistant, &RunnerOption::default())
            .unwrap();
        runner
            .run_test_case(test_case_persistant, implementation_persistant)
            .map_err(|e| e.trace(format!("running the test case {}", test_to_run)))?;
    }

    Ok(())
}
