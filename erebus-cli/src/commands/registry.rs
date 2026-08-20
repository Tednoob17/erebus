use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};
use erebus_core::registry::{ArtifactRecord, LayerRecord, Registry};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct RegistryArgs {
    /// Registry URL (e.g., `<https://registry.example.com/v1>`)
    #[arg(short, long, env = "ERE_REGISTRY")]
    pub registry: String,

    /// Authentication token (Bearer auth)
    #[arg(short, long, env = "ERE_TOKEN")]
    pub token: Option<String>,

    #[command(subcommand)]
    pub command: RegistryCommand,
}

#[derive(Subcommand, Debug)]
pub enum RegistryCommand {
    /// Push a layer to the registry
    PushLayer {
        /// Path to the layer file
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Pull a layer from the registry
    PullLayer {
        /// Layer hash (hex-encoded SHA-256) or "latest"
        #[arg(short, long)]
        hash: String,

        /// Output path for the downloaded layer
        #[arg(short, long)]
        output: PathBuf,
    },

    /// List all layer hashes in the registry
    ListLayers,

    /// Publish an artifact's metadata to the registry (layers must be pushed first)
    PublishArtifact {
        /// Path to the .erebus binary or metadata JSON file
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Inspect an artifact in the registry
    InspectArtifact {
        /// Artifact hash (hex-encoded SHA-256 of the .erebus binary)
        #[arg(short, long)]
        hash: String,
    },

    /// List all artifacts in the registry
    ListArtifacts,
}

pub fn run(args: RegistryArgs) -> Result<()> {
    let mut client = HttpRegistryClient::new(&args.registry, args.token.clone());

    match args.command {
        RegistryCommand::PushLayer { file } => {
            let data = std::fs::read(&file)
                .with_context(|| format!("Failed to read {}", file.display()))?;
            let hash = Sha256::digest(&data);
            let hash_hex = client.push_layer(&hash.into(), &data)?;
            println!("Pushed layer {} ({} bytes)", hash_hex, data.len());
            Ok(())
        }
        RegistryCommand::PullLayer { hash, output } => {
            let hash_bytes: [u8; 32] = hex::decode(&hash)
                .with_context(|| format!("Invalid hex hash: {hash}"))?
                .try_into()
                .map_err(|_| anyhow!("Hash must be 32 bytes (64 hex chars)"))?;
            let data = client
                .pull_layer(&hash_bytes)?
                .ok_or_else(|| anyhow!("Layer not found: {hash}"))?;
            std::fs::write(&output, &data)
                .with_context(|| format!("Failed to write {}", output.display()))?;
            println!(
                "Pulled layer {} to {} ({} bytes)",
                hash,
                output.display(),
                data.len()
            );
            Ok(())
        }
        RegistryCommand::ListLayers => {
            let layers = client.list_layers()?;
            println!("Layers in registry ({}):", layers.len());
            for hash in &layers {
                println!("  {hash}");
            }
            Ok(())
        }
        RegistryCommand::PublishArtifact { file } => {
            let data = std::fs::read(&file)
                .with_context(|| format!("Failed to read {}", file.display()))?;

            // If it's a .erebus file, extract metadata from footer
            let (artifact_hash, metadata_json, layer_records) = if file
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("erebus"))
            {
                extract_metadata_from_erebus(&data, &file)?
            } else {
                // Treat as JSON metadata file
                let meta: serde_json::Value =
                    serde_json::from_slice(&data).context("Failed to parse metadata JSON")?;
                (compute_binary_hash(&file)?, meta, vec![])
            };

            let artifact = ArtifactRecord {
                hash: hex::encode(artifact_hash),
                name: metadata_json["name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                metadata: metadata_json,
                layers: layer_records,
            };

            let result_hash = client.publish_artifact(&artifact)?;
            println!("Published artifact {} ({})", result_hash, artifact.name);
            Ok(())
        }
        RegistryCommand::InspectArtifact { hash } => {
            let hash_bytes: [u8; 32] = hex::decode(&hash)
                .with_context(|| format!("Invalid hex hash: {hash}"))?
                .try_into()
                .map_err(|_| anyhow!("Hash must be 32 bytes (64 hex chars)"))?;

            let artifact = client
                .inspect_artifact(&hash_bytes)?
                .ok_or_else(|| anyhow!("Artifact not found: {hash}"))?;

            println!("Artifact: {}", artifact.name);
            println!("Hash:     {}", artifact.hash);
            println!("Layers:   {}", artifact.layers.len());
            for layer in &artifact.layers {
                println!("  - {} ({} bytes)", layer.hash, layer.size);
            }
            println!("\nMetadata:");
            println!("{}", serde_json::to_string_pretty(&artifact.metadata)?);
            Ok(())
        }
        RegistryCommand::ListArtifacts => {
            let artifacts = client.list_artifacts()?;
            println!("Artifacts in registry ({}):", artifacts.len());
            for hash in &artifacts {
                println!("  {hash}");
            }
            Ok(())
        }
    }
}

/// HTTP-backed Registry implementation using reqwest.
pub struct HttpRegistryClient {
    base_url: String,
    client: reqwest::blocking::Client,
    #[allow(dead_code)]
    token: Option<String>,
}

impl HttpRegistryClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        let mut builder =
            reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(30));
        if let Some(ref t) = token {
            builder = builder.default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {t}")).unwrap(),
                );
                h
            });
        }
        let client = builder.build().expect("failed to build HTTP client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            token,
        }
    }
}

