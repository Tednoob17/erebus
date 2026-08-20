//! Flatpak packaging support.
//!
//! Generates Flatpak manifests (`flatpak-manifest.json` or `yml`) from
//! erebus artifact metadata, enabling `flatpak-builder` to produce
//! distributable Flatpak applications.
//!
//! ## Output formats
//! - `--package-format flatpak` → generates `manifest.json` + `.desktop` file
//! - The manifest references the erebus stub + extracted rootfs as a local
//!   filesystem extension point that `flatpak-builder` packages.

use std::io;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum PackageFormat {
    #[default]
    Erebus,
    Flatpak,
    Snap,
}

impl std::str::FromStr for PackageFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "erebus" | "default" => Ok(Self::Erebus),
            "flatpak" => Ok(Self::Flatpak),
            "snap" => Ok(Self::Snap),
            other => Err(format!("unknown package format: '{other}'")),
        }
    }
}

impl std::fmt::Display for PackageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Erebus => write!(f, "erebus"),
            Self::Flatpak => write!(f, "flatpak"),
            Self::Snap => write!(f, "snap"),
        }
    }
}

/// A Flatpak application manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatpakManifest {
    pub app_id: String,
    pub runtime: String,
    pub sdk: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_args: Option<std::collections::BTreeMap<String, Vec<String>>>,
    #[serde(rename = "modules")]
    pub modules: Vec<FlatpakModule>,
}

/// A module in the Flatpak manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatpakModule {
    pub name: String,
    #[serde(rename = "buildsystem")]
    pub build_system: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<FlatpakSource>,
}

/// A source in a Flatpak module.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FlatpakSource {
    #[serde(rename = "file")]
    File {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dest: Option<String>,
    },
}

/// Flatpak runtime mappings for common erebus runtimes.
/// Maps erebus runtime name → (`flatpak_runtime`, `flatpak_sdk`).
pub fn runtime_mapping(runtime: &str) -> Option<(&'static str, &'static str)> {
    match runtime.to_lowercase().as_str() {
        "python" | "python3" | "node" | "nodejs" | "ruby" | "deno" | "java" | "go" | "binary" => {
            Some(("org.freedesktop.Platform", "org.freedesktop.Sdk"))
        }
        _ => None,
    }
}

/// Generate a Flatpak manifest from artifact metadata.
pub fn generate_manifest(
    app_name: &str,
    runtime: &str,
    entrypoint: &[String],
    layers: &[crate::layer::SerializableLayer],
) -> io::Result<FlatpakManifest> {
    let (flatpak_runtime, flatpak_sdk) =
        runtime_mapping(runtime).unwrap_or(("org.freedesktop.Platform", "org.freedesktop.Sdk"));

    let command = entrypoint
        .first()
        .cloned()
        .unwrap_or_else(|| runtime.to_string());

    let mut modules = Vec::new();

    for layer in layers {
        let module = FlatpakModule {
            name: layer.name().to_string(),
            build_system: "simple".to_string(),
            sources: vec![],
        };
        modules.push(module);
    }

    if modules.is_empty() {
        modules.push(FlatpakModule {
            name: "app".to_string(),
            build_system: "simple".to_string(),
            sources: vec![],
        });
    }

    let mut finish_args = std::collections::BTreeMap::new();
    finish_args.insert("--share=network".to_string(), vec![]);
    finish_args.insert("--socket=x11".to_string(), vec![]);
    finish_args.insert("--socket=wayland".to_string(), vec![]);
    finish_args.insert("--device=dri".to_string(), vec![]);

    Ok(FlatpakManifest {
        app_id: sanitize_app_id(app_name),
        runtime: flatpak_runtime.to_string(),
        sdk: flatpak_sdk.to_string(),
        command,
        finish_args: Some(finish_args),
        modules,
    })
}

/// Generate a `.desktop` file for the Flatpak application.
pub fn generate_desktop_file(app_name: &str, display_name: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={display_name}\n\
         Exec={app_name}\n\
         Icon=application-x-executable\n\
         Terminal=false\n\
         Categories=Utility;\n"
    )
}

/// Sanitize an app name into a valid Flatpak application ID.
fn sanitize_app_id(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();

    if sanitized.starts_with(|c: char| c.is_numeric()) {
        format!("app.{sanitized}")
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_runtime_mapping() {
        assert_eq!(
            runtime_mapping("python3"),
            Some(("org.freedesktop.Platform", "org.freedesktop.Sdk"))
        );
        assert_eq!(
            runtime_mapping("node"),
            Some(("org.freedesktop.Platform", "org.freedesktop.Sdk"))
        );
        assert!(runtime_mapping("unknown-runtime").is_none());
    }

    #[test]
    fn manifest_generation() {
        let layers: Vec<crate::layer::SerializableLayer> = vec![];
        let manifest = generate_manifest(
            "myapp",
            "python3",
            &["python3".to_string(), "/app/main.py".to_string()],
            &layers,
        )
        .unwrap();

        assert_eq!(manifest.app_id, "myapp");
        assert_eq!(manifest.runtime, "org.freedesktop.Platform");
        assert_eq!(manifest.sdk, "org.freedesktop.Sdk");
        assert_eq!(manifest.command, "python3");
        assert!(!manifest.modules.is_empty());
    }

    #[test]
    fn sanitize_app_id() {
        assert_eq!(super::sanitize_app_id("my-app"), "my-app");
        assert_eq!(super::sanitize_app_id("my app"), "my-app");
        assert_eq!(super::sanitize_app_id("123app"), "app.123app");
    }

    #[test]
    fn desktop_file_generation() {
        let content = generate_desktop_file("myapp", "My Application");
        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Name=My Application"));
        assert!(content.contains("Exec=myapp"));
    }

    #[test]
    fn package_format_parse() {
        assert_eq!(
            "flatpak".parse::<PackageFormat>().unwrap(),
            PackageFormat::Flatpak
        );
        assert_eq!(
            "snap".parse::<PackageFormat>().unwrap(),
            PackageFormat::Snap
        );
        assert!("unknown".parse::<PackageFormat>().is_err());
    }
}
