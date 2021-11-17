use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use tokio::process;

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
                let mut cmd = process::Command::new("native-jdktools");
                cmd.arg("javac").env(
                    "JDKTOOLS_HOME",
                    "C:/Program Files/Eclipse Foundation/jdk-17.0.0.35-hotspot",
                );
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
                let path = PathBuf::from(
                    env::var("KOTLINC_HOME")
                        .expect("KOTLINC_HOME expected to be set to where kotlinc is installed."),
                );
                process::Command::new(path.join("bin/kotlinc"))
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
                let mut cmd = process::Command::new("native-jdktools");
                cmd.arg("javadoc").env(
                    "JDKTOOLS_HOME",
                    "C:/Program Files/Eclipse Foundation/jdk-17.0.0.35-hotspot",
                );
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
                let mut cmd = process::Command::new("native-jdktools");
                cmd.arg("jar").env(
                    "JDKTOOLS_HOME",
                    "C:/Program Files/Eclipse Foundation/jdk-17.0.0.35-hotspot",
                );
                cmd
            }
        }
    }
}
