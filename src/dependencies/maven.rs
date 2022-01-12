use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::dependencies::dependency_graph::DependencyGraph;
use crate::dependencies::mavenpom::{DependencyScope, MavenDependencyScope, MavenPom};
use crate::dependencies::MavenRepoDependency;
use crate::download::download_memory;

/*
We have a dependency graph
- We explore the nodes concurrently
- When an explorer arrive on a node, it checks whether the node task has not been done
- If the task is open, it launches a task stored in the graph node itself
- It then awaits the task output
- If the task is not open, it awaits the task result
- The task result must be cached since it can be awaited multiple times
 */

#[async_recursion::async_recursion]
pub async fn explore_dependency(
    client: Client,
    graph: DependencyGraph,
    base_dir: PathBuf,
    root: MavenRepoDependency,
    sub_tasks: UnboundedSender<JoinHandle<Result<()>>>,
) -> Result<()> {
    let file_path = base_dir.join(&root.pom_name());

    if !file_path.exists() {
        println!("Exploring main node '{}'", root);

        let repo = Arc::clone(&root.repo);
        let mut pom = fetch_pom(graph.clone(), client.clone(), root, true).await?;
        //println!("Downloaded pom : {:#?}", pom);

        if let Some(deps) = pom.dependencies {
            for dep in deps.dependencies {
                /*
                if let Some(scope) = dep
                    .scope
                    .clone()
                    .map(|s| s.value)
                    .or(Some(MavenDependencyScope::Compile))
                {
                    if scope == MavenDependencyScope::Compile
                        || scope == MavenDependencyScope::Runtime
                    {*/
                println!("Should download dependency : {}", dep.dependency_notation());
                let repo = Arc::clone(&repo);
                let task = tokio::spawn(explore_dependency(
                    client.clone(),
                    graph.clone(),
                    base_dir.clone(),
                    MavenRepoDependency {
                        group: dep.group_id.value,
                        artifact: dep.artifact_id.value,
                        version: dep.version.unwrap().value,
                        repo,
                    },
                    sub_tasks.clone(),
                ));
                sub_tasks.send(task);
            }
        }
    }

    /*
    let file_path = dir.join(&repodep.get_jar_name());

    if !file_path.exists() {
        println!("Downloading '{}' (jar) from {}", repodep, repodep.repo.name);

        let url = repodep.jar_url();
        //dbg!(&url);
        download_file(&client, url, &file_path).await.unwrap();
    }

    println!("Dependency '{}' OK", repodep);*/
    Ok(())
}

/// The returned pom will have all its parents merged.
#[async_recursion::async_recursion]
async fn fetch_pom(
    graph: DependencyGraph,
    client: Client,
    dep: MavenRepoDependency,
    main: bool,
) -> Result<MavenPom> {
    let key = dep.dependency_notation();
    let graph_ = graph.clone();
    graph
        .get_or_init(&key, async move {
            println!(
                "Running in {} node '{}': downloading pom",
                if main { "main" } else { "parent" },
                dep.dependency_notation()
            );
            let mut pom = MavenPom::parse(&download_memory(&client, dep.pom_url()).await?)?;
            if let Some(parent) = pom.parent.clone() {
                // Recurse to fetch parent pom
                let parent = fetch_pom(
                    graph_,
                    client,
                    MavenRepoDependency {
                        group: parent.group_id.value,
                        artifact: parent.artifact_id.value,
                        version: parent.version.value,
                        repo: Arc::clone(&dep.repo),
                    },
                    false,
                )
                .await?;
                // Merge current pom with parent
                pom = parent.merge(&pom);
                if main {
                    pom.clean();
                }
            }
            return Ok(pom);
        })
        .await
}
