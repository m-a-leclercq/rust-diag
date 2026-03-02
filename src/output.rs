use crate::collector::CollectionResult;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct OutputManager {
    pub temp_dir: TempDir,
}

impl OutputManager {
    pub fn new() -> Result<Self> {
        Ok(OutputManager {
            temp_dir: TempDir::new()?,
        })
    }

    /// Write a single collection result to the temp directory.
    pub fn write_result(&self, result: &CollectionResult) -> Result<()> {
        let ep = &result.endpoint;

        let dest_dir = if let Some(subdir) = &ep.subdir {
            self.temp_dir.path().join(subdir)
        } else {
            self.temp_dir.path().to_path_buf()
        };

        fs::create_dir_all(&dest_dir)?;

        let filename = format!("{}{}", ep.name, ep.extension);
        let file_path = dest_dir.join(filename);

        let mut f = fs::File::create(&file_path)?;
        f.write_all(result.body.as_bytes())?;

        Ok(())
    }

    /// Create a .zip archive of the temp directory.
    /// Returns the path to the created archive.
    pub fn create_archive(&self, cluster_name: &str, output_dir: &Path) -> Result<PathBuf> {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let safe_name: String = cluster_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();

        let archive_name = format!("{}-diagnostics-{}.zip", safe_name, timestamp);
        let archive_path = output_dir.join(&archive_name);

        let file = fs::File::create(&archive_path)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let base = self.temp_dir.path();
        for entry in WalkDir::new(base).min_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            // Use forward slashes inside the zip regardless of OS
            let name = path
                .strip_prefix(base)?
                .to_str()
                .ok_or("non-UTF8 path in temp dir")?
                .replace('\\', "/");

            if path.is_dir() {
                zip.add_directory(&name, options)?;
            } else {
                zip.start_file(&name, options)?;
                let mut f = fs::File::open(path)?;
                std::io::copy(&mut f, &mut zip)?;
            }
        }

        zip.finish()?;
        Ok(archive_path)
    }
}
