use std::{fs::File, io::BufReader, sync::Arc};

use interpreter::{GlobalDatas, Reference, Runner};

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
    runner
        .run_test_case(
            runner
                .get_entry_for_reference(&Reference::from_zid("Z8130").unwrap())
                .unwrap(),
            runner
                .get_entry_for_reference(&Reference::from_zid("Z913").unwrap())
                .unwrap(),
        )
        .map_err(|e| e.trace("running the test test case".to_string()))?;

    Ok(())
}
