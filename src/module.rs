use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;

use crate::dependencies::Dependencies;
use crate::manifest::{EntrypointDef, ModuleManifest};
use crate::Env;

#[derive(Debug)]
pub struct Module {
    /// Module root directory
    pub dir: PathBuf,
    /// Artifact group
    pub group: String,
    /// Artifact id
    pub artifact: String,
    /// Project version
    pub version: String,
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

    pub fn source_dir(&self) -> PathBuf {
        self.dir.join("src")
    }

    pub fn resources_dir(&self) -> PathBuf {
        self.dir.join("resources")
    }

    pub fn target_dir(&self) -> PathBuf {
        self.dir.join("target")
    }

    pub fn classes_dir(&self) -> PathBuf {
        self.target_dir().join("classes")
    }

    pub fn docs_dir(&self) -> PathBuf {
        self.target_dir().join("docs")
    }

    pub fn artifacts_dir(&self) -> PathBuf {
        self.target_dir().join("artifacts")
    }
}
