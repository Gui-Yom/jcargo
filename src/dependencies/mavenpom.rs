use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

use anyhow::Result;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

/// Xml element with only a string body
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
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

pub type Properties = HashMap<String, String>;

pub trait PropertiesExt {
    fn get_resolve(&self, key: &str) -> Option<String>;
}

impl PropertiesExt for Properties {
    fn get_resolve(&self, key: &str) -> Option<String> {
        let pat = Regex::new("\\$\\{(?P<prop_name>.+)\\}").unwrap();
        self.get(key).map(|it| {
            pat.replace_all(it, |caps: &Captures| {
                self.get(caps.name("prop_name").unwrap().as_str()).unwrap()
            })
            .to_string()
        })
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PomDependency {
    #[serde(rename = "groupId")]
    pub group_id: Element,
    #[serde(rename = "artifactId")]
    pub artifact_id: Element,
    pub version: Option<Element>,
    pub scope: Option<Element>,
    pub r#type: Option<Element>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DependencyManagement {
    pub dependencies: PomDependencies,
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
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::dependencies::mavenpom::{
        Element, MavenPom, ParentPom, PomDependencies, PomDependency,
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

    #[tokio::test]
    async fn test_deser_real0() -> Result<()> {
        let pom = reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-api/2.14.1/log4j-api-2.14.1.pom")
            .await?
            .text()
            .await?;
        let pom = MavenPom::parse(&pom)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    #[tokio::test]
    async fn test_deser_real1() -> Result<()> {
        let pom = reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j/2.14.1/log4j-2.14.1.pom")
            .await?
            .text()
            .await?;
        let pom = MavenPom::parse(&pom)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    #[tokio::test]
    async fn test_deser_real2() -> Result<()> {
        let pom = reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/logging-parent/3/logging-parent-3.pom")
            .await?
            .text()
            .await?;
        let pom = MavenPom::parse(&pom)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }

    #[tokio::test]
    async fn test_deser_real3() -> Result<()> {
        let pom =
            reqwest::get("https://repo.maven.apache.org/maven2/org/apache/apache/23/apache-23.pom")
                .await?
                .text()
                .await?;
        let pom = MavenPom::parse(&pom)?;
        println!("parsed pom {:#?}", pom);
        Ok(())
    }
}
