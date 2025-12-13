use std::{fs::File, io::BufReader};

use interpreter::DataEntry;

fn main() {
    let file =
        BufReader::new(File::open("./wikifunctionswiki-20251201-pages-meta-current.xml").unwrap());
    for result in parse_mediawiki_dump_reboot::parse(file) {
        let result = result.unwrap();
        if result.model.unwrap() == "zobject" {
            println!("{}", result.title);
            let _entry: DataEntry = serde_json::from_str(&result.text).unwrap();
        }
    }
}
