use std::path::PathBuf;
use std::sync::Arc;

use structopt::StructOpt;

use crate::backend::{CompilationBackend, DocumentationBackend, PackageBackend, Runtime};
use crate::dependencies::MavenRepo;
use crate::module::Module;
use crate::tasks::execute_task;

mod backend;
mod dependencies;
mod javac_parser;
mod manifest;
mod module;
mod tasks;

#[derive(StructOpt, Debug)]
#[structopt(name = "jcargo", about = "Cargo but for java")]
struct Opts {
    #[structopt(short, long)]
    debug: bool,
    #[structopt(short, long = "--working-dir", default_value = ".")]
    working_dir: PathBuf,
    #[structopt(long)]
    native: bool,
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
    /// Run a main class
    Run { entrypoint: Option<String> },
    /// Create javadoc
    Doc,
    /// Create a jar of the built classes
    Package {
        /// Create a sources jar
        #[structopt(long = "sources")]
        sources: bool,
        /// Create a doc jar
        #[structopt(long = "docs")]
        docs: bool,
        entrypoint: Option<String>,
    },
    /// Delete all generated directories
    Clean,
}

#[derive(Debug)]
pub struct Env {
    pub repos: Vec<Arc<MavenRepo>>,
    pub comp_backend: CompilationBackend,
    pub runtime: Runtime,
    pub doc_backend: DocumentationBackend,
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
        comp_backend: if opts.native {
            CompilationBackend::NativeJavac
        } else {
            CompilationBackend::JdkJavac
        },
        runtime: Runtime::Java,
        doc_backend: if opts.native {
            DocumentationBackend::NativeJavadoc
        } else {
            DocumentationBackend::JdkJavadoc
        },
        package_backend: if opts.native {
            PackageBackend::NativeJar
        } else {
            PackageBackend::JdkJar
        },
    };

    let module_resolver = async {
        let module = Module::load(&opts.working_dir, &env).await;
        dbg!(&module);
        module
    };

    execute_task(opts.task, &env, module_resolver).await;
}
