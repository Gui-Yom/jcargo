use std::fmt::{Display, Formatter};
use std::sync::Arc;

use semver::Version;

use crate::manifest::{CompleteDependencyDef, DependenciesDef};
use crate::Env;

#[derive(Debug, Clone)]
pub struct Dependencies {
    pub compile: Vec<Dependency>,
    pub runtime: Vec<Dependency>,
    pub compile_runtime: Vec<Dependency>,
    pub transitive: Vec<Dependency>,
}

impl Dependencies {
    pub fn from_def(dd: DependenciesDef, env: &Env) -> Self {
        Self {
            compile: dd
                .compile
                .into_iter()
                .map(|it| Dependency::from_def(it.into(), env))
                .collect(),
            runtime: dd
                .runtime
                .into_iter()
                .map(|it| Dependency::from_def(it.into(), env))
                .collect(),
            compile_runtime: dd
                .compile_runtime
                .into_iter()
                .map(|it| Dependency::from_def(it.into(), env))
                .collect(),
            transitive: dd
                .transitive
                .into_iter()
                .map(|it| Dependency::from_def(it.into(), env))
                .collect(),
        }
    }

    /// Total number of dependencies, all scopes
    pub fn len(&self) -> usize {
        self.compile.len() + self.runtime.len() + self.compile_runtime.len() + self.transitive.len()
    }

    /// Returns an iterator over all dependencies
    pub fn iter(&self) -> impl Iterator<Item = &Dependency> {
        self.compile
            .iter()
            .chain(self.runtime.iter())
            .chain(self.compile_runtime.iter())
            .chain(self.transitive.iter())
    }

    /// Returns an Iterator over all dependencies that should be available at compile time
    pub fn iter_compile(&self) -> impl Iterator<Item = &Dependency> {
        self.compile
            .iter()
            .chain(self.compile_runtime.iter())
            .chain(self.transitive.iter())
    }

    /// Returns an Iterator over all dependencies that should be available at runtime
    pub fn iter_runtime(&self) -> impl Iterator<Item = &Dependency> {
        self.runtime
            .iter()
            .chain(self.compile_runtime.iter())
            .chain(self.transitive.iter())
    }
}

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
        let first = dd.version.comparators.first().unwrap();
        Self::Repo(RepoDependency {
            group: dd.group,
            artifact: dd.artifact,
            version: Version::new(first.major, first.minor.unwrap(), first.patch.unwrap()),
            repo: Arc::clone(&env.repos[0]),
        })
    }

    pub fn classpath(&self) -> String {
        match self {
            Dependency::Repo(repodep) => format!("libs/{}", repodep.get_file()),
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
pub struct GitDependency {
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
pub struct ProjectDependency {
    path: String,
}

#[derive(Debug, Clone)]
pub struct ExternalDependency {
    path: String,
}
