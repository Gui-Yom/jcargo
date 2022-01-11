use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use async_oncecell::OnceCell;
use tokio::sync::Mutex;

use crate::dependencies::mavenpom::MavenPom;

#[derive(Clone)]
pub struct DependencyGraph {
    graph: Arc<Mutex<HashMap<String, Arc<OnceCell<MavenPom>>>>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<MavenPom> {
        let graph_ = self.graph.lock().await;
        graph_.get(key).and_then(|c| c.get().cloned())
    }

    pub async fn get_or_init<F>(&self, key: &str, init: F) -> Result<MavenPom>
    where
        F: Future<Output = Result<MavenPom>>,
    {
        let cell = {
            let mut graph_ = self.graph.lock().await;
            if !graph_.contains_key(key) {
                graph_.insert(key.to_string(), Arc::new(OnceCell::new()));
            }
            graph_.get(key).unwrap().clone()
        };
        // The map lock is released so we can still operate on the graph while waiting on a specific cell
        cell.get_or_try_init(init).await.map(|p| p.clone())
    }
}
