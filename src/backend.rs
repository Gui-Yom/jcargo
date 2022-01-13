use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use tokio::process;

fn native_jdktools_path() -> String {
    env::var("NATIVE_JDK").expect("NATIVE_JDK needs to point to the native-jdktools executable")
}

fn kotlinc_path() -> String {
    format!(
        "{}/bin/kotlinc",
        env::var("KOTLINC_HOME")
            .expect("KOTLINC_HOME expected to be set to where kotlinc is installed.")
    )
}

#[derive(Debug, Copy, Clone)]
pub enum JavaCompilationBackend {
    JdkJavac,
    NativeJavac,
}

impl FromStr for JavaCompilationBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "javac" => Ok(JavaCompilationBackend::JdkJavac),
            "native" => Ok(JavaCompilationBackend::NativeJavac),
            other => Err(format!("Can't convert {} to a valid Backend", other)),
        }
    }
}

impl JavaCompilationBackend {
    pub fn command(&self) -> process::Command {
        match self {
            JavaCompilationBackend::JdkJavac => process::Command::new("javac"),
            JavaCompilationBackend::NativeJavac => {
                let mut cmd = process::Command::new(native_jdktools_path());
                cmd.arg("javac");
                cmd
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum KotlinCompilationBackend {
    Kotlinc,
}

impl KotlinCompilationBackend {
    pub fn command(&self) -> process::Command {
        match self {
            KotlinCompilationBackend::Kotlinc => {
                let mut cmd = process::Command::new(kotlinc_path());
                cmd
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub enum DocumentationBackend {
    JdkJavadoc,
    NativeJavadoc,
}

impl DocumentationBackend {
    pub fn command(&self) -> process::Command {
        match self {
            DocumentationBackend::JdkJavadoc => {
                let cmd = process::Command::new("javadoc");
                cmd
            }
            DocumentationBackend::NativeJavadoc => {
                let mut cmd = process::Command::new(native_jdktools_path());
                cmd.arg("javadoc");
                cmd
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PackageBackend {
    JdkJar,
    NativeJar,
}

impl PackageBackend {
    pub fn command(&self) -> process::Command {
        match self {
            PackageBackend::JdkJar => {
                let cmd = process::Command::new("jar");
                cmd
            }
            PackageBackend::NativeJar => {
                let mut cmd = process::Command::new(native_jdktools_path());
                cmd.arg("jar");
                cmd
            }
        }
    }
}
