use std::future::Future;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{fs, process};
use walkdir::WalkDir;

use crate::backend::{DocumentationBackend, KotlinCompilationBackend};
use crate::dependencies::Dependency;
use crate::download::download_file;
use crate::{Env, JavaCompilationBackend, Module, PackageBackend, Runtime, Task};

pub async fn execute_task(
    task: Task,
    env: &Env,
    module_resolver: impl Future<Output = Result<Module>>,
) {
    match task {
        Task::Init { group, artifact } => {
            println!("Init '{}:{}' in the current directory", group, artifact);
            let manifest_path = PathBuf::from("jcargo.toml");
            if manifest_path.exists() {
                println!("Error: There is already a manifest in the current directory.");
                return;
            }
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&manifest_path)
                .await
                .unwrap();

            let mut buf = BufWriter::new(file);
            buf.write(
                format!(
                    r#"
        group = "{}"
        artifact = "{}"
        version = "0.1.0"
        "#,
                    group, artifact
                )
                .as_ref(),
            )
            .await
            .unwrap();
            buf.flush().await.unwrap();
        }
        _ => {
            let module = module_resolver.await.unwrap();
            execute_task_mod(task, env, &module).await;
        }
    }
}

#[async_recursion::async_recursion]
pub async fn execute_task_mod(task: Task, env: &Env, module: &Module) {
    match task {
        Task::Check => {
            println!("   Checking dependencies");
            let instant = Instant::now();

            check(module).await;

            println!("   Done. (took {} ms)", instant.elapsed().as_millis());
        }
        Task::Build => {
            execute_task_mod(Task::Check, env, module).await;
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
            execute_task_mod(Task::Build, env, module).await;
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

            build_doc(module, env.doc_backend).await;

            println!(
                "   Finished building docs. (took {} ms)",
                instant.elapsed().as_millis()
            );
        }
        Task::Package {
            sources,
            docs,
            entrypoint,
        } => {
            execute_task_mod(Task::Build, env, module).await;
            if docs {
                execute_task_mod(Task::Doc, env, module).await;
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
            println!("Cleaned project (removed 'target' dir).")
        }
        _ => {}
    }
}

pub async fn check(module: &Module) {
    setup_all_dependencies(module).await;
}

pub async fn build(module: &Module, backend: JavaCompilationBackend) {
    let source_dir = module.source_dir();
    let output_dir = module.classes_dir();
    fs::create_dir_all(&output_dir).await.unwrap();

    // We need to build kotlin first since it can handle java source files
    // Javac can't handle kotlin source files
    // Required for Java <-> Kotlin references

    let mut sources = collect_files(&source_dir, Some(&[".kt", ".java"])).peekable();
    // Pass if no kotlin sources
    if sources.peek().is_some() {
        println!("Detected kotlin sources ...");

        let mut ktcmd = KotlinCompilationBackend::Kotlinc.command();
        ktcmd.args([
            "-jvm-target",
            "17",
            "-language-version",
            "1.6",
            "-d",
            &output_dir.display().to_string(),
            "-cp",
        ]);

        // Collect dependencies include paths
        let cp = module
            .dependencies
            .iter_compile()
            .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
            .chain(iter::once(output_dir.display().to_string()))
            .reduce(|a, b| format!("{};{}", a, b))
            .unwrap();
        ktcmd.arg(&cp);
        println!("compile classpath: {}", &cp);

        sources.for_each(|it| {
            ktcmd.arg(it);
        });

        ktcmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();

        println!("Compiled kotlin sources.");
    }

    let mut sources = collect_files(&source_dir, Some(&[".java"])).peekable();
    // Pass if no java sources
    if sources.peek().is_some() {
        println!("Detected java sources ...");

        let mut cmd: process::Command = backend.command();
        cmd.args([
            "-source",
            "17",
            "-target",
            "17",
            "-encoding",
            "UTF-8",
            "-Xlint",
            "-d",
            &output_dir.display().to_string(),
            "-cp",
        ]);

        // Collect dependencies include paths
        let cp = module
            .dependencies
            .iter_compile()
            .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
            .chain(iter::once(output_dir.display().to_string()))
            .reduce(|a, b| format!("{};{}", a, b))
            .unwrap();
        cmd.arg(&cp);
        println!("compile classpath: {}", &cp);

        sources.for_each(|it| {
            cmd.arg(it);
        });

        cmd.stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();

        println!("Compiled kotlin sources.");
    }
}

pub async fn run(module: &Module, entrypoint_name: Option<String>) {
    let output_dir = module.classes_dir();

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
        .chain(iter::once(output_dir.display().to_string()))
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

    let output = module.docs_dir();

    tokio::fs::create_dir_all(&output).await.unwrap();

    cmd.arg("-d").arg(&output.display().to_string()).arg("-cp");

    // Collect dependencies include paths
    let cp = module
        .dependencies
        .iter_compile()
        .map(|it| format!("{}/{}", module.dir.display(), it.classpath()))
        .reduce(|a, b| format!("{};{}", a, b))
        .unwrap();
    cmd.arg(&cp);
    println!("compile classpath: {}", &cp);

    collect_files(&module.source_dir(), Some(&[".java"])).for_each(|it| {
        cmd.arg(it);
    });

    cmd.stdout(Stdio::inherit())
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
    let artifact_dir = module.artifacts_dir();
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

    tokio::fs::create_dir_all(&artifact_dir).await.unwrap();

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
        } else {
            cmd.arg("-M");
        }

        cmd.arg("-C")
            .arg(&base_dir2.join("target/classes"))
            .arg(".");

        cmd.stdout(Stdio::inherit())
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
                .arg("-M")
                .arg("-f")
                .arg(&format!("{}-sources.jar", artifact_base_name2));

            cmd.arg("-C").arg(&base_dir2.join("src")).arg(".");

            cmd.stdout(Stdio::inherit())
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
                .arg("-M")
                .arg("-f")
                .arg(&format!("{}-docs.jar", artifact_base_name2));

            let docs_dir = base_dir2.join("target/docs");
            cmd.arg("-C").arg(&docs_dir).arg(".");

            cmd.stdout(Stdio::inherit())
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

fn collect_files<P: AsRef<Path>>(
    path: P,
    extensions: Option<&'static [&'static str]>,
) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(move |it| {
            if it.file_type().is_file() {
                if let Some(extensions) = extensions {
                    let file_name = it.file_name().to_str().unwrap();
                    for e in extensions {
                        if file_name.ends_with(e) {
                            return true;
                        }
                    }
                    return false;
                }
                return true;
            }
            return false;
        })
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

                    let file_path = dir.join(&repodep.get_file_name());

                    if file_path.exists() {
                        println!("Dependency '{}' OK", repodep);
                        return;
                    }

                    println!("Downloading '{}' from {}", repodep, repodep.repo.name);

                    let url = repodep.jar_url();
                    //dbg!(&url);
                    download_file(client.as_ref(), url, &file_path)
                        .await
                        .unwrap();

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
