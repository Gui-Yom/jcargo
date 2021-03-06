use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;
use lazy_regex::{regex, Lazy};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::dependencies::xml_utils::Elem;

const SCHEMA_XSD: &str =
    "http://maven.apache.org/POM/4.0.0 http://maven.apache.org/maven-v4_0_0.xsd";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "project")]
pub struct MavenPom {
    #[serde(rename = "xsi:schemaLocation")]
    schema_location: String,
    #[serde(rename = "modelVersion")]
    pub model_version: Elem<String>,
    /// If none, then derived from parent
    #[serde(rename = "groupId")]
    pub group_id: Option<Elem<String>>,
    #[serde(rename = "artifactId")]
    pub artifact_id: Elem<String>,
    /// If none, then derived from parent
    pub version: Option<Elem<String>>,
    /// None if this is a top level pom
    pub parent: Option<ParentPom>,
    pub properties: Option<HashMap<String, String>>,
    pub dependencies: Option<PomDependencies>,
    #[serde(rename = "dependencyManagement")]
    pub dependency_management: Option<DependencyManagement>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParentPom {
    #[serde(rename = "groupId")]
    pub group_id: Elem<String>,
    #[serde(rename = "artifactId")]
    pub artifact_id: Elem<String>,
    pub version: Elem<String>,
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
    pub group_id: Elem<String>,
    #[serde(rename = "artifactId")]
    pub artifact_id: Elem<String>,
    pub version: Option<Elem<String>>,
    pub scope: Option<Elem<MavenDependencyScope>>,
    pub r#type: Option<Elem<String>>,
    pub optional: Option<Elem<bool>>,
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

    pub fn save(&self) -> Result<String> {
        Ok(quick_xml::se::to_string(self)?)
    }

    pub fn dependency_notation(&self) -> String {
        return format!(
            "{}:{}:{}",
            self.group_id.as_ref().unwrap().value,
            self.artifact_id.value,
            self.version.as_ref().unwrap().value
        );
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
            schema_location: SCHEMA_XSD.to_string(),
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

    /// Remove unneeded dependencies (e.g. test scope)
    pub fn clean(&mut self) {
        if let Some(deps) = self.dependencies.as_mut() {
            if let Some(mgmt) = self.dependency_management.as_ref() {
                deps.apply_rules(mgmt);
            }
            deps.clean();
            if let Some(props) = self.properties.as_ref() {
                for dep in deps.dependencies.iter_mut() {
                    if let Some(x) = dep.version.as_mut() {
                        x.value = props
                            .recurse_resolve(
                                &x.value,
                                self.version.as_ref().map(|e| &e.value).unwrap(),
                            )
                            .into_owned();
                    }
                }
            }
        }
        if self
            .dependencies
            .as_ref()
            .map_or(true, |d| d.dependencies.is_empty())
        {
            self.dependencies = None;
        }
        self.properties = None;
        self.dependency_management = None;
    }
}

impl ParentPom {
    pub fn dependency_notation(&self) -> String {
        return format!(
            "{}:{}:{}",
            self.group_id.value, self.artifact_id.value, self.version.value
        );
    }
}

impl PomDependencies {
    /// Merge pdeps with cdeps
    pub fn merge(&self, new: &PomDependencies) -> PomDependencies {
        let mut dependencies = self.clone();
        for new_dep in &new.dependencies {
            let candidate = dependencies.dependencies.iter_mut().find(|it| {
                it.group_id == new_dep.group_id && it.artifact_id == new_dep.artifact_id
            });
            // Dependency was already included, we just update it
            if let Some(curr) = candidate {
                *curr = curr.merge(new_dep);
            } else {
                // New dependency
                dependencies.dependencies.push(new_dep.clone());
            }
        }
        dependencies
    }

    pub fn apply_rules(&mut self, rules: &DependencyManagement) {
        for dep in self.dependencies.iter_mut() {
            *dep = dep.apply_rules(rules);
        }
    }

    pub fn clean(&mut self) {
        self.dependencies.retain(|dep| dep.should_keep());
    }
}

impl PomDependency {
    pub fn dependency_notation(&self) -> String {
        return format!(
            "{}:{}:{}",
            self.group_id.value,
            self.artifact_id.value,
            self.version.as_ref().unwrap().value
        );
    }

    /// Merge 2 dependencies (they should be the same group:artifact)
    /// Apply new onto self
    pub fn merge(&self, new: &PomDependency) -> PomDependency {
        PomDependency {
            group_id: new.group_id.clone(),
            artifact_id: new.artifact_id.clone(),
            version: new.version.as_ref().or(self.version.as_ref()).cloned(),
            scope: new.scope.as_ref().or(self.scope.as_ref()).cloned(),
            r#type: new.r#type.as_ref().or(self.r#type.as_ref()).cloned(),
            optional: new.optional.as_ref().or(self.optional.as_ref()).cloned(),
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
                version: self.version.as_ref().or(rule.version.as_ref()).cloned(),
                scope: self.scope.as_ref().or(rule.scope.as_ref()).cloned(),
                r#type: self.r#type.as_ref().or(rule.r#type.as_ref()).cloned(),
                optional: self.optional.as_ref().or(rule.optional.as_ref()).cloned(),
            }
        } else {
            self.clone()
        }
    }

    /// returns false if this dependency is useless, e.g. test dependency
    fn should_keep(&self) -> bool {
        !self.optional.clone().unwrap_or(false.into()).value && {
            let scope = self
                .scope
                .as_ref()
                .map(|x| x.value)
                .unwrap_or(MavenDependencyScope::Compile);
            //println!("dep: {}, scope: {:?}", self.dependency_notation(), scope);
            scope == MavenDependencyScope::Compile || scope == MavenDependencyScope::Runtime
        }
    }
}

impl DependencyManagement {
    fn merge(&self, new: &DependencyManagement) -> DependencyManagement {
        DependencyManagement {
            dependencies: self.dependencies.merge(&new.dependencies),
        }
    }
}

pub type Properties = HashMap<String, String>;

pub trait PropertiesExt {
    /// Recursively resolve properties in the given text
    fn recurse_resolve<'t>(&self, text: &'t str, project_version: &str) -> Cow<'t, str>;

