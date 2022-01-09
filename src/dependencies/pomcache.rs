use crate::dependencies::mavenpom::MavenPom;
use std::collections::HashMap;

/// In-memory pom cache, so we don't have to download them again
pub struct PomCache {
    /// groupid:artifactid:version as key
    cache: HashMap<String, MavenPom>,
}

impl PomCache {
    pub fn new() -> Self {
        PomCache {
            cache: HashMap::new(),
        }
    }

    /*
    pub fn add(&mut self, pom: MavenPom) {
        self.cache[&format!(
            "{}:{}:{}",
            pom.group_id.unwrap(),
            pom.artifact_id,
            pom.version.unwrap()
        )] = pom;
    }*/

    pub fn get(&self, notation: &str) -> Option<&MavenPom> {
        self.cache.get(notation)
    }
}
