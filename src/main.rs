use std::{fs::File, io::BufReader, sync::Arc};

use wikifunctions_interpreter::{GlobalDatas, Runner, RunnerOption, Zid, parse_tool::WfTestCase};

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
        let test_case_persistent = runner
            .get_persistent_object::<WfTestCase>(&Zid::from_zid(test_to_run).unwrap())
            .unwrap();
        let function = test_case_persistent
            .value
            .function
            .evaluate(&runner)
            .unwrap();
        let implementation_persistant = runner
            .get_preferred_implementation(&function, &RunnerOption::default())
            .unwrap();
        runner
            .run_test_case(&test_case_persistent, &implementation_persistant)
            .map_err(|e| e.trace(format!("running the test case {}", test_to_run)))?;
    }

    Ok(())
}
