use std::collections::HashMap;
use std::ops::Deref;

use crate::dependencies::mavenpom::{
    Element, MavenPom, PomDependencies, PomDependency, Properties,
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
    let deps = if let Some(pdeps) = parent.dependencies.as_ref() {
        if let Some(cdeps) = child.dependencies.as_ref() {
            Some(merge_dependencies(pdeps, cdeps))
        } else {
            Some(pdeps.clone())
        }
    } else {
        None
    };
    let props = if let Some(pprops) = parent.properties.as_ref() {
        if let Some(cprops) = child.properties.as_ref() {
            Some(merge_properties(pprops, cprops))
        } else {
            Some(pprops.clone())
        }
    } else {
        None
    };

    MavenPom {
        model_version: "4.0.0".into(),
        group_id: child.group_id.clone().or(parent.group_id.clone()),
        artifact_id: child.artifact_id.clone(),
        version: child.version.clone().or(parent.version.clone()),
        parent: child.parent.clone(),
        properties: props,
        dependencies: deps,
        dependency_management: None,
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

#[cfg(test)]
mod tests {
    use crate::dependencies::maven::merge_poms;
    use crate::dependencies::mavenpom::{
        Element, MavenPom, ParentPom, PomDependencies, PomDependency, Properties,
    };

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
                        relative_path: "../".into(),
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
}
