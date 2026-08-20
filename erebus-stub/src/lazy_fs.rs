//! FUSE filesystem for lazy loading squashfs/zstd-tar payloads.
//!
//! Instead of extracting the entire payload to disk, we serve files on-demand
//! from the in-memory blob using FUSE. This enables lazy loading where only
//! the files actually accessed by the application are decompressed/read.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use backhand::v4::filesystem::node::InnerNode;
use backhand::v4::filesystem::reader::FilesystemReader;
use fuser::{
    BackgroundSession, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation,
    INodeNo, KernelConfig, LockOwner, OpenFlags, ReplyAttr, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request,
};
use libc::{EIO, EISDIR, ENOENT};
use tar::Archive;
use zstd::Decoder;

use crate::format::PAYLOAD_FORMAT_SQUASHFS;

/// In-memory blob representing a layer (squashfs or zstd+tar).
#[derive(Clone)]
struct LayerBlob {
    data: Vec<u8>,
    format: String, // "squashfs" or "zstd-tar"
}

/// File entry in the virtual filesystem.
#[derive(Clone)]
struct VirtualEntry {
    #[allow(dead_code)]
    path: String,
    is_dir: bool,
    size: u64,
    mode: u32,
    mtime: u64,
    layer_idx: usize,
}

/// Lazy-loading filesystem backed by in-memory layer blobs.
pub struct LazyFs {
    layers: Vec<LayerBlob>,
    entries: HashMap<String, VirtualEntry>,
}

impl LazyFs {
    /// Create a new `LazyFs` from layer blobs.
    pub fn new(layers: Vec<(Vec<u8>, String)>) -> io::Result<Self> {
        let mut fs = Self {
            layers: Vec::new(),
            entries: HashMap::new(),
        };

        for (data, format) in layers {
            fs.layers.push(LayerBlob { data, format });
        }

        // Build directory index from all layers
        fs.build_index()?;

        Ok(fs)
    }

    /// Build the virtual directory index by reading layer metadata.
    fn build_index(&mut self) -> io::Result<()> {
        let layer_count = self.layers.len();
        for idx in 0..layer_count {
            let format = self.layers[idx].format.clone();
            if format == PAYLOAD_FORMAT_SQUASHFS {
                self.index_squashfs_layer(idx)?;
            } else {
                self.index_tar_layer(idx)?;
            }
        }
        Ok(())
    }

