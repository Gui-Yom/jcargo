use std::path::PathBuf;
use std::process;
use std::process::Stdio;
use std::time::Instant;
use std::{env, fs};

use structopt::StructOpt;
use walkdir::WalkDir;

use crate::backend::Backend;
use crate::module::Module;

mod backend;
mod javac_parser;
mod module;

#[derive(StructOpt, Debug)]
#[structopt(name = "jcargo", about = "Cargo but for java")]
struct Opts {
    #[structopt(short, long)]
    debug: bool,
    #[structopt(short, long = "--working-dir", default_value = ".")]
    working_dir: PathBuf,
    #[structopt(short, long, default_value = "javacnative")]
    backend: Backend,
    #[structopt(subcommand)]
    cmd: Task,
}

#[derive(StructOpt, Debug)]
enum Task {
    Build,
    Jar,
    Run { entrypoint: Option<String> },
}

#[derive(Debug)]
struct Env {
    pub backend: Backend,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let opts = Opts::from_args();
    dbg!(&opts);

    let module = Module::load(&opts.working_dir).await;
    dbg!(&module);

    module
        .execute_task(
            opts.cmd,
            Env {
                backend: opts.backend,
            },
        )
        .await;
}
