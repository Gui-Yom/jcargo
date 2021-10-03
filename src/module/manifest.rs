use std::fmt::{Display, Formatter};

use serde::Deserialize;

/// Root of the TOML document
#[derive(Debug, Deserialize)]
pub struct ModuleManifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub entrypoints: Vec<EntrypointDef>,
    pub dependencies: Vec<DependencyDef>,
}

impl ModuleManifest {
    pub fn new(document: &str) -> Self {
        toml::from_str(document).expect("Can't parse this document as valid module manifest")
    }

    pub fn validate(&self) -> bool {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
pub struct EntrypointDef {
    #[serde(default)]
    pub name: String,
    pub class: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencyDef {
    GradleNotation(String),
    MavenNotation(CompleteDependencyDef),
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompleteDependencyDef {
    pub group: String,
    pub artifact: String,
    pub version: String,
}

impl CompleteDependencyDef {
    pub fn get_path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group.replace(".", "/"),
            self.artifact,
            self.version
        )
    }

    pub fn get_file(&self) -> String {
        format!("{}-{}.jar", self.artifact, self.version)
    }
}

impl From<DependencyDef> for CompleteDependencyDef {
    fn from(dd: DependencyDef) -> Self {
        match dd {
            DependencyDef::GradleNotation(full) => {
                let mut pieces = full.split(":");
                Self {
                    group: pieces.next().unwrap().to_string(),
                    artifact: pieces.next().unwrap().to_string(),
                    version: pieces.next().unwrap().to_string(),
                }
            }
            DependencyDef::MavenNotation(complete) => complete,
        }
    }
}

impl Display for CompleteDependencyDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.group, self.artifact, self.version)
    }
}
