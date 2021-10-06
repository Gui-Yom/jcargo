use std::str::FromStr;

use tokio::process;

#[derive(Debug, Copy, Clone)]
pub enum CompilationBackend {
    Javac,
    NativeJdkTools,
}

impl FromStr for CompilationBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "javac" => Ok(CompilationBackend::Javac),
            "native" => Ok(CompilationBackend::NativeJdkTools),
            other => Err(format!("Can't convert {} to a valid Backend", other)),
        }
    }
}

impl CompilationBackend {
    pub fn command(&self) -> process::Command {
        match self {
            CompilationBackend::Javac => process::Command::new("javac"),
            CompilationBackend::NativeJdkTools => {
                let mut cmd = process::Command::new("native-jdktools");
                cmd.arg("javac");
                cmd
            }
        }
    }
}

#[derive(Debug)]
pub enum Runtime {
    Java,
}

impl Runtime {
    pub fn command(&self) -> process::Command {
        match self {
            Runtime::Java => process::Command::new("java"),
        }
    }
}

#[derive(Debug)]
pub enum PackageBackend {
    Jar,
    NativeJdkTools,
}

impl PackageBackend {
    pub fn command(&self) -> process::Command {
        match self {
            PackageBackend::Jar => {
                let mut cmd = process::Command::new("jar");
                cmd
            }
            PackageBackend::NativeJdkTools => {
                let mut cmd = process::Command::new("native-jdktools");
                cmd.arg("jar");
                cmd
            }
        }
    }
}
