//! Snap packaging support.
//!
//! Generates `snapcraft.yaml` from erebus artifact metadata, enabling
//! `snapcraft` to produce distributable Snap packages.

use std::collections::BTreeMap;
use std::io;

use serde::{Deserialize, Serialize};

/// A snapcraft.yaml configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapConfig {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps: Option<BTreeMap<String, SnapApp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<BTreeMap<String, SnapService>>,
    #[serde(rename = "parts", skip_serializing_if = "BTreeMap::is_empty")]
    pub parts: BTreeMap<String, SnapPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confinement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapApp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugs: Option<Vec<String>>,
    #[serde(rename = "command-chain", skip_serializing_if = "Option::is_none")]
    pub command_chain: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(rename = "source", skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "stage")]
    pub stage: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", rename = "organize")]
    pub organize: BTreeMap<String, String>,
}

/// Generate a snapcraft.yaml from artifact metadata.
pub fn generate_snapcraft_yaml(
    app_name: &str,
    version: &str,
    description: Option<&str>,
    runtime: &str,
    entrypoint: &[String],
    artifact_type: &crate::metadata::ArtifactType,
) -> io::Result<SnapConfig> {
    let command = entrypoint
        .first()
        .cloned()
        .unwrap_or_else(|| runtime.to_string());

    let mut apps = BTreeMap::new();
    apps.insert(
        app_name.to_string(),
        SnapApp {
            command: Some(command.clone()),
            plugs: Some(vec!["network".to_string(), "home".to_string()]),
            command_chain: None,
        },
    );

    let mut parts = BTreeMap::new();
    parts.insert(
        "erebus-app".to_string(),
        SnapPart {
            plugin: Some("dump".to_string()),
            source: Some(".".to_string()),
            stage: vec![],
            organize: BTreeMap::new(),
        },
    );

    Ok(SnapConfig {
        name: app_name.to_string(),
        version: version.to_string(),
        summary: Some(format!("{} application", app_name)),
        description: description.map(str::to_string),
        apps: Some(apps),
        services: if matches!(artifact_type, crate::metadata::ArtifactType::Service) {
            let mut services = BTreeMap::new();
            services.insert(
                app_name.to_string(),
                SnapService {
                    command: Some(command),
                    plugs: Some(vec!["network".to_string()]),
                },
            );
            Some(services)
        } else {
            None
        },
        parts,
        base: Some("core22".to_string()),
        confinement: Some("strict".to_string()),
        grade: Some("stable".to_string()),
    })
}

/// Generate the snapcraft.yaml as a string.
pub fn generate_yaml_string(config: &SnapConfig) -> io::Result<String> {
    serde_yaml::to_string(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("yaml: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_config_generation() {
        let config = generate_snapcraft_yaml(
            "myapp",
            "1.0.0",
            Some("A test app"),
            "python3",
            &["python3".to_string(), "/app/main.py".to_string()],
            &crate::metadata::ArtifactType::Application,
        )
        .unwrap();

        assert_eq!(config.name, "myapp");
        assert_eq!(config.version, "1.0.0");
        assert!(config.apps.is_some());
        assert!(config.apps.as_ref().unwrap().contains_key("myapp"));
    }

    #[test]
    fn snap_service_generation() {
        let config = generate_snapcraft_yaml(
            "myapi",
            "2.0.0",
            None,
            "python3",
            &["python3".to_string(), "/app/api.py".to_string()],
            &crate::metadata::ArtifactType::Service,
        )
        .unwrap();

        assert!(config.services.is_some());
        assert!(config.apps.is_some());
    }

    #[test]
    fn yaml_string_generation() {
        let config = generate_snapcraft_yaml(
            "test",
            "1.0.0",
            Some("test"),
            "python3",
            &["python3".to_string(), "/app/main.py".to_string()],
            &crate::metadata::ArtifactType::Application,
        )
        .unwrap();

        let yaml = generate_yaml_string(&config).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: 1.0.0"));
    }
}
