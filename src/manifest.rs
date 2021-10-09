use serde::Deserialize;

/// Root of the TOML document
#[derive(Debug, Deserialize)]
pub struct ModuleManifest {
    // Group can be inferred from the root manifest
    pub group: Option<String>,
    pub artifact: String,
    pub version: String,
    #[serde(default)]
    pub base_package: String,
    #[serde(flatten)]
    pub extra_info: ExtraInfo,
    // May be a library
    #[serde(default)]
    pub entrypoints: Vec<EntrypointDef>,
    // No dependencies is ok
    #[serde(default)]
    pub dependencies: DependenciesDef,
}

impl ModuleManifest {
    /// If parent is None, the manifest is the root manifest
    pub fn parse(document: &str, parent: Option<&ModuleManifest>) -> Self {
        let mut document: ModuleManifest =
            toml::from_str(document).expect("Can't parse this document as valid module manifest");
        if let Some(parent) = parent {
            if document.group.is_none() {
                document.group = parent.group.clone();
            }
        }
        document
    }

    pub fn validate(&self) -> bool {
        todo!()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ExtraInfo {
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: String,
}

#[derive(Debug, Deserialize)]
pub struct EntrypointDef {
    /// Name used when invoking the run task
    #[serde(default)]
    pub name: String,
    /// Fully qualified name of the main class to launch
    pub class: String,
}

impl EntrypointDef {
    pub fn validate(&self) -> bool {
        if self.name.contains(" ") {
            return false;
        }
        return true;
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct DependenciesDef {
    #[serde(default)]
    pub compile: Vec<DependencyDef>,
    #[serde(default)]
    pub runtime: Vec<DependencyDef>,
    #[serde(default, rename = "compileRuntime")]
    pub compile_runtime: Vec<DependencyDef>,
    #[serde(default)]
    pub transitive: Vec<DependencyDef>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencyDef {
    ShortNotation(String),
    CompleteNotation(CompleteDependencyDef),
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompleteDependencyDef {
    pub group: String,
    pub artifact: String,
    pub version: String,
}

impl From<DependencyDef> for CompleteDependencyDef {
    fn from(dd: DependencyDef) -> Self {
        match dd {
            DependencyDef::ShortNotation(full) => {
                let mut pieces = full.split(":");
                Self {
                    group: pieces.next().unwrap().to_string(),
                    artifact: pieces.next().unwrap().to_string(),
                    version: pieces.next().unwrap().to_string(),
                }
            }
            DependencyDef::CompleteNotation(complete) => complete,
        }
    }
}
