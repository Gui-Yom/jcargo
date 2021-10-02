use std::process;
use std::str::FromStr;

#[derive(Debug)]
pub enum Backend {
    Javac,
    JavacNative,
}

impl FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "javac" => Ok(Backend::Javac),
            "javacnative" => Ok(Backend::JavacNative),
            other => Err(format!("Can't convert {} to a valid Backend", other)),
        }
    }
}

impl Backend {
    pub fn command(&self) -> process::Command {
        match self {
            Backend::Javac => process::Command::new("javac"),
            Backend::JavacNative => {
                process::Command::new(r"D:\Coding\Rust\jcargo\javac-native.exe")
            }
        }
    }
}
