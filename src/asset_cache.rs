use std::collections::HashMap;
use std::future::ready;
use std::io::Write as _;
use std::path;

use axum::extract::Path;
use bytes::Bytes;
use futures_util::stream::StreamExt as _;
use tokio_stream::wrappers::ReadDirStream;
use tracing::info;

const HASH_SPLIT_CHAR: char = '.';

/// Maps static asset filenames to their compressed bytes and content type. This
/// is used to serve static assets from the build directory without reading from
/// disk, as the cache stays in RAM for the life of the server.
///
/// This type should be accessed via the `cache` property in `AppState`.
#[derive(Debug)]
pub struct AssetCache(HashMap<String, StaticAsset>);

impl AssetCache {
    /// Attempts to return a static asset from the cache from a cache key. If
    /// the asset is not found, `None` is returned.
    pub fn get(&self, key: &str) -> Option<&StaticAsset> {
        self.0.get(key)
    }

    /// Helper method to get a static asset from an extracted request path.
    pub fn get_from_path(&self, path: &Path<String>) -> Option<&StaticAsset> {
        let key = Self::get_cache_key(path);
        self.get(&key)
    }

    fn get_cache_key(path: &str) -> String {
        let mut parts = path.split(|c| c == '.' || c == HASH_SPLIT_CHAR);

        let basename = parts.next().unwrap_or_default();
        let ext = parts.last().unwrap_or_default();

        format!("{}.{}", basename, ext)
    }

    pub async fn load_files(dir: &path::Path) -> color_eyre::Result<Self> {
        info!(dir=%dir.display(), "Loading assets");
        let mut cache = HashMap::default();

        let assets: Vec<color_eyre::Result<(String, String, Bytes)>> =
            ReadDirStream::new(tokio::fs::read_dir(dir).await?)
                .map(|file| async move {
                    let file = file?;
                    let path = file.path();
                    let filename = path.file_name().and_then(|n| n.to_str());
                    let ext = path.extension().and_then(|p| p.to_str());

                    let (filename, ext) = match (filename, ext) {
                        (Some(filename), Some(ext)) => (filename, ext),
                        _ => return Ok(None),
                    };

                    let stored_path = path
                        .clone()
                        .into_os_string()
                        .into_string()
                        .map_err(|_| color_eyre::eyre::format_err!("Invalid path"))?;
                    tracing::debug!(path = %stored_path, "Loading asset");

                    let bytes = tokio::fs::read(&path).await?;

                    let contents = match ext {
                        "css" | "js" => compress_data(&bytes),
                        _ => bytes,
                    };

                    Ok(Some((
                        stored_path,
                        filename.to_string(),
                        Bytes::from(contents),
                    )))
                })
                .buffered(8)
                .filter_map(
                    |res_opt: color_eyre::Result<
                        std::option::Option<(
                            std::string::String,
                            std::string::String,
                            bytes::Bytes,
                        )>,
                    >| ready(res_opt.transpose()),
                )
                .collect::<Vec<_>>()
                .await;

        for asset_res in assets {
            let (stored_path, filename, contents) = asset_res?;
            cache.insert(
                Self::get_cache_key(&filename),
                StaticAsset {
                    path: stored_path,
                    contents,
                },
            );
        }

        for (key, asset) in &cache {
            tracing::debug!(%key, path = %asset.path, "Asset loaded");
        }
        tracing::debug!(len = cache.len(), "Loaded assets");

        Ok(Self(cache))
    }
}

/// Represents a single static asset from the build directory. Assets are
/// represented as pre-compressed bytes via Brotli and their original content
/// type so the set_content_type middleware service can set the correct
/// Content-Type header.
#[derive(Debug)]
pub struct StaticAsset {
    pub path: String,
    pub contents: Bytes,
}

impl StaticAsset {
    pub fn ext(&self) -> Option<&str> {
        let parts: Vec<&str> = self.path.split('.').collect();

        parts.last().copied()
    }
    /// Returns the content type of the asset based on its file extension.
    pub fn content_type(&self) -> Option<&'static str> {
        self.ext().and_then(|ext| {
            Some(match ext {
                "js" => "application/javascript",
                "css" => "text/css",
                _ => return None,
            })
        })
    }
}

fn compress_data(input: &[u8]) -> Vec<u8> {
    let mut bytes = vec![];

    let mut writer = brotli::CompressorWriter::new(&mut bytes, 4096, 6, 20);

    writer.write_all(input).expect("Can't fail");

    drop(writer);

    bytes
}