    fn merge(&self, other: &Properties) -> Properties;
}

impl PropertiesExt for Properties {
    fn recurse_resolve<'t>(&self, text: &'t str, project_version: &str) -> Cow<'t, str> {
        // Regex is compiled at compile time
        let pat: &Lazy<Regex> = regex!("\\$\\{(?P<prop_name>.+)\\}");
        pat.replace_all(text, |caps: &Captures| {
            let prop = caps.name("prop_name").unwrap().as_str();

            // project.version is defined by maven
            if prop == "project.version" {
                return Cow::Borrowed(project_version);
            }

            let res = self.get(prop);
            if res.is_none() {
                println!("Can't resolve {}", text);
                return Cow::Borrowed(text);
            } else {
                return self.recurse_resolve(res.unwrap(), project_version);
            }
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use reqwest::Client;

    use crate::dependencies::mavenpom::{
        MavenPom, ParentPom, PomDependencies, PomDependency, Properties, PropertiesExt, SCHEMA_XSD,
    };

    #[test]
    fn test_ser() {
        println!(
            "{}",
            quick_xml::se::to_string(&MavenPom {
                schema_location: SCHEMA_XSD.to_string(),
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
                            optional: None,
                        },
                        PomDependency {
                            group_id: "marais".into(),
                            artifact_id: "pomreader".into(),
                            version: None,
                            scope: None,
                            r#type: None,
                            optional: None,
                        },
                    ]
                }),
                dependency_management: None,
            })
            .unwrap()
        );
    }

    async fn pom_source_0() -> Result<String> {
        Ok(reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-core/2.17.1/log4j-core-2.17.1.pom")
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
            props.recurse_resolve("yay ${propname}", "yay").to_string(),
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
    async fn test_clean() -> Result<()> {
        let mut pom = MavenPom::parse(&pom_source_0().await?)?;
        pom.clean();
        println!("{:#?}", pom);
        Ok(())
    }

    #[tokio::test]
    async fn test_ser_deser() -> Result<()> {
        let mut pom = MavenPom::parse(&pom_source_0().await?)?;
        pom.clean();
        let pom = MavenPom::parse(&pom.save()?)?;
        println!("pom: {:#?}", pom);
        Ok(())
    }

    #[test]
    fn test_deser() -> Result<()> {
        let text = r#"<project xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/maven-v4_0_0.xsd"><modelVersion>4.0.0</modelVersion><groupId>org.apache.logging.log4j</groupId><artifactId>log4j-api</artifactId><version>2.17.1</version></project>"#;
        let pom = MavenPom::parse(&text)?;
        Ok(())
    }
}
