use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{fs, process};
use walkdir::WalkDir;

use manifest::{CompleteDependencyDef, EntrypointDef, ModuleManifest};

use crate::backend::Backend;
use crate::{Env, Task};

pub mod manifest;

#[derive(Debug)]
pub struct Module {
    pub dir: PathBuf,
    pub name: String,
    pub version: String,
    pub entrypoints: Vec<EntrypointDef>,
    pub dependencies: Vec<CompleteDependencyDef>,
}

impl Module {
    pub async fn load(path: &Path) -> Self {
        let document = fs::read_to_string(path.join("jcargo.toml"))
            .await
            .expect("Can't read jcargo.toml file");
        let manifest = ModuleManifest::new(&document);
        Self {
            dir: path.to_path_buf(),
            name: manifest.name,
            version: manifest.version,
            entrypoints: manifest.entrypoints,
            dependencies: manifest
                .dependencies
                .into_iter()
                .map(|it| it.into())
                .collect(),
        }
    }

    #[async_recursion::async_recursion]
    pub async fn execute_task(&self, task: Task, env: Env) {
        match task {
            Task::Build => {
                println!("   Compiling {} v{} <path>", self.name, self.version);

                let instant = Instant::now();
                self.build(env.backend).await;

                println!(
                    "   Finished build. (took {} ms)",
                    instant.elapsed().as_millis()
                );
                println!();
            }

            Task::Jar => {
                self.execute_task(Task::Build, env).await;
            }

            Task::Run { entrypoint } => {
                self.execute_task(Task::Build, env).await;
                println!("   Running 'Main'");
                let instant = Instant::now();

                self.run(entrypoint).await;

                println!(
                    "   Execution finished. (took {} ms)",
                    instant.elapsed().as_millis()
                );
            }
        }
    }

    pub async fn build(&self, backend: Backend) {
        self.download_dependencies().await;

        let mut cmd: process::Command = backend.command().into();
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
        .spawn()
        .unwrap()
        .wait_with_output()
        .await
        .unwrap();
    }

    fn collect_source_files(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(self.dir.join("src"))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|it| it.file_type().is_file())
            .map(|it| it.path().to_path_buf())
    }

    pub async fn run(&self, entrypoint_name: Option<String>) {
        let output_dir = format!("{}/target/classes", self.dir.display());

        let mut class = None;
        match entrypoint_name {
            Some(name) => class = self.find_entrypoint(&name).map(|it| &it.class),
            None => {
                class = self.pick_entrypoint().map(|it| &it.class);
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
            .await
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

    async fn download_dependencies(&self) {
        //https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-api/2.14.1/

        let client_ = Arc::new(reqwest::Client::new());

        let mut handles = Vec::new();
        for dep_ in self.dependencies.iter() {
            let dep = dep_.clone();
            let client = client_.clone();
            handles.push(tokio::spawn(async move {
                // TODO download dependencies to a known place
                // TODO verify file hash for update

                if PathBuf::from(dep.get_file()).exists() {
                    return;
                }

                let url = format!(
                    "https://repo.maven.apache.org/maven2/{}/{}",
                    dep.get_path(),
                    dep.get_file()
                );
                let mut res = client.get(&url).send().await.unwrap();

                let file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(dep.get_file())
                    .await
                    .expect("Can't create/open file");
                let mut buf_file = BufWriter::new(file);
                while let Some(chunk) = res.chunk().await.expect("Can't get a chunk") {
                    buf_file.write(&chunk).await.unwrap();
                }
                buf_file.flush().await.expect("Can't write file to disk");
                println!("Downloaded {}", dep);
            }))
        }
        for x in handles {
            x.await.expect("Error when waiting for download");
        }
    }
}
