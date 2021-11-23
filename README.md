# jcargo

Trying to remake Rust's excellent build tool for the JVM ecosystem.

## Motivations

Maven is awful, who wants to write xml. Gradle is a step in the right direction, but you need a
degree to use it (and it requires a damn daemon to hide the fact that it is painfully slow).

Jcargo doesn't run on the JVM, it doesn't suffer long boot times (essential for a CLI). It comes
with another project of mine : https://github.com/Gui-Yom/native-jdktools, an attempt at compiling
the jdk tools (javac, javadoc, jar ...) with GraalVM to improve boot times.

Jcargo is configured from a single `jcargo.toml` file that is simple to write and to read. It
follows the principle of "Simple and efficient enough for 90% of use cases", for the remaining 10%
we may need a build script or something (to be explored later).

## Installation

### Download a prebuilt binary

See the release page.

### Building from source with cargo

Requires at least `Rust 1.56`.

```shell
cargo install jcargo
```

## Runtime

For now, `JDK_HOME/bin` must be in your path for jcargo to find the jdk tools. If you want to
compile kotlin sources, set `KOTLINC_HOME` to point to the installation directory of kotlinc.

## Roadmap for 1.0

- [x] Project model, configuration and management
    * [ ] Stable configuration model (TOML)
    * [x] Project initialization (jcargo init)
    * [x] Project cleanup (jcargo clean)
    * [x] Consistency check (jcargo check)
    * [ ] Dependency handling
        - [ ] Standard maven binary repositories
            * [x] Maven pom parsing
            * [ ] Recurse and merge poms
            * [ ] Download full dependency tree
            * [ ] Gradle metadata ?
        - [ ] Custom binary repositories
        - [ ] Git dependencies
        - [ ] Local dependency
        - [ ] Dependency caching
    * [ ] Multiple source sets
        - [ ] Tests
        - [ ] Examples
        - [ ] Per source set dependencies
    * [ ] Multiple modules
        - [ ] Inter modules dependencies
- [ ] Java support
    * [x] Compilation
    * [x] Javadoc generation
    * [ ] Annotation processing
    * [ ] Toolchain handling
        - [ ] Handle source and target versions
        - [ ] Handle jdk installations
- [ ] Kotlin support
    * [x] JVM Compilation support
    * [ ] Kdoc generation
    * [ ] Annotation processing
    * [ ] Toolchain handling
        - [ ] Handle source and target versions
        - [ ] Handle kotlinc and jdk installations
- [ ] Packaging
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

## Roadmap for 2.0

- Error messages processing
