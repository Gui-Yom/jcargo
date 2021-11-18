use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// No properties xml element with only a string body
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Element {
    #[serde(rename = "$value")]
    pub value: String,
}

impl Element {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "project")]
pub struct MavenPom {
    #[serde(rename = "modelVersion")]
    pub model_version: Element,
    pub parent: Option<ParentPom>,
    pub properties: Option<HashMap<String, String>>,
    pub dependencies: Option<PomDependencies>,
    #[serde(rename = "dependencyManagement")]
    pub dependency_management: Option<DependencyManagement>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParentPom {
    #[serde(rename = "groupId")]
    pub group_id: Element,
    #[serde(rename = "artifactId")]
    pub artifact_id: Element,
    pub version: Element,
    #[serde(rename = "relativePath")]
    pub relative_path: Element,
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
        let pom: Self = quick_xml::de::from_str(text)?;
        Ok(pom)
    }
}

#[cfg(test)]
mod tests {
    use crate::mavenpom::{Element, MavenPom, ParentPom, PomDependencies, PomDependency};

    #[test]
    fn test_parse() {
        println!(
            "{:#?}",
            MavenPom::parse(
                r#"<project xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/maven-v4_0_0.xsd">
<modelVersion>4.0.0</modelVersion>
<parent>
<groupId>org.apache.logging.log4j</groupId>
<artifactId>log4j</artifactId>
<version>2.14.1</version>
<relativePath>../</relativePath>
</parent>
<artifactId>log4j-api</artifactId>
<packaging>jar</packaging>
<name>Apache Log4j API</name>
<description>The Apache Log4j API</description>
<properties>
<log4jParentDir>${basedir}/..</log4jParentDir>
<docLabel>API Documentation</docLabel>
<projectDir>/api</projectDir>
<maven.doap.skip>true</maven.doap.skip>
</properties>
<dependencies>
<dependency>
<groupId>org.apache.logging.log4j</groupId>
<artifactId>log4j-api-java9</artifactId>
<scope>provided</scope>
<type>zip</type>
</dependency>
<!--
 Place Felix before Equinox because Felix is signed. / also place it before org.osgi.core so that its versions of the OSGi classes are used 
-->
<dependency>
<groupId>org.apache.felix</groupId>
<artifactId>org.apache.felix.framework</artifactId>
<scope>test</scope>
</dependency>
<dependency>
<groupId>org.osgi</groupId>
<artifactId>org.osgi.core</artifactId>
<scope>provided</scope>
</dependency>
<dependency>
<groupId>org.junit.vintage</groupId>
<artifactId>junit-vintage-engine</artifactId>
</dependency>
</dependencies></project>"#,
            )
                .expect("Can't parse pom")
        );
    }

    #[test]
    fn test_ser() {
        println!(
            "{}",
            quick_xml::se::to_string(&MavenPom {
                model_version: Element::new("4.0.0".to_string()),
                parent: Some(ParentPom {
                    group_id: Element::new("marais".to_string()),
                    artifact_id: Element::new("jcargo".to_string()),
                    version: Element::new("0.1.0".to_string()),
                    relative_path: Element::new(String::new()),
                }),
                properties: Some([("a".to_string(), "b".to_string())].into_iter().collect()),
                dependencies: Some(PomDependencies {
                    dependencies: vec![
                        PomDependency {
                            group_id: Element::new("marais".to_string()),
                            artifact_id: Element::new("pomreader".to_string()),
                            version: None,
                            scope: None,
                            r#type: None,
                        },
                        PomDependency {
                            group_id: Element::new("marais".to_string()),
                            artifact_id: Element::new("pomreader".to_string()),
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
}
