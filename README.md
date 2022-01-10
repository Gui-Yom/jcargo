# jcargo

An attempt at making an equivalent to Rust's excellent build tool for the JVM ecosystem.

## Motivations

Maven is awful, who wants to write xml. Gradle is a step in the right direction, but you need a
degree to use it (and it requires a damn daemon to hide the fact that it is painfully slow).

Jcargo doesn't run on the JVM, it doesn't suffer long boot times (essential for a CLI). It comes
with another project of mine : https://github.com/Gui-Yom/native-jdktools, an attempt at compiling
the jdk tools (javac, javadoc, jar ...) with GraalVM to improve boot times.

Jcargo is configured from a single `jcargo.toml` file that is simple to write and read. It follows
the principle of "Simple and efficient enough for 90% of use cases", for the remaining 10% we may
need a build script or something (to be explored later).

For 90% of use cases, you just need to bootstrap a project that pulls up some dependencies without
any special processing of any sort.

## Current state

The project is very far from being usable in practice on anything real. It can successfully compile
projects without any dependencies tho.

## Installation

### Downloading a prebuilt binary

See the [Releases](https://github.com/Gui-Yom/jcargo/releases) page.

### Building from source

Requires at least `Rust 1.56`.

#### From crates.io (published version)

```shell
cargo install jcargo
```

#### From master branch

```shell
git clone https://github.com/Gui-Yom/jcargo
cd jcargo
cargo install --path .
```

## Runtime

For now, `JDK_HOME/bin` must be in your path for jcargo to find the jdk tools. If you want to
compile kotlin sources, set `KOTLINC_HOME` to point to the installation directory of kotlinc.

## Configuration

Configuration is definitely not frozen. I particularly don't like how dependencies are specified.
Example :

```toml
group = "marais"
artifact = "testproject"
version = "0.1.0"

[dependencies]
# Compile and runtime dependencies
compileRuntime = [
    "org.apache.logging.log4j:log4j-api:2.17.1"
]
# Runtime only dependencies
runtime = [
    { group = "org.apache.logging.log4j", artifact = "log4j-core", version = "2.17.1" }
]
# Compile only
compile = []
transitive = []

[[entrypoints]]
class = "Main"

[[entrypoints]]
name = "Other"
class = "OtherMain"
```

### Dependencies configuration

Explored configuration structures :

```toml
# Table for sources sets, but array for dependencies
[main]
dependencies = [
    "org.apache.logging.log4j:log4j-api:2.17.1", # Defaults to compile + runtime
    { notation = "org.apache.logging.log4j:log4j-api:2.17.1", compile = true, runtime = true }
]
```

```toml
# Table for source sets and table for dependencies
# Similar to what cargo does
[main.dependencies]
"org.apache.logging.log4j:log4j-api" = { version = "2.17.1", compile = true, runtime = true, transitive = false }

[test.dependencies]
"org.junit:junit" = "5"

# We can't do this, this isn't valid toml
[dependencies.example]
"group:a:1"
"group:b:1"
```

```toml
# Table for sources sets and table for scopes
[main.dependencies]
compileRuntime = [
    "org.apache.logging.log4j:log4j-api:2.17.1"
]
runtime = [
    { group = "org.apache.logging.log4j", artifact = "log4j-core", version = "2.17.1" }
]
compile = []
```

## Roadmap for 1.0

- [x] means a feature is partially implemented, not completely finished
- [ ] means a feature is completely absent


- [x] Project model, configuration and management
    * [ ] Stable configuration model (TOML)
    * [x] Project initialization (jcargo init)
        - [x] Create an initial configuration file
    * [x] Project cleanup (jcargo clean)
        - [x] Delete the `target` dir
    * [x] Consistency check (jcargo check)
        - [ ] Verify configuration file
        - [x] Download dependencies
        - [ ] Check dependencies versions
    * [ ] Dependency handling
        - [x] Standard maven binary repositories
            * [x] Maven pom parsing
            * [x] Recurse and merge poms
            * [x] Download full dependency tree
            * [ ] Gradle metadata ?
        - [ ] Custom binary repositories
        - [ ] Git dependencies (project made with jcargo)
        - [ ] Local dependency (project made with jcargo)
        - [ ] Dependency caching
            * [ ] Cache pom and jar
            * [ ] Cache maven metadata
            * [ ] Cache dependency graph resolution
            * [ ] Verify file hashes
    * [ ] Multiple source sets
        - [ ] Main
        - [ ] Tests
        - [ ] Examples
        - [ ] Benchmarks ?
        - [ ] Per source set dependencies
    * [ ] Multi-modules builds
        - [ ] Inter modules dependencies
- [x] Java support
    * [x] Compilation
    * [x] Javadoc generation
    * [ ] Annotation processing
    * [ ] Toolchain handling
        - [ ] Handle source and target versions
        - [ ] Handle multiple jdk installations
- [x] Kotlin support
    * [x] JVM Compilation support
    * [ ] Kdoc generation
    * [ ] Annotation processing
    * [ ] Toolchain handling
        - [ ] Handle source and target versions
        - [ ] Handle multiple kotlinc and jdk installations
- [x] Packaging
    * [x] Application jar
    * [x] Documentation jar
    * [x] Sources jar
    * [ ] Sources tarball
    * [x] Resources handling
    * [ ] Dependency vendoring options
    * [ ] Publishing to binary repositories
        - [ ] Maven's POM generation
        - [ ] Gradle Module metadata generation
        - [ ] Remote repository publication
- [ ] IDE Support
    * [ ] IntelliJ IDEA integration
        - [ ] Configuration file support
        - [ ] Full classpath support

## Other ideas

- Error messages processing, beautify javac error messages for command line use.
