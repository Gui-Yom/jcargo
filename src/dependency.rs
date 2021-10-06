use std::fmt::{Display, Formatter};
use std::sync::Arc;

use semver::Version;

use crate::manifest::CompleteDependencyDef;
use crate::Env;

#[derive(Debug, Clone)]
pub enum Dependency {
    /// Dependency on a library from a maven repo
    Repo(RepoDependency),
    /// Dependency on another jcargo project in a git repository
    Git(GitDependency),
    /// Dependency on another local jcargo project
    Project(ProjectDependency),
    /// Dependency on an external project
    /// Point directly to the compiled classes / jar
    External(ExternalDependency),
}

impl Dependency {
    pub fn from_def(dd: CompleteDependencyDef, env: &Env) -> Self {
        Self::Repo(RepoDependency {
            group: dd.group,
            artifact: dd.artifact,
            version: Version::parse(&dd.version).unwrap(),
            repo: Arc::clone(&env.repos[0]),
        })
    }

    pub fn include_arg(&self) -> String {
        match self {
            Dependency::Repo(repodep) => repodep.get_file(),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MavenRepo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct RepoDependency {
    pub group: String,
    pub artifact: String,
    pub version: Version,
    pub repo: Arc<MavenRepo>,
}

impl RepoDependency {
    pub fn get_path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group.replace(".", "/"),
            self.artifact,
            self.version
        )
    }

    pub fn get_file(&self) -> String {
        format!("{}-{}.jar", self.artifact, self.version)
    }

    pub fn download_url(&self) -> String {
        format!("{}/{}/{}", self.repo.url, self.get_path(), self.get_file())
    }
}

impl Display for RepoDependency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.group, self.artifact, self.version)
    }
}

#[derive(Debug, Clone)]
struct GitDependency {
    /// Repository url
    url: String,
    /// Repo branch or tag
    /// Defaults to master or main
    branch: String,
    /// Commit to fetch
    /// Defaults to latest
    commit: String,
    /// Subdirectory to include as a dependency
    dir: String,
}

#[derive(Debug, Clone)]
struct ProjectDependency {
    path: String,
}

#[derive(Debug, Clone)]
struct ExternalDependency {
    path: String,
}
