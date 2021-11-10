# jcargo

Bringing Rust's excellent build tool to the JVM.

## Roadmap for 1.0

- [x] Project model, configuration and management
    * [ ] Stable configuration model (TOML)
    * [x] Project initialization (jcargo init)
    * [ ] Project cleanup (jcargo clean)
    * [x] Consistency check (jcargo check)
    * [ ] Dependency handling
        - [x] Standard binary repositories
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
- [ ] Kotlin support
    * [ ] JVM Compilation support
    * [ ] Kdoc generation
    * [ ] Annotation processing
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
