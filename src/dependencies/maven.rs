use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::dependencies::dependency_graph::DependencyGraph;
use crate::dependencies::mavenpom::MavenPom;
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
    dep: MavenRepoDependency,
    sub_tasks: UnboundedSender<JoinHandle<Result<()>>>,
) -> Result<()> {
    let file_path = base_dir.join(&dep.pom_name());

    if !file_path.exists() {
        println!("Exploring root '{}' (pom) from {}", dep, dep.repo.name);

        let pom = fetch_pom(graph, client.clone(), dep).await?;
        println!("Downloaded pom : {:#?}", pom.dependency_notation());

        /*
        if let Some(deps) = pom.dependencies {
            for dep in deps.dependencies {
                println!("Should download dependency : {:#?}", dep);
                MavenRepoDependency {
                    group: dep.group_id.value,
                    artifact: dep.artifact_id.value,
                    version: Version::parse(&dep.version.unwrap().value).unwrap(),
                    repo: Arc::clone(&repodep.repo),
                }))
                .unwrap();
            }
        }*/
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

#[async_recursion::async_recursion]
async fn fetch_pom(
    graph: DependencyGraph,
    client: Client,
    dep: MavenRepoDependency,
) -> Result<MavenPom> {
    let key = dep.dependency_notation();
    let graph_ = graph.clone();
    graph
        .get_or_init(&key, async move {
            println!("In graph node, downloading {}", dep.dependency_notation());
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
                )
                .await?;
                // Merge current pom with parent
                pom = parent.merge(&pom);
            }
            return Ok(pom);
        })
        .await
}
