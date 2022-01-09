use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use anyhow::Result;
use lazy_regex::{regex, Lazy};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

/// Xml element with only a string body
#[derive(Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Element {
    #[serde(rename = "$value")]
    pub value: String,
}

impl Element {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl<S> From<S> for Element
where
    S: Into<String>,
{
    fn from(s: S) -> Self {
        Element::new(s)
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Debug for Element {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

pub type Properties = HashMap<String, String>;

pub trait PropertiesExt {
    /// Recursively resolve properties in the given text
    fn recurse_resolve<'t>(&self, text: &'t str) -> Cow<'t, str>;

    fn merge(&self, other: &Properties) -> Properties;
}

impl PropertiesExt for Properties {
    fn recurse_resolve<'t>(&self, text: &'t str) -> Cow<'t, str> {
        // Regex is compiled at compile time
        let pat: &Lazy<Regex> = regex!("\\$\\{(?P<prop_name>.+)\\}");
        pat.replace_all(text, |caps: &Captures| {
            self.recurse_resolve(self.get(caps.name("prop_name").unwrap().as_str()).unwrap())
        })
    }

    fn merge(&self, other: &Properties) -> Properties {
        let mut new = self.clone();
        for (k, v) in other.iter() {
            new.insert(k.clone(), v.clone());
        }
        new
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "project")]
pub struct MavenPom {
    #[serde(rename = "modelVersion")]
    pub model_version: Element,
    /// If none, then derived from parent
    #[serde(rename = "groupId")]
    pub group_id: Option<Element>,
    #[serde(rename = "artifactId")]
    pub artifact_id: Element,
    /// If none, then derived from parent
    pub version: Option<Element>,
    /// None if this is a top level pom
    pub parent: Option<ParentPom>,
    pub properties: Option<HashMap<String, String>>,
    pub dependencies: Option<PomDependencies>,
    #[serde(rename = "dependencyManagement")]
    pub dependency_management: Option<DependencyManagement>,
}

impl MavenPom {
    pub fn parse(text: &str) -> Result<Self> {
        let mut pom: Self = quick_xml::de::from_str(text)?;
        if pom.group_id.is_none() {
            pom.group_id = Some(pom.parent.as_ref().unwrap().group_id.clone());
        }
        if pom.version.is_none() {
            pom.version = Some(pom.parent.as_ref().unwrap().version.clone());
        }
        Ok(pom)
    }

