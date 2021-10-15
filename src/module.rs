use std::iter;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{fs, process};
use walkdir::WalkDir;

use crate::backend::{CompilationBackend, Runtime};
use crate::dependencies::{Dependencies, Dependency};
use crate::manifest::{CompleteDependencyDef, EntrypointDef, ModuleManifest};
use crate::{Env, Task};

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
    pub async fn load(path: &Path, env: &Env) -> Self {
        let document = fs::read_to_string(path.join("jcargo.toml"))
            .await
            .expect("Can't read jcargo.toml file");
        let manifest = ModuleManifest::parse(&document, None);
        Self {
            dir: path.to_path_buf(),
            group: manifest.group.unwrap(),
            artifact: manifest.artifact,
            version: manifest.version,
            base_package: manifest.base_package,
            entrypoints: manifest.entrypoints,
            dependencies: Dependencies::from_def(manifest.dependencies, env),
        }
    }

    #[async_recursion::async_recursion]
    pub async fn execute_task(&self, task: Task, env: &Env) {
        match task {
            Task::Check => {
                println!("   Checking dependencies");
                self.check().await;
                println!("   Done !")
            }
            Task::Build => {
                self.execute_task(Task::Check, env).await;
                println!("   Compiling {} v{} <path>", self.artifact, self.version);

                let instant = Instant::now();
                self.build(env.comp_backend).await;

                println!(
                    "   Finished build. (took {} ms)",
                    instant.elapsed().as_millis()
                );
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
            Task::Clean => {}
        }
    }

    pub async fn check(&self) {
        self.setup_all_dependencies().await;
    }

    pub async fn build(&self, backend: CompilationBackend) {
        let mut cmd: process::Command = backend.command();
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
        ]);

        // Collect dependencies include paths
        let cp = self
            .dependencies
            .iter_compile()
            .map(|it| format!("{}/{}", self.dir.display(), it.classpath()))
            .chain(iter::once(output_dir))
            .reduce(|a, b| format!("{};{}", a, b))
            .unwrap();
        cmd.arg(&cp);
        println!("compile classpath: {}", &cp);

        self.collect_source_files().for_each(|it| {
            cmd.arg(it);
        });

        cmd.env(
            "JDKTOOLS_HOME",
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

        let class;
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

        let mut cmd = Runtime::Java.command();
        cmd.args([
            "-Xshare:on",
            "-XX:TieredStopAtLevel=1",
            "-XX:+UseSerialGC",
            "-cp",
        ]);

        // Collect dependencies include paths
        let cp = self
            .dependencies
            .iter_runtime()
            .map(|it| format!("{}/{}", self.dir.display(), it.classpath()))
            .chain(iter::once(output_dir))
            .reduce(|a, b| format!("{};{}", a, b))
            .unwrap();
        cmd.arg(&cp);

        println!("runtime classpath: {}", &cp);

        cmd.arg(class.unwrap())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();
    }

    async fn generate_jar_manifest(&self, entrypoint_name: Option<String>) {
        let manifest = self.dir.join("target/classes/META-INF/MANIFEST.MF");

        fs::write(
            &manifest,
            r"
        Manifest-Version: 1.0
        Main-Class: Main
        ",
        )
        .await;
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

    /// Setup all dependencies from any scope
    async fn setup_all_dependencies(&self) {
        let client_ = Arc::new(reqwest::Client::new());

        let mut handles = Vec::with_capacity(self.dependencies.len());
        for dep_ in self.dependencies.iter() {
            // Manually clone
            let dep = dep_.clone();
            let client = Arc::clone(&client_);
            let dir = self.dir.join("libs");
            fs::create_dir_all(&dir).await;

            let task = tokio::spawn(async move {
                match dep {
                    Dependency::Repo(repodep) => {
                        // TODO download dependencies to a known place
                        // TODO verify file hash for update

                        let file_path = dir.join(&repodep.get_file());

                        if file_path.exists() {
                            println!("Dependency '{}' OK", repodep);
                            return;
                        }

                        println!("Downloading '{}' from {}", repodep, repodep.repo.name);

                        let mut res = client.get(&repodep.download_url()).send().await.unwrap();

                        let file = fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&file_path)
                            .await
                            .expect("Can't create/open file");
                        let mut buf_file = BufWriter::new(file);
                        while let Some(chunk) = res.chunk().await.expect("Can't get a chunk") {
                            buf_file.write(&chunk).await.unwrap();
                        }
                        buf_file.flush().await.expect("Can't write file to disk");
                        println!("Downloaded {}", repodep);
                    }
                    _ => {
                        todo!()
                    }
                }
            });
            handles.push(task);
        }
        for x in handles {
            x.await.expect("Error when waiting for dependency setup");
        }
    }
}
