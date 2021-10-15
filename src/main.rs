use std::path::PathBuf;
use std::sync::Arc;

use structopt::StructOpt;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::backend::{CompilationBackend, PackageBackend, Runtime};
use crate::dependencies::MavenRepo;
use crate::module::Module;

mod backend;
mod dependencies;
mod javac_parser;
mod manifest;
mod module;

#[derive(StructOpt, Debug)]
#[structopt(name = "jcargo", about = "Cargo but for java")]
struct Opts {
    #[structopt(short, long)]
    debug: bool,
    #[structopt(short, long = "--working-dir", default_value = ".")]
    working_dir: PathBuf,
    #[structopt(short, long, default_value = "native")]
    backend: CompilationBackend,
    #[structopt(subcommand)]
    task: Task,
}

#[derive(StructOpt, Debug)]
pub enum Task {
    /// Init a new project in the current directory
    Init { group: String, artifact: String },
    /// Check project consistency (manifest, dependencies)
    Check,
    /// Build project classes
    Build,
    /// Create a jar of the built classes
    Jar,
    /// Run a main class
    Run { entrypoint: Option<String> },
    /// Delete all generated directories
    Clean,
}

#[derive(Debug)]
pub struct Env {
    pub repos: Vec<Arc<MavenRepo>>,
    pub comp_backend: CompilationBackend,
    pub runtime: Runtime,
    pub package_backend: PackageBackend,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let opts = Opts::from_args();
    dbg!(&opts);

    let env = Env {
        repos: vec![Arc::new(MavenRepo {
            name: "maven-central".to_string(),
            url: "https://repo.maven.apache.org/maven2".to_string(),
        })],
        comp_backend: opts.backend,
        runtime: Runtime::Java,
        package_backend: PackageBackend::NativeJdkTools,
    };

    if let Task::Init { group, artifact } = &opts.task {
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
        buf.flush().await;
    } else {
        let module = Module::load(&opts.working_dir, &env).await;
        dbg!(&module);

        module.execute_task(opts.task, &env).await;
    }
}
