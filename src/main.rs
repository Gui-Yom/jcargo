use std::env;
use std::process;
use std::process::Stdio;
use std::time::Instant;

use walkdir::WalkDir;

fn main() {
    let instant = Instant::now();

    let mut javac = process::Command::new("javac-native");
    javac.args([
        "-source",
        "17",
        "-target",
        "17",
        "-encoding",
        "UTF-8",
        "-d",
        "buildjava/",
    ]);

    for item in WalkDir::new("srcjava").into_iter().filter_map(|e| e.ok()) {
        if item.file_type().is_file() {
            javac.arg(item.path().to_str().unwrap());
        }
    }

    javac
        .env(
            "JAVAC_HOME",
            "C:/Program Files/Eclipse Foundation/jdk-17.0.0.35-hotspot",
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    println!(
        "build finished. (took {} ms)",
        instant.elapsed().as_millis()
    );
    let instant = Instant::now();

    process::Command::new("java")
        .args([
            "-Xshare:on",
            "-XX:TieredStopAtLevel=1",
            "-XX:+UseSerialGC",
            "-cp",
            "buildjava",
            "Main",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    println!(
        "Program execution finished. (took {} ms)",
        instant.elapsed().as_millis()
    )
}
