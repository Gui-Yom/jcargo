use std::collections::HashMap;
use std::ops::Deref;

use regex::Captures;

use crate::dependencies::mavenpom::{
    DependencyManagement, Element, MavenPom, PomDependencies, PomDependency, Properties,
};

pub struct PomCache {
    /// groupid:artifactid:version as key
    cache: HashMap<String, MavenPom>,
}

impl PomCache {
    pub fn new() -> Self {
        PomCache {
            cache: HashMap::new(),
        }
    }

    /*
    pub fn add(&mut self, pom: MavenPom) {
        self.cache[&format!(
            "{}:{}:{}",
            pom.group_id.unwrap(),
            pom.artifact_id,
            pom.version.unwrap()
        )] = pom;
    }*/

    pub fn get(&self, groupid: &str, artifactid: &str, version: &str) -> Option<&MavenPom> {
        self.cache
            .get(&format!("{}:{}:{}", groupid, artifactid, version))
    }
}

pub fn merge_poms(parent: &MavenPom, child: &MavenPom) -> MavenPom {
    let props = if let Some(p) = parent.properties.as_ref() {
        if let Some(c) = child.properties.as_ref() {
            Some(merge_properties(p, c))
        } else {
            Some(p.clone())
        }
    } else {
        if let Some(c) = child.properties.as_ref() {
            Some(c.clone())
        } else {
            None
        }
    };

    let deps = if let Some(p) = parent.dependencies.as_ref() {
        if let Some(c) = child.dependencies.as_ref() {
            Some(merge_dependencies(p, c))
        } else {
            Some(p.clone())
        }
    } else {
        if let Some(c) = child.dependencies.as_ref() {
            Some(c.clone())
        } else {
            None
        }
    };

    let dep_mgmt = if let Some(p) = parent.dependency_management.as_ref() {
        if let Some(c) = child.dependency_management.as_ref() {
            Some(merge_dep_mgmt(p, c))
        } else {
            Some(p.clone())
        }
    } else {
        if let Some(c) = child.dependency_management.as_ref() {
            Some(c.clone())
        } else {
            None
        }
    };

    MavenPom {
        model_version: "4.0.0".into(),
        group_id: child.group_id.clone().or(parent.group_id.clone()),
        artifact_id: child.artifact_id.clone(),
        version: child.version.clone().or(parent.version.clone()),
        // The resulting merged pom has no parent
        parent: None,
        properties: props,
        dependencies: deps,
        dependency_management: dep_mgmt,
    }
}

/// Merge pdeps with cdeps
pub fn merge_dependencies(pdeps: &PomDependencies, cdeps: &PomDependencies) -> PomDependencies {
    let mut dependencies = pdeps.clone();
    for cdep in &cdeps.dependencies {
        let f = dependencies
            .dependencies
            .iter_mut()
            .find(|it| it.group_id == cdep.group_id && it.artifact_id == cdep.artifact_id);
        if let Some(f) = f {
            let new = merge_pom_dependency(f, cdep);
            *f = new;
        } else {
            dependencies.dependencies.push(cdep.clone());
        }
    }

    dependencies
}

/// Overwrite initial with new
pub fn merge_pom_dependency(initial: &PomDependency, new: &PomDependency) -> PomDependency {
    PomDependency {
        group_id: new.group_id.clone(),
        artifact_id: new.artifact_id.clone(),
        version: new.version.clone().or(initial.version.clone()),
        scope: new.scope.clone().or(initial.scope.clone()),
        r#type: new.r#type.clone().or(initial.r#type.clone()),
    }
}

fn merge_properties(pprops: &Properties, cprops: &Properties) -> Properties {
    let mut props = pprops.clone();
    for (k, v) in cprops.iter() {
        props.insert(k.clone(), v.clone());
    }
    props
}

fn merge_dep_mgmt(
    pdep_mgmt: &DependencyManagement,
    cdep_mgmt: &DependencyManagement,
) -> DependencyManagement {
    DependencyManagement {
        dependencies: merge_dependencies(&pdep_mgmt.dependencies, &cdep_mgmt.dependencies),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use reqwest::Client;

    use crate::dependencies::maven::merge_poms;
    use crate::dependencies::mavenpom::{
        Element, MavenPom, ParentPom, PomDependencies, PomDependency, Properties, PropertiesExt,
    };

    #[test]
    fn test_resolve_props() {
        let mut props = Properties::from([
            ("a".to_string(), "value".to_string()),
            ("b".to_string(), "c${a}".to_string()),
        ]);
        assert_eq!(props.get_resolve("b"), Some("cvalue".to_string()));
    }

    #[test]
    fn test() {
        print!(
            "{:#?}",
            merge_poms(
                &MavenPom {
                    model_version: "4.0.0".into(),
                    group_id: Some("marais".into()),
                    artifact_id: "jcargo".into(),
                    version: Some("0.1.0".into()),
                    parent: None,
                    properties: Some(Properties::from([(
                        "depVersion".to_string(),
                        "1.0.0".to_string()
                    )])),
                    dependencies: None,
                    dependency_management: None,
                },
                &MavenPom {
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
                    properties: None,
                    dependencies: Some(PomDependencies {
                        dependencies: vec![PomDependency {
                            group_id: "org".into(),
                            artifact_id: "somedep".into(),
                            version: Some("{depVersion}".into()),
                            scope: None,
                            r#type: None,
                        }]
                    }),
                    dependency_management: None,
                },
            )
        );
    }

    #[tokio::test]
    async fn test_real() -> anyhow::Result<()> {
        let client = Client::new();

        let pom = client
            .get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j-api/2.14.1/log4j-api-2.14.1.pom")
            .send()
            .await?
            .text()
            .await?;
        let pomC = MavenPom::parse(&pom)?;
        println!("parsed pom C {:#?}", pomC);

        let pom = client.get("https://repo.maven.apache.org/maven2/org/apache/logging/log4j/log4j/2.14.1/log4j-2.14.1.pom").send()
            .await?
            .text()
            .await?;
        let pomB = MavenPom::parse(&pom)?;
        println!("parsed pom B {:#?}", pomB);

        let pom = client.get("https://repo.maven.apache.org/maven2/org/apache/logging/logging-parent/3/logging-parent-3.pom").send()
            .await?
            .text()
            .await?;
        let pomA = MavenPom::parse(&pom)?;
        println!("parsed pom A {:#?}", pomA);

        let pom = client
            .get("https://repo.maven.apache.org/maven2/org/apache/apache/23/apache-23.pom")
            .send()
            .await?
            .text()
            .await?;
        let pomA2 = MavenPom::parse(&pom)?;
        println!("parsed pom A2 {:#?}", pomA2);

        let merged = merge_poms(&pomA2, &pomA);
        let merged = merge_poms(&merged, &pomB);
        let mut merged = merge_poms(&merged, &pomC);

        println!("merged {:#?}", merged);

        Ok(())
    }
}