    /// Get a new pom by applying a child pom over a parent pom
    pub fn merge(&self, new: &MavenPom) -> MavenPom {
        let props = if let Some(p) = self.properties.as_ref() {
            if let Some(c) = new.properties.as_ref() {
                Some(p.merge(c))
            } else {
                Some(p.clone())
            }
        } else {
            if let Some(c) = new.properties.as_ref() {
                Some(c.clone())
            } else {
                None
            }
        };

        let deps = if let Some(p) = self.dependencies.as_ref() {
            if let Some(c) = new.dependencies.as_ref() {
                Some(p.merge(c))
            } else {
                Some(p.clone())
            }
        } else {
            if let Some(c) = new.dependencies.as_ref() {
                Some(c.clone())
            } else {
                None
            }
        };

        let dep_mgmt = if let Some(p) = self.dependency_management.as_ref() {
            if let Some(c) = new.dependency_management.as_ref() {
                Some(p.merge(c))
            } else {
                Some(p.clone())
            }
        } else {
            if let Some(c) = new.dependency_management.as_ref() {
                Some(c.clone())
            } else {
                None
            }
        };

        MavenPom {
            model_version: "4.0.0".into(),
            group_id: new.group_id.clone().or(self.group_id.clone()),
            artifact_id: new.artifact_id.clone(),
            version: new.version.clone().or(self.version.clone()),
            // The resulting merged pom has no parent
            parent: None,
            properties: props,
            dependencies: deps,
            dependency_management: dep_mgmt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParentPom {
    #[serde(rename = "groupId")]
    pub group_id: Element,
    #[serde(rename = "artifactId")]
    pub artifact_id: Element,
    pub version: Element,
    //#[serde(rename = "relativePath")]
    //pub relative_path: Option<Element>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PomDependencies {
    #[serde(rename = "dependency")]
    pub dependencies: Vec<PomDependency>,
}

impl PomDependencies {
    /// Merge pdeps with cdeps
    pub fn merge(&self, new: &PomDependencies) -> PomDependencies {
        let mut dependencies = self.clone();
        for newDep in &new.dependencies {
            let candidate = dependencies
                .dependencies
                .iter_mut()
                .find(|it| it.group_id == newDep.group_id && it.artifact_id == newDep.artifact_id);
            // Dependency was already included, we just update it
            if let Some(curr) = candidate {
                *curr = curr.merge(newDep);
            } else {
                // New dependency
                dependencies.dependencies.push(newDep.clone());
            }
        }
        dependencies
    }

    pub fn apply_properties(&mut self, properties: &Properties) {
        for dep in self.dependencies.iter_mut() {
            if let Some(x) = dep.version.as_mut() {
                x.value = properties.recurse_resolve(&x.value).into_owned();
            }
        }
    }

    pub fn apply_rules(&self, rules: &DependencyManagement) -> PomDependencies {
        let mut newDeps = PomDependencies {
            dependencies: Vec::with_capacity(self.dependencies.len()),
        };
        for dep in self.dependencies.iter() {
            newDeps.dependencies.push(dep.apply_rules(rules));
        }
        newDeps
    }

    pub fn extract_deps(&self) {
        for dep in self.dependencies.iter() {
            //println!("Got dep {:#?}", dep);
            if dep
                .scope
                .as_ref()
                .map(|x| x.value)
                .unwrap_or(MavenDependencyScope::Compile)
                == MavenDependencyScope::Compile
            {
                println!("kept dep {:#?}", dep);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PomDependency {
    #[serde(rename = "groupId")]
    pub group_id: Element,
    #[serde(rename = "artifactId")]
    pub artifact_id: Element,
    pub version: Option<Element>,
    pub scope: Option<DependencyScope>,
    pub r#type: Option<Element>,
}

impl PomDependency {
    /// Merge 2 dependencies (they should be the same group:artifact)
    /// Apply new onto self
    pub fn merge(&self, new: &PomDependency) -> PomDependency {
        PomDependency {
            group_id: new.group_id.clone(),
            artifact_id: new.artifact_id.clone(),
            version: new.version.clone().or(self.version.clone()),
            scope: new.scope.clone().or(self.scope.clone()),
            r#type: new.r#type.clone().or(self.r#type.clone()),
        }
    }

    /// Complete a dependency field by applying the given dependency rules
    fn apply_rules(&self, rules: &DependencyManagement) -> PomDependency {
        if let Some(rule) = rules
            .dependencies
            .dependencies
            .iter()
            .find(|v| v.group_id == self.group_id && v.artifact_id == self.artifact_id)
        {
            PomDependency {
                group_id: self.group_id.clone(),
                artifact_id: self.artifact_id.clone(),
                version: self
                    .version
                    .as_ref()
                    .or_else(|| rule.version.as_ref())
                    .cloned(),
                scope: self.scope.as_ref().or_else(|| rule.scope.as_ref()).cloned(),
                r#type: self
                    .r#type
                    .as_ref()
                    .or_else(|| rule.r#type.as_ref())
                    .cloned(),
            }
        } else {
            self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DependencyScope {
    #[serde(rename = "$value")]
    pub value: MavenDependencyScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum MavenDependencyScope {
    #[serde(rename = "compile")]
    Compile,
    #[serde(rename = "runtime")]
    Runtime,
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "provided")]
    Provided,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DependencyManagement {
    pub dependencies: PomDependencies,
}

impl DependencyManagement {
    fn merge(&self, new: &DependencyManagement) -> DependencyManagement {
        DependencyManagement {
            dependencies: self.dependencies.merge(&new.dependencies),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use reqwest::Client;

    use crate::dependencies::mavenpom::{
        MavenPom, ParentPom, PomDependencies, PomDependency, Properties, PropertiesExt,
    };

    #[test]
    fn test_ser() {
        println!(
            "{}",
            quick_xml::se::to_string(&MavenPom {
                model_version: "4.0.0".into(),
                group_id: None,
                artifact_id: "jcargo-bin".into(),
                version: None,
                parent: Some(ParentPom {
                    group_id: "marais".into(),
                    artifact_id: "jcargo".into(),
                    version: "0.1.0".into(),
                    //relative_path: None,
                }),
                properties: Some([("a".to_string(), "b".to_string())].into_iter().collect()),
                dependencies: Some(PomDependencies {
                    dependencies: vec![
                        PomDependency {
                            group_id: "marais".into(),
                            artifact_id: "pomreader".into(),
                            version: None,
                            scope: None,
                            r#type: None,
                        },
                        PomDependency {
                            group_id: "marais".into(),
                            artifact_id: "pomreader".into(),
                            version: None,
                            scope: None,
                            r#type: None,
                        },
                    ]
                }),
                dependency_management: None,
            })
            .unwrap()
        );
    }

    async fn pom_source_0() -> Result<String> {
        Ok(reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-api/2.17.1/log4j-api-2.17.1.pom")
            .await?
            .text()
            .await?)
    }

    #[tokio::test]
    async fn test_deser_0() -> Result<()> {
        let pom = MavenPom::parse(&pom_source_0().await?)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    async fn pom_source_1() -> Result<String> {
        Ok(reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j/2.17.1/log4j-2.17.1.pom")
            .await?
            .text()
            .await?)
    }

    #[tokio::test]
    async fn test_deser_1() -> Result<()> {
        let pom = MavenPom::parse(&pom_source_1().await?)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    async fn pom_source_2() -> Result<String> {
        Ok(reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/logging-parent/3/logging-parent-3.pom")
            .await?
            .text()
            .await?)
    }

    #[tokio::test]
    async fn test_deser_2() -> Result<()> {
        let pom = MavenPom::parse(&pom_source_2().await?)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    async fn pom_source_3() -> Result<String> {
        Ok(
            reqwest::get("https://repo.maven.apache.org/maven2/org/apache/apache/23/apache-23.pom")
                .await?
                .text()
                .await?,
        )
    }

    #[tokio::test]
    async fn test_deser_3() -> Result<()> {
        let pom = MavenPom::parse(&pom_source_3().await?)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    #[test]
    fn test_props_resolve() -> Result<()> {
        let mut props = Properties::new();
        props.insert(
            "propname".to_string(),
            "you thought it was me, ${other}".to_string(),
        );
        props.insert("other".to_string(), "but it was me dio".to_string());
        assert_eq!(
            props.recurse_resolve("yay ${propname}").to_string(),
            "yay you thought it was me, but it was me dio".to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_merge() -> Result<()> {
        let pomA = MavenPom::parse(&pom_source_0().await?)?;

        let pomB = MavenPom::parse(&pom_source_1().await?)?;

        let pomC = MavenPom::parse(&pom_source_2().await?)?;

        let pomD = MavenPom::parse(&pom_source_3().await?)?;

        let merged = pomD.merge(&pomC).merge(&pomB).merge(&pomA);

        println!("merged : {:#?}", merged);

        Ok(())
    }

    #[tokio::test]
    async fn test_resolve() -> Result<()> {
        let client = Client::new();

        let pom = client
            .get("https://repo.maven.apache.org/maven2/com/graphql-java/graphql-java/17.3/graphql-java-17.3.pom")
            .send()
            .await?
            .text()
            .await?;
        let pom = MavenPom::parse(&pom)?;
        println!("parsed pom {:#?}", pom);

        pom.dependencies.map(|deps| deps.extract_deps());

        Ok(())
    }
}