impl Registry for HttpRegistryClient {
    fn push_layer(&mut self, hash: &[u8; 32], data: &[u8]) -> std::io::Result<String> {
        let url = format!("{}/layers/{}", self.base_url, hex::encode(hash));
        let resp = self
            .client
            .put(&url)
            .body(data.to_vec())
            .header("Content-Type", "application/octet-stream")
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("upload failed: {}", resp.status()),
            ));
        }
        Ok(hex::encode(hash))
    }

    fn pull_layer(&self, hash: &[u8; 32]) -> std::io::Result<Option<Vec<u8>>> {
        let url = format!("{}/layers/{}", self.base_url, hex::encode(hash));
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if resp.status() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("download failed: {}", resp.status()),
            ));
        }
        Ok(Some(
            resp.bytes()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                .to_vec(),
        ))
    }

    fn publish_artifact(&mut self, artifact: &ArtifactRecord) -> std::io::Result<String> {
        let url = format!("{}/artifacts", self.base_url);
        let json = serde_json::to_vec(artifact)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let resp = self
            .client
            .post(&url)
            .body(json)
            .header("Content-Type", "application/json")
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("publish failed: {}", resp.status()),
            ));
        }
        Ok(artifact.hash.clone())
    }

    fn inspect_artifact(&self, hash: &[u8; 32]) -> std::io::Result<Option<ArtifactRecord>> {
        let url = format!("{}/artifacts/{}", self.base_url, hex::encode(hash));
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if resp.status() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("inspect failed: {}", resp.status()),
            ));
        }
        let record: ArtifactRecord = serde_json::from_slice(
            &resp
                .bytes()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(record))
    }

    fn list_layers(&self) -> std::io::Result<Vec<String>> {
        let url = format!("{}/layers", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("list failed: {}", resp.status()),
            ));
        }
        let body: serde_json::Value = serde_json::from_slice(
            &resp
                .bytes()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let list = body["layers"].as_array().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "expected layers array")
        })?;
        Ok(list
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
    }

    fn list_artifacts(&self) -> std::io::Result<Vec<String>> {
        let url = format!("{}/artifacts", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if !resp.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("list failed: {}", resp.status()),
            ));
        }
        let body: serde_json::Value = serde_json::from_slice(
            &resp
                .bytes()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let list = body["artifacts"].as_array().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "expected artifacts array")
        })?;
        Ok(list
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
    }
}

fn extract_metadata_from_erebus(
    data: &[u8],
    _path: &std::path::Path,
) -> Result<([u8; 32], serde_json::Value, Vec<LayerRecord>)> {
    use erebus_core::format::{read_at, Footer};
    use std::io::Cursor;

    let mut cursor = Cursor::new(data);
    let footer =
        Footer::read_from(&mut cursor).context("Failed to read footer from .erebus file")?;

    // Read metadata bytes
    let meta_bytes = read_at(&mut cursor, footer.meta_offset, footer.meta_size as usize)
        .context("Failed to read metadata from .erebus file")?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&meta_bytes).context("Failed to parse metadata JSON")?;

    // Compute layer records from metadata.layers
    let layer_records: Vec<LayerRecord> = metadata["layers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|l| {
                    let hash = l["payload_sha256"].as_str()?;
                    let size = l["size"].as_u64().unwrap_or(0);
                    Some(LayerRecord {
                        hash: hash.to_string(),
                        size,
                        content_type: "application/octet-stream".to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // The file hash is the SHA-256 of the binary
    let file_hash: [u8; 32] = Sha256::digest(data).into();

    Ok((file_hash, metadata, layer_records))
}

fn compute_binary_hash(path: &Path) -> Result<[u8; 32]> {
    let data = std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(Sha256::digest(&data).into())
}
