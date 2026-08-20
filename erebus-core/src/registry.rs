//! Content-addressable registry for erebus layers and artifacts.
//!
//! Defines the `Registry` trait and an `HttpRegistry` implementation that
//! speaks HTTP/HTTPS with bearer-token auth. The transport (reqwest) is
//! optional — `HttpRegistry` is gated behind the `http` feature so the core
//! library stays lightweight and dependency-free for `no_std` builds.
//!
//! ## API
//! - `push_layer(hash, data)`   — upload a layer blob, returns its hash
//! - `pull_layer(hash)`         — download a layer blob by content hash
//! - `publish_artifact(meta)`   — register artifact metadata (layers already pushed)
//! - `list_layers()`            — list all layer hashes in the registry
//! - `inspect_artifact(hash)`  — fetch artifact metadata by SHA-256
//!
//! ## Protocol
//! Minimal REST surface:
//! ```text
//! GET  /v1/layers/{hash}          → download layer blob
//! PUT  /v1/layers/{hash}          → upload layer blob (auth: Bearer)
//! GET  /v1/layers                 → list layer hashes
//! POST /v1/artifacts              → publish artifact metadata
//! GET  /v1/artifacts/{hash}       → inspect artifact metadata
//! GET  /v1/artifacts              → list artifact hashes
//! ```

use std::io;

use sha2::{Digest, Sha256};

/// A content-addressed layer blob.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayerRecord {
    /// SHA-256 of the layer payload (hex).
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
    /// MIME or content-type hint (e.g. "application/octet-stream", "application/gzip").
    #[serde(default)]
    pub content_type: String,
}

/// Artifact metadata published to a registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactRecord {
    /// SHA-256 of the `.erebus` binary (`footer.payload_sha256`).
    pub hash: String,
    /// Artifact name.
    pub name: String,
    /// Artifact metadata JSON (the same blob embedded in the binary footer).
    pub metadata: serde_json::Value,
    /// Layer records referenced by this artifact.
    #[serde(default)]
    pub layers: Vec<LayerRecord>,
}

/// Trait for a content-addressable registry.
///
/// Implementations can be in-memory (`MemoryRegistry`), on-disk (`DiskRegistry`),
/// or remote HTTP (`HttpRegistry` behind the `http` feature).
pub trait Registry {
    /// Upload a layer blob. The `hash` must be `SHA-256(data)`.
    fn push_layer(&mut self, hash: &[u8; 32], data: &[u8]) -> io::Result<String>;

    /// Download a layer blob by content hash. Returns `None` if not found.
    fn pull_layer(&self, hash: &[u8; 32]) -> io::Result<Option<Vec<u8>>>;

    /// Publish artifact metadata (layers must already be pushed).
    fn publish_artifact(&mut self, artifact: &ArtifactRecord) -> io::Result<String>;

    /// Fetch artifact metadata by binary SHA-256.
    fn inspect_artifact(&self, hash: &[u8; 32]) -> io::Result<Option<ArtifactRecord>>;

    /// List all layer hashes in the registry.
    fn list_layers(&self) -> io::Result<Vec<String>>;

    /// List all artifact hashes in the registry.
    fn list_artifacts(&self) -> io::Result<Vec<String>>;
}

/// In-memory registry mock for tests and tools.
#[derive(Default)]
pub struct MemoryRegistry {
    layers: std::collections::HashMap<[u8; 32], Vec<u8>>,
    artifacts: std::collections::HashMap<[u8; 32], ArtifactRecord>,
}

impl MemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Registry for MemoryRegistry {
    fn push_layer(&mut self, hash: &[u8; 32], data: &[u8]) -> io::Result<String> {
        let actual = Sha256::digest(data);
        if actual.as_slice() != hash.as_slice() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "hash mismatch: data does not match provided hash",
            ));
        }
        self.layers.insert(*hash, data.to_vec());
        Ok(hex::encode(hash))
    }

    fn pull_layer(&self, hash: &[u8; 32]) -> io::Result<Option<Vec<u8>>> {
        match self.layers.get(hash) {
            None => Ok(None),
            Some(data) => {
                let actual = Sha256::digest(data);
                if actual.as_slice() != hash.as_slice() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "stored layer corrupted (hash mismatch on read)",
                    ));
                }
                Ok(Some(data.clone()))
            }
        }
    }

    fn publish_artifact(&mut self, artifact: &ArtifactRecord) -> io::Result<String> {
        let hash_bytes: [u8; 32] = hex::decode(&artifact.hash)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid artifact hash hex"))?
            .try_into()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "artifact hash must be 32 bytes",
                )
            })?;
        self.artifacts.insert(hash_bytes, artifact.clone());
        Ok(artifact.hash.clone())
    }

    fn inspect_artifact(&self, hash: &[u8; 32]) -> io::Result<Option<ArtifactRecord>> {
        Ok(self.artifacts.get(hash).cloned())
    }

    fn list_layers(&self) -> io::Result<Vec<String>> {
        Ok(self.layers.keys().map(hex::encode).collect())
    }

    fn list_artifacts(&self) -> io::Result<Vec<String>> {
        Ok(self.artifacts.keys().map(hex::encode).collect())
    }
}