    /// Index a squashfs layer using backhand.
    fn index_squashfs_layer(&mut self, idx: usize) -> io::Result<()> {
        let cursor = Cursor::new(self.layers[idx].data.clone());
        let fs = FilesystemReader::from_reader(cursor).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("squashfs parse: {e}"))
        })?;

        // Walk the filesystem to build entries
        for node in fs.files() {
            let rel = node.fullpath.strip_prefix("/").unwrap_or(&node.fullpath);
            let path_str = rel.to_string_lossy().to_string();

            let (is_dir, size, mode) = match &node.inner {
                InnerNode::Dir(_) => (true, 0, 0o755),
                InnerNode::File(f) => (
                    false,
                    f.file_len() as u64,
                    u32::from(node.header.permissions),
                ),
                InnerNode::Symlink(_) => (false, 0, 0o777),
                _ => continue,
            };

            let mtime = u64::from(node.header.mtime);

            self.entries.insert(
                path_str.clone(),
                VirtualEntry {
                    path: path_str,
                    is_dir,
                    size,
                    mode,
                    mtime,
                    layer_idx: idx,
                },
            );
        }
        Ok(())
    }

    /// Index a zstd-compressed tar layer.
    fn index_tar_layer(&mut self, idx: usize) -> io::Result<()> {
        let cursor = Cursor::new(self.layers[idx].data.clone());
        let decoder = Decoder::new(cursor)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("zstd: {e}")))?;
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            let is_dir = entry.header().entry_type().is_dir();
            let size = entry.size();
            let mode = entry
                .header()
                .mode()
                .unwrap_or(if is_dir { 0o755 } else { 0o644 });
            let mtime = entry.header().mtime().unwrap_or(0);

            let path_key = path.clone();
            self.entries.insert(
                path_key,
                VirtualEntry {
                    path,
                    is_dir,
                    size,
                    mode,
                    mtime,
                    layer_idx: idx,
                },
            );
        }
        Ok(())
    }

    /// Get file attribute for a path.
    fn get_attr(&self, path: &str) -> Option<FileAttr> {
        self.entries.get(path).map(|e| {
            let kind = if e.is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            FileAttr {
                ino: INodeNo(0),
                size: e.size,
                blocks: 0,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH + std::time::Duration::from_secs(e.mtime),
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind,
                perm: e.mode as u16,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
                blksize: 4096,
            }
        })
    }

    /// Read file data from the layer.
    fn read_file(&self, path: &str, offset: u64, size: u32) -> io::Result<Vec<u8>> {
        let entry = self
            .entries
            .get(path)
            .ok_or_else(|| io::Error::from_raw_os_error(ENOENT))?;
        if entry.is_dir {
            return Err(io::Error::from_raw_os_error(EISDIR));
        }

        if self.layers[entry.layer_idx].format == PAYLOAD_FORMAT_SQUASHFS {
            self.read_squashfs_file(entry.layer_idx, path, offset, size)
        } else {
            self.read_tar_file(entry.layer_idx, path, offset, size)
        }
    }

    /// Read a file from squashfs layer.
    fn read_squashfs_file(
        &self,
        layer_idx: usize,
        path: &str,
        offset: u64,
        size: u32,
    ) -> io::Result<Vec<u8>> {
        let cursor = Cursor::new(self.layers[layer_idx].data.clone());
        let fs = FilesystemReader::from_reader(cursor).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("squashfs parse: {e}"))
        })?;

        // Find the node by path
        let _rel = path.strip_prefix("/").unwrap_or(path);
        let node = fs
            .files()
            .find(|n| {
                let rel = n.fullpath.strip_prefix("/").unwrap_or(&n.fullpath);
                rel == path
            })
            .ok_or_else(|| io::Error::from_raw_os_error(ENOENT))?;

        // Get the file reader
        let file_reader = match &node.inner {
            InnerNode::File(f) => f,
            _ => return Err(io::Error::from_raw_os_error(EISDIR)),
        };

        let file_reader = fs.file(file_reader);
        let mut reader = file_reader.reader();

        // Seek to offset
        reader.seek(SeekFrom::Start(offset))?;

        // Read requested size
        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read a file from tar layer (simplified - reads whole file then slices).
    fn read_tar_file(
        &self,
        layer_idx: usize,
        path: &str,
        offset: u64,
        size: u32,
    ) -> io::Result<Vec<u8>> {
        let cursor = Cursor::new(self.layers[layer_idx].data.clone());
        let decoder = Decoder::new(cursor)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("zstd: {e}")))?;
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.to_string_lossy() == path {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                // Slice to requested range
                let start = offset as usize;
                let end = (start + size as usize).min(buf.len());
                return Ok(buf[start..end].to_vec());
            }
        }
        Err(io::Error::from_raw_os_error(ENOENT))
    }

    /// List directory entries.
    fn read_dir(&self, path: &str) -> io::Result<Vec<(String, FileAttr)>> {
        let prefix = if path == "/" {
            String::new()
        } else {
            path.strip_prefix("/").unwrap_or(path).to_string()
        };
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix)
        };

        let mut results = Vec::new();
        for entry_path in self.entries.keys() {
            if entry_path.starts_with(&prefix) {
                let rest = &entry_path[prefix.len()..];
                if !rest.contains('/') && !rest.is_empty() {
                    if let Some(attr) = self.get_attr(entry_path) {
                        results.push((rest.to_string(), attr));
                    }
                }
            }
        }
        Ok(results)
    }
}

impl Filesystem for LazyFs {
    fn init(&mut self, _req: &Request, _config: &mut KernelConfig) -> io::Result<()> {
        Ok(())
    }

    fn getattr(&self, _req: &Request, _ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let path = "/".to_string();

        if let Some(attr) = self.get_attr(&path) {
            reply.attr(&std::time::Duration::from_secs(0), &attr);
        } else {
            reply.error(Errno::ENOENT);
        }
    }

    fn lookup(&self, _req: &Request, _parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let name = name.to_string_lossy();
        let path = format!("/{}", name);

        if let Some(attr) = self.get_attr(&path) {
            reply.entry(&std::time::Duration::from_secs(0), &attr, Generation(0));
        } else {
            reply.error(Errno::ENOENT);
        }
    }

    fn open(&self, _req: &Request, _ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        reply.opened(
            FileHandle(0),
            FopenFlags::from_bits_truncate(flags.0.cast_unsigned()),
        );
    }

    fn read(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock: Option<LockOwner>,
        reply: fuser::ReplyData,
    ) {
        // Simplified: always read root for now
        let path = "/".to_string();
        match self.read_file(&path, offset, size) {
            Ok(data) => reply.data(&data),
            Err(e) => reply.error(Errno::from_i32(e.raw_os_error().unwrap_or(EIO))),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let path = "/".to_string();
        let entries = self.read_dir(&path).unwrap_or_default();

        for (i, (name, attr)) in entries.into_iter().enumerate() {
            let ino = (i + 1) as u64;
            let kind = if attr.kind == FileType::Directory {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            if reply.add(INodeNo(ino), offset, kind, &name) {
                break;
            }
        }
        reply.ok();
    }
}

/// Mount a `LazyFs` at the given mountpoint.
/// Returns a handle that must be kept alive to maintain the mount.
pub fn mount_lazy_fs(
    layers: Vec<(Vec<u8>, String)>,
    mountpoint: &Path,
) -> io::Result<BackgroundSession> {
    let fs = LazyFs::new(layers)?;
    let config = fuser::Config::default();
    fuser::spawn_mount(fs, mountpoint, &config)
}
