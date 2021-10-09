use std::path::PathBuf;
use std::sync::Arc;

use structopt::StructOpt;

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
    cmd: Task,
}

#[derive(StructOpt, Debug)]
pub enum Task {
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

    let module = Module::load(&opts.working_dir, &env).await;
    dbg!(&module);

    module.execute_task(opts.cmd, &env).await;
}
