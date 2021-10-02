use std::path::PathBuf;
use std::process;
use std::process::Stdio;
use std::time::Instant;
use std::{env, fs};

use structopt::StructOpt;
use walkdir::WalkDir;

use crate::backend::Backend;
use crate::module::{Module, ModuleConfig};

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

fn main() {
    let opts = Opts::from_args();
    println!("{:?}", opts);

    let module = Module::load(&opts.working_dir);
    dbg!(&module);

    println!(
        "   Compiling {} v{} <path>",
        module.config.name, module.config.version
    );

    let backend = opts.backend;
    let build = || {
        let instant = Instant::now();
        module.compile(backend);

        println!(
            "   Finished build. (took {} ms)",
            instant.elapsed().as_millis()
        );
        println!();
    };

    let run = |entrypoint: Option<String>| {
        println!("   Running 'Main'");
        let instant = Instant::now();

        module.run(entrypoint);

        println!(
            "   Execution finished. (took {} ms)",
            instant.elapsed().as_millis()
        )
    };

    match opts.cmd {
        Task::Build => {
            build();
        }

        Task::Jar => {
            build();
        }

        Task::Run { entrypoint } => {
            build();
            run(entrypoint);
        }
    }
}