/// On-disk registry backed by an `ObjectStore`.
/// Stores `layers/{hash}` and `artifacts/{hash}.json` under the root dir.
pub struct DiskRegistry {
    layers_root: std::path::PathBuf,
    artifacts_root: std::path::PathBuf,
}

impl DiskRegistry {
    pub fn new<P: AsRef<std::path::Path>>(root: P) -> io::Result<Self> {
        let layers_root = root.as_ref().join("layers");
        let artifacts_root = root.as_ref().join("artifacts");
        std::fs::create_dir_all(&layers_root)?;
        std::fs::create_dir_all(&artifacts_root)?;
        Ok(Self {
            layers_root,
            artifacts_root,
        })
    }

    fn layer_path(&self, hash: &[u8; 32]) -> std::path::PathBuf {
        self.layers_root.join(hex::encode(hash))
    }

    fn artifact_path(&self, hash: &[u8; 32]) -> std::path::PathBuf {
        self.artifacts_root
            .join(format!("{}.json", hex::encode(hash)))
    }
}

impl Registry for DiskRegistry {
    fn push_layer(&mut self, hash: &[u8; 32], data: &[u8]) -> io::Result<String> {
        let actual = Sha256::digest(data);
        if actual.as_slice() != hash.as_slice() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "hash mismatch: data does not match provided hash",
            ));
        }
        let path = self.layer_path(hash);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(hex::encode(hash))
    }

    fn pull_layer(&self, hash: &[u8; 32]) -> io::Result<Option<Vec<u8>>> {
        let path = self.layer_path(hash);
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let actual = Sha256::digest(&data);
        if actual.as_slice() != hash.as_slice() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stored layer corrupted (hash mismatch on read)",
            ));
        }
        Ok(Some(data))
    }

    fn publish_artifact(&mut self, artifact: &ArtifactRecord) -> io::Result<String> {
        let hash_bytes: [u8; 32] = hex::decode(&artifact.hash)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid artifact hash hex"))?
            .try_into()
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "artifact hash must be 32 bytes",
                )
            })?;
        let path = self.artifact_path(&hash_bytes);
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_vec(artifact)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("json: {e}")))?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(artifact.hash.clone())
    }

    fn inspect_artifact(&self, hash: &[u8; 32]) -> io::Result<Option<ArtifactRecord>> {
        let path = self.artifact_path(hash);
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let record: ArtifactRecord = serde_json::from_slice(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("json: {e}")))?;
        Ok(Some(record))
    }

    fn list_layers(&self) -> io::Result<Vec<String>> {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&self.layers_root)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                result.push(name.to_string());
            }
        }
        Ok(result)
    }

    fn list_artifacts(&self) -> io::Result<Vec<String>> {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&self.artifacts_root)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if std::path::Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                {
                    result.push(name.trim_end_matches(".json").to_string());
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_registry_roundtrip() {
        let mut reg = MemoryRegistry::new();
        let data = b"layer payload contents";
        let hash = Sha256::digest(data);

        let result = reg.push_layer(&hash.into(), data).unwrap();
        assert_eq!(result, hex::encode(hash));

        let pulled = reg.pull_layer(&hash.into()).unwrap().unwrap();
        assert_eq!(pulled, data);
    }

    #[test]
    fn memory_registry_rejects_wrong_hash() {
        let mut reg = MemoryRegistry::new();
        let wrong = [0u8; 32];
        assert!(reg.push_layer(&wrong, b"data").is_err());
    }

    #[test]
    fn memory_registry_missing_returns_none() {
        let reg = MemoryRegistry::new();
        let absent = [0u8; 32];
        assert_eq!(reg.pull_layer(&absent).unwrap(), None);
    }

    #[test]
    fn artifact_publish_and_inspect() {
        let mut reg = MemoryRegistry::new();
        let data = b"payload";
        let hash = Sha256::digest(data);

        reg.push_layer(&hash.into(), data).unwrap();

        let artifact = ArtifactRecord {
            hash: hex::encode(hash),
            name: "test-app".into(),
            metadata: serde_json::json!({"name": "test-app"}),
            layers: vec![LayerRecord {
                hash: hex::encode(hash),
                size: data.len() as u64,
                content_type: "application/octet-stream".into(),
            }],
        };

        reg.publish_artifact(&artifact).unwrap();
        let fetched = reg.inspect_artifact(&hash.into()).unwrap().unwrap();
        assert_eq!(fetched.name, "test-app");
        assert_eq!(fetched.layers.len(), 1);
    }

    #[test]
    fn disk_registry_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = DiskRegistry::new(dir.path()).unwrap();

        let data = b"disk layer blob";
        let hash = Sha256::digest(data);

        reg.push_layer(&hash.into(), data).unwrap();
        let pulled = reg.pull_layer(&hash.into()).unwrap().unwrap();
        assert_eq!(pulled, data);

        let layers = reg.list_layers().unwrap();
        assert_eq!(layers.len(), 1);
    }
}
