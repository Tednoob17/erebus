use anyhow::{anyhow, Context, Result};
use clap::Args;
use ed25519_dalek::SigningKey;
use erebus_core::{
    assembly::{assemble_erebus, AssemblyInput},
    format::Footer,
    sisr_stage::{build_artifacts, SisrBuildConfig},
};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Path to the .erebus binary to modify
    pub binary: PathBuf,

    /// Name of the layer to swap (e.g., "model", "runtime", "config")
    pub layer: String,

    /// Path to the new layer content (file or directory)
    pub new_content: PathBuf,

    /// Output path for the modified binary (default: overwrites input with backup)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Generate SISR delta update instead of full rebuild (requires SISR-enabled binary)
    #[arg(long)]
    pub sisr_delta: bool,

    /// Signing key for SISR manifest (required if --sisr-delta and binary was signed)
    #[arg(long)]
    pub key: Option<PathBuf>,

    /// Skip backup of original binary
    #[arg(long)]
    pub no_backup: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn run(args: SwapArgs, verbose: bool) -> Result<()> {
    let input_path = &args.binary;
    let layer_name = &args.layer;
    let new_content_path = &args.new_content;

    if verbose {
        eprintln!("Reading binary: {}", input_path.display());
    }

    // Read the binary file
    let mut file = fs::File::open(input_path)
        .with_context(|| format!("Failed to open {}", input_path.display()))?;

    // Read footer
    let footer = Footer::read_from(&mut file)
        .with_context(|| "Failed to read footer - not a valid .erebus file")?;

    let format_version = footer.format_version;
    let is_signed = footer.is_signed();
    let has_sisr = footer.has_sisr();

    if verbose {
        eprintln!("Format version: {}", format_version);
        eprintln!("Signed: {}", is_signed);
        eprintln!("Has SISR: {}", has_sisr);
    }

    // Read metadata
    file.seek(SeekFrom::Start(footer.meta_offset))?;
    let mut meta_bytes = vec![0u8; footer.meta_size as usize];
    file.read_exact(&mut meta_bytes)?;

    let metadata: serde_json::Value =
        serde_json::from_slice(&meta_bytes).context("Failed to parse metadata JSON")?;

    // Check if layer exists in metadata
    let layers = metadata["layers"]
        .as_array()
        .ok_or_else(|| anyhow!("No layers found in metadata"))?;

    let target_layer = layers
        .iter()
        .find(|l| l["name"].as_str() == Some(layer_name))
        .ok_or_else(|| anyhow!("Layer '{}' not found in metadata", layer_name))?;

    let layer_kind = target_layer["kind"]
        .as_str()
        .ok_or_else(|| anyhow!("Layer '{}' has no kind field", layer_name))?;

    if verbose {
        eprintln!("Found layer: {} (kind: {})", layer_name, layer_kind);
    }

    // Read the new content
    let new_payload = if new_content_path.is_dir() {
        // Create tar archive of directory
        create_tar_archive(new_content_path)?
    } else {
        fs::read(new_content_path)
            .with_context(|| format!("Failed to read {}", new_content_path.display()))?
    };

    if verbose {
        eprintln!("New payload size: {} bytes", new_payload.len());
    }

    // Extract stub bytes
    file.seek(SeekFrom::Start(0))?;
    let mut stub_bytes = vec![0u8; footer.payload_offset as usize];
    file.read_exact(&mut stub_bytes)?;

    // Read original payload (for SISR delta computation)
    let original_payload = if has_sisr && args.sisr_delta {
        file.seek(SeekFrom::Start(footer.payload_offset))?;
        let mut payload = vec![0u8; footer.payload_csize as usize];
        file.read_exact(&mut payload)?;
        Some(payload)
    } else {
        None
    };

    // Compute new payload hash
    let new_payload_hash = sha2_hash(&new_payload);

    // Build new metadata with updated layer hash
    let mut new_metadata = metadata.clone();
    if let Some(layers_arr) = new_metadata["layers"].as_array_mut() {
        for layer in layers_arr {
            if layer["name"].as_str() == Some(layer_name) {
                layer["payload_sha256"] = serde_json::Value::String(hex::encode(new_payload_hash));
                break;
            }
        }
    }

    let new_meta_bytes =
        serde_json::to_vec(&new_metadata).context("Failed to serialize new metadata")?;

    // Handle SISR delta update
    let sisr_artifacts = if has_sisr && args.sisr_delta {
        if original_payload.is_some() {
            if verbose {
                eprintln!("Computing SISR delta...");
            }

            let signing_key = if let Some(key_path) = &args.key {
                let key_bytes = fs::read(key_path).with_context(|| {
                    format!("Failed to read signing key {}", key_path.display())
                })?;
                Some(SigningKey::from_bytes(&key_bytes[..32].try_into()?))
            } else if is_signed {
                return Err(anyhow!(
                    "Binary is signed; --key required for SISR delta update"
                ));
            } else {
                None
            };

            let config = SisrBuildConfig {
                enabled: true,
                chunk_target_size: 8192,
                signing_key,
            };

            Some(build_artifacts(&new_payload, &config).context("Failed to build SISR artifacts")?)
        } else {
            None
        }
    } else {
        None
    };

    // Prepare output path
    let output_path = args.output.clone().unwrap_or_else(|| input_path.clone());
    let backup_path = if output_path == *input_path && !args.no_backup {
        let mut backup = input_path.clone();
        backup.set_extension("erebus.bak");
        if verbose {
            eprintln!("Creating backup: {}", backup.display());
        }
        fs::copy(input_path, &backup)
            .with_context(|| format!("Failed to create backup at {}", backup.display()))?;
        Some(backup)
    } else {
        None
    };

    // Assemble new binary
    if verbose {
        eprintln!("Assembling new binary...");
    }

    let payload_format = if metadata["payload_format"].as_str() == Some("squashfs") {
        "squashfs"
    } else {
        "zstd-tar"
    };

    let assemble_input = AssemblyInput {
        stub_bytes: &stub_bytes,
        payload: &new_payload,
        meta_bytes: &new_meta_bytes,
        encrypt: footer.format_version >= 4,
        squashfs: payload_format == "squashfs",
        target_arch: None,
        sisr: sisr_artifacts,
    };

    assemble_erebus(&output_path, &assemble_input).context("Failed to assemble new binary")?;

    if verbose {
        eprintln!("Successfully created: {}", output_path.display());
        if let Some(bak) = backup_path {
            eprintln!("Backup saved to: {}", bak.display());
        }
    }

    Ok(())
}

fn sha2_hash(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn create_tar_archive(dir: &Path) -> Result<Vec<u8>> {
    use std::process::Command;
    let output = Command::new("tar")
        .args(["-czf", "-", "-C", dir.to_str().unwrap(), "."])
        .output()
        .context("Failed to run tar command")?;
    if !output.status.success() {
        return Err(anyhow!(
            "tar failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}
