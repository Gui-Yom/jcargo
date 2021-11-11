use std::iter;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{fs, process};
use walkdir::WalkDir;

use crate::backend::DocumentationBackend;
use crate::dependencies::Dependency;
use crate::{CompilationBackend, Env, Module, PackageBackend, Runtime, Task};

#[async_recursion::async_recursion]
pub async fn execute_task(task: Task, module: &Module, env: &Env) {
    match task {
        Task::Check => {
            println!("   Checking dependencies");
            check(module).await;
            println!("   Done !")
        }
        Task::Build => {
            execute_task(Task::Check, module, env).await;
            println!(
                "   Compiling {} v{} <path>",
                module.artifact, module.version
            );

            let instant = Instant::now();
            build(module, env.comp_backend).await;

            println!(
                "   Finished build. (took {} ms)",
                instant.elapsed().as_millis()
            );
        }
        Task::Run { entrypoint } => {
            execute_task(Task::Build, module, env).await;
            println!("   Running 'Main'");
            let instant = Instant::now();

            run(module, entrypoint).await;

            println!(
                "   Execution finished. (took {} ms)",
                instant.elapsed().as_millis()
            );
        }
        Task::Doc => {
            println!("   Building documentation");
            let instant = Instant::now();

            build_doc(module, DocumentationBackend::JdkJavadoc).await;

            println!(
                "   Finished build. (took {} ms)",
                instant.elapsed().as_millis()
            );
        }
        Task::Package {
            sources,
            docs,
            entrypoint,
        } => {
            execute_task(Task::Build, module, env).await;
            if docs {
                execute_task(Task::Doc, module, env).await;
            }

            println!(
                "   Packaging jar{}{} ...",
                if sources { " +sources" } else { "" },
                if docs { " +docs" } else { "" }
            );
            let instant = Instant::now();

            package(module, env.package_backend, sources, docs, entrypoint).await;

            println!(
                "   Packaging finished. (took {} ms)",
                instant.elapsed().as_millis()
            );
        }
        Task::Clean => {
            fs::remove_dir_all(module.dir.join("target")).await.unwrap();
        }
        _ => {}
    }
}

pub async fn check(module: &Module) {
    setup_all_dependencies(module).await;
}

pub async fn build(module: &Module, backend: CompilationBackend) {
    let mut cmd: process::Command = backend.command();
    let output_dir = format!("{}/target/classes", module.dir.display());
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
    let cp = module
        .dependencies
        .iter_compile()
        .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
        .chain(iter::once(output_dir))
        .reduce(|a, b| format!("{};{}", a, b))
        .unwrap();
    cmd.arg(&cp);
    println!("compile classpath: {}", &cp);

    collect_files(module.dir.join("src")).for_each(|it| {
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

pub async fn run(module: &Module, entrypoint_name: Option<String>) {
    let output_dir = format!("{}/target/classes", module.dir.display());

    let class;
    match entrypoint_name {
        Some(name) => class = module.find_entrypoint(&name).map(|it| &it.class),
        None => {
            class = module.pick_entrypoint().map(|it| &it.class);
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
    let cp = module
        .dependencies
        .iter_runtime()
        .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
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

pub async fn build_doc(module: &Module, backend: DocumentationBackend) {
    let mut cmd: process::Command = backend.command();

    tokio::fs::create_dir_all(module.dir.join("target/docs"))
        .await
        .unwrap();

    cmd.arg("-d")
        .arg(&format!("{}/target/docs", module.dir.display()))
        .arg("-cp");

    // Collect dependencies include paths
    let cp = module
        .dependencies
        .iter_compile()
        .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
        .reduce(|a, b| format!("{};{}", a, b))
        .unwrap();
    cmd.arg(&cp);
    println!("compile classpath: {}", &cp);

    collect_files(module.dir.join("src")).for_each(|it| {
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

pub async fn package(
    module: &Module,
    backend: PackageBackend,
    sources: bool,
    docs: bool,
    entrypoint: Option<String>,
) {
    let base_dir = Arc::new(module.dir.clone());
    let artifact_dir = module.dir.join("target/artifacts");
    let artifact_base_name = Arc::new(format!(
        "{}/{}-{}",
        artifact_dir.display(),
        module.artifact,
        module.version
    ));

    let entrypoint_class = entrypoint
        .as_ref()
        .map(|it| module.find_entrypoint(it))
        .flatten()
        .map(|it| it.class.clone());

    tokio::fs::create_dir_all(artifact_dir).await.unwrap();

    let base_dir2 = base_dir.clone();
    let artifact_base_name2 = artifact_base_name.clone();
    let mut handles = Vec::new();
    handles.push(tokio::spawn(async move {
        let mut cmd: process::Command = backend.command();

        // Create mode
        cmd.arg("-c")
            .arg("-f")
            .arg(&format!("{}.jar", artifact_base_name2));

        if let Some(entrypoint) = entrypoint_class {
            cmd.arg("-e").arg(&entrypoint);
        }

        collect_files(base_dir2.join("target/classes"))
            .chain(collect_files(base_dir2.join("resources")))
            .for_each(|it| {
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
    }));

    if sources {
        let base_dir2 = base_dir.clone();
        let artifact_base_name2 = artifact_base_name.clone();
        handles.push(tokio::spawn(async move {
            let mut cmd: process::Command = backend.command();

            // Create mode
            cmd.arg("-c")
                .arg("-f")
                .arg(&format!("{}-sources.jar", artifact_base_name2));

            collect_files(base_dir2.join("src"))
                .chain(collect_files(base_dir2.join("resources")))
                .for_each(|it| {
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
        }));
    }

    if docs {
        let base_dir2 = base_dir.clone();
        let artifact_base_name2 = artifact_base_name.clone();
        handles.push(tokio::spawn(async move {
            let mut cmd: process::Command = backend.command();

            // Create mode
            cmd.arg("-c")
                .arg("-f")
                .arg(&format!("{}-docs.jar", artifact_base_name2));

            collect_files(base_dir2.join("docs")).for_each(|it| {
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
        }));
    }

    for x in handles {
        x.await.unwrap();
    }
}

fn collect_files<P: AsRef<Path>>(path: P) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|it| it.file_type().is_file())
        .map(|it| it.path().to_path_buf())
}

/// Setup all dependencies from any scope
async fn setup_all_dependencies(module: &Module) {
    let client_ = Arc::new(reqwest::Client::new());

    let mut handles = Vec::with_capacity(module.dependencies.len());
    for dep_ in module.dependencies.iter() {
        // Manually clone
        let dep = dep_.clone();
        let client = Arc::clone(&client_);
        let dir = module.dir.join("libs");
        fs::create_dir_all(&dir).await.unwrap();

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

async fn generate_jar_manifest(module: &Module, entrypoint_name: Option<String>) {
    let manifest = module.dir.join("target/classes/META-INF/MANIFEST.MF");

    fs::write(
        &manifest,
        r"
        Manifest-Version: 1.0
        Main-Class: Main
        ",
    )
    .await
    .unwrap();
}
