use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{fs, process};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::backend::Backend;

#[derive(Debug)]
pub struct Module {
    pub dir: PathBuf,
    pub config: ModuleConfig,
}

impl Module {
    pub fn load(path: &Path) -> Self {
        let document = fs::read_to_string(path.join("jcargo.toml")).unwrap();
        let config = toml::from_str(&document).unwrap();
        Self {
            dir: path.to_path_buf(),
            config,
        }
    }

    pub fn compile(&self, backend: Backend) {
        let mut cmd = backend.command();
        let output_dir = format!("{}/target/classes", self.dir.display());
        cmd.args([
            "-source",
            "17",
            "-target",
            "17",
            "-encoding",
            "UTF-8",
            "-Xlint",
            "-d",
            &output_dir,
            "-cp",
            &output_dir,
        ]);

        self.collect_source_files().for_each(|it| {
            cmd.arg(it);
        });

        cmd.env(
            "JAVAC_HOME",
            "C:/Program Files/Eclipse Foundation/jdk-17.0.0.35-hotspot",
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    }

    fn collect_source_files(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(self.dir.join("src"))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|it| it.file_type().is_file())
            .map(|it| it.path().to_path_buf())
    }

    pub fn run(&self, entrypoint_name: Option<String>) {
        let output_dir = format!("{}/target/classes", self.dir.display());

        let mut class = None;
        match entrypoint_name {
            Some(str) => {
                for e in self.config.entrypoints.iter() {
                    if e.name == str {
                        class = Some(&e.class);
                        break;
                    }
                }
                if class.is_none() {
                    for e in self.config.entrypoints.iter() {
                        if e.class == str {
                            class = Some(&e.class);
                            break;
                        }
                    }
                }
            }
            None => {
                // Pick the first in the list
                for e in self.config.entrypoints.iter() {
                    class = Some(&e.class);
                    break;
                }
            }
        };

        if class.is_none() {
            println!("Can't find entrypoint");
            return;
        }

        process::Command::new("java")
            .args([
                "-Xshare:on",
                "-XX:TieredStopAtLevel=1",
                "-XX:+UseSerialGC",
                "-cp",
                &output_dir,
                class.unwrap(),
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();
    }

    fn generate_jar_manifest(&self, entrypoint_name: Option<String>) {
        let manifest = self.dir.join("target/classes/META-INF/MANIFEST.MF");

        fs::write(
            &manifest,
            r"
        Manifest-Version: 1.0
        Main-Class: Main
        ",
        );
    }

    fn download_dependencies(&self) {
        //https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-api/2.14.1/
    }
}

#[derive(Debug, Deserialize)]
pub struct ModuleConfig {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub entrypoints: Vec<EntrypointDefinition>,
    pub dependencies: HashMap<String, DependencyDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct EntrypointDefinition {
    #[serde(default)]
    pub name: String,
    pub class: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencyDefinition {
    SimpleVersion(String),
    Complete(CompleteDependencyConfig),
}

#[derive(Debug, Deserialize)]
pub struct CompleteDependencyConfig {
    pub version: String,
}
