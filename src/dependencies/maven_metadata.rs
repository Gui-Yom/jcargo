use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::dependencies::xml_utils::Elem;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "metadata")]
pub struct MavenMetadata {
    #[serde(rename = "groupId")]
    pub group_id: Elem<String>,
    #[serde(rename = "artifactId")]
    pub artifact_id: Elem<String>,
    pub versioning: Versioning,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Versioning {
    pub latest: Elem<String>,
    pub release: Elem<String>,
    pub versions: Versions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Versions {
    #[serde(rename = "version")]
    pub versions: Vec<Elem<String>>,
}

impl MavenMetadata {
    pub fn parse(text: &str) -> Result<Self> {
        let meta: Self = quick_xml::de::from_str(text)?;
        Ok(meta)
    }
}

mod tests {
    use anyhow::Result;

    use crate::dependencies::maven_metadata::{MavenMetadata, Versioning, Versions};
    use crate::dependencies::xml_utils::Elem;

    #[test]
    fn test_ser() {
        println!(
            "{}",
            quick_xml::se::to_string(&MavenMetadata {
                group_id: "marais".into(),
                artifact_id: "graphql".into(),
                versioning: Versioning {
                    latest: "0.1.0".into(),
                    release: "0.1.0".into(),
                    versions: Versions {
                        versions: vec!["0.1.0".into()]
                    },
                },
            })
            .unwrap()
        );
    }

    async fn pom_source_0() -> Result<String> {
        Ok(reqwest::get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-core/maven-metadata.xml")
            .await?
            .text()
            .await?)
    }

    #[tokio::test]
    async fn test_deser_0() -> Result<()> {
        let meta = MavenMetadata::parse(&pom_source_0().await?)?;
        println!("parsed metadata {:#?}", meta);
        Ok(())
    }
}
