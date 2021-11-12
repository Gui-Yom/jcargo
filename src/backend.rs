use std::str::FromStr;

use tokio::process;

#[derive(Debug, Copy, Clone)]
pub enum CompilationBackend {
    JdkJavac,
    NativeJavac,
}

impl FromStr for CompilationBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "javac" => Ok(CompilationBackend::JdkJavac),
            "native" => Ok(CompilationBackend::NativeJavac),
            other => Err(format!("Can't convert {} to a valid Backend", other)),
        }
    }
}

impl CompilationBackend {
    pub fn command(&self) -> process::Command {
        match self {
            CompilationBackend::JdkJavac => process::Command::new("javac"),
            CompilationBackend::NativeJavac => {
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
