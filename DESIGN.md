# Jcargo design choices

This documents contains a little explainer for some designs and inner workings of jcargo.

## Dependency configuration

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

## Dependency resolver

The dependency graph isn't known before we explore it. We explore it concurrently using multiple
asynchronous tasks. There are 2 type of nodes :

- Main nodes : the actual dependencies we need (either root or transitive). They are sent as tasks
  through a queue. We launch a tokio::task for each of them.
- Parent nodes : they represent maven parent poms we need in order to parse main nodes correctly.
  They are explored recursively.

Even though node exploration is done concurrently, no node is explored twice. A task will wait (
.await) for another task to finish processing a node before consuming its result. This is made
possible thanks to [async_cell](https://crates.io/crates/async_cell).
