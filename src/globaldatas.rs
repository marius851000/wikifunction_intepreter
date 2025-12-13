use std::collections::HashMap;

use anyhow::{Context, bail};

use crate::{DataEntry, Reference};

#[derive(Default)]
pub struct GlobalDatas {
    map: HashMap<Reference, DataEntry>,
}

impl GlobalDatas {
    pub fn add_entry(&mut self, title: &str, content: &str) -> anyhow::Result<()> {
        //TODO: a special type for identifier (as reference was before I added the K value)
        let id = Reference::from_zid(title)
            .with_context(|| format!("Can’t parse {:?} title as reference", title))?;
        if self.map.contains_key(&id) {
            bail!("A page with the title {:?} has already been added", title);
        }
        let entry: DataEntry = serde_json::from_str(content)
            .with_context(|| format!("Can’t parse page {:?} body content", content))?;
        self.map.insert(id, entry);
        Ok(())
    }

    pub fn get(&self, reference: &Reference) -> Option<&DataEntry> {
        self.map.get(reference)
    }
}
