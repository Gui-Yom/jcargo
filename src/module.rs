use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;

use crate::dependencies::Dependencies;
use crate::manifest::{EntrypointDef, ModuleManifest};
use crate::Env;

#[derive(Debug)]
pub struct Module {
    pub dir: PathBuf,
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub base_package: String,
    pub entrypoints: Vec<EntrypointDef>,
    pub dependencies: Dependencies,
}

impl Module {
    pub async fn load(path: &Path, env: &Env) -> Result<Self> {
        let document = fs::read_to_string(path.join("jcargo.toml")).await?;
        let manifest = ModuleManifest::parse(&document, None)?;
        Ok(Self {
            dir: path.to_path_buf(),
            group: manifest.group.unwrap(),
            artifact: manifest.artifact,
            version: manifest.version,
            base_package: manifest.base_package,
            entrypoints: manifest.entrypoints,
            dependencies: Dependencies::from_def(manifest.dependencies, env),
        })
    }

    /// Find an entrypoint with the given name.
    /// If not found, find one with class name.
    pub fn find_entrypoint(&self, name: &str) -> Option<&EntrypointDef> {
        self.entrypoints
            .iter()
            .find(|it| it.name == name)
            .or_else(|| self.entrypoints.iter().find(|it| it.class == name))
    }

    /// Pick the first in the list.
    pub fn pick_entrypoint(&self) -> Option<&EntrypointDef> {
        self.entrypoints.first()
    }
}
