use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use tokio::fs;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::dependencies::dependency_graph::DependencyGraph;
use crate::dependencies::mavenpom::MavenPom;
use crate::dependencies::MavenRepoDependency;
use crate::io::{download_file, download_memory, save_to_file};

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
    println!("Exploring main node '{}'", root);

    let repo = Arc::clone(&root.repo);
    let pom = fetch_pom(graph.clone(), client.clone(), &base_dir, root.clone()).await?;
    //println!("Downloaded pom : {:#?}", pom);

    let jar_file = base_dir.join(root.jar_name());
    if !jar_file.exists() {
        println!(
            "Downloading artifacts for '{}' (jar) from {}",
            root.dependency_notation(),
            &repo.name
        );
        download_file(&client, root.jar_url(), &jar_file).await?;
    } else {
        println!("Dependency '{}' OK", root.dependency_notation());
    }

    if let Some(deps) = pom.dependencies {
        for dep in deps.dependencies {
            //println!("Should download dependency : {}", dep.dependency_notation());
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
            sub_tasks.send(task)?;
        }
    }
    Ok(())
}

/// The returned pom will have all its parents merged.
async fn fetch_pom(
    graph: DependencyGraph,
    client: Client,
    dir: &Path,
    dep: MavenRepoDependency,
) -> Result<MavenPom> {
    let key = dep.dependency_notation();
    let graph_ = graph.clone();
    graph
        .get_or_init(&key, async {
            let file = dir.join(dep.pom_name());

            Ok(if file.exists() {
                println!("Running in main node '{}': fetching pom (cache hit)", &key);
                MavenPom::parse(&fs::read_to_string(&file).await?).unwrap()
            } else {
                println!("Running in main node '{}': fetching pom", &key);
                let mut pom = MavenPom::parse(&download_memory(&client, dep.pom_url()).await?)?;
                if let Some(parent) = pom.parent.clone() {
                    // Recurse to download and merge parent pom hierarchy
                    let parent = fetch_parent_pom(
                        graph_,
                        client,
                        MavenRepoDependency {
                            group: parent.group_id.value,
                            artifact: parent.artifact_id.value,
                            version: parent.version.value,
                            repo: Arc::clone(&dep.repo),
                        },
                    )
                    .await?;
                    // Merge current pom with parent
                    pom = parent.merge(&pom);
                }
                pom.clean();
                save_to_file(&pom.save()?, &file).await?;
                pom
            })
        })
        .await
}

#[async_recursion::async_recursion]
async fn fetch_parent_pom(
    graph: DependencyGraph,
    client: Client,
    dep: MavenRepoDependency,
) -> Result<MavenPom> {
    let key = dep.dependency_notation();
    let graph_ = graph.clone();
    graph
        .get_or_init(&key, async {
            println!("Running in parent node '{}': fetching pom", &key);
            let mut pom = MavenPom::parse(&download_memory(&client, dep.pom_url()).await?)?;
            if let Some(parent) = pom.parent.clone() {
                let parent = fetch_parent_pom(
                    graph_,
                    client,
                    MavenRepoDependency {
                        group: parent.group_id.value,
                        artifact: parent.artifact_id.value,
                        version: parent.version.value,
                        repo: Arc::clone(&dep.repo),
                    },
                )
                .await?;
                // Merge current pom with parent
                pom = parent.merge(&pom);
            }
            Ok(pom)
        })
        .await
}
