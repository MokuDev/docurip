use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

pub fn zip_directory(src_dir: &Path, dst_file: &Path) -> anyhow::Result<()> {
    let file = File::create(dst_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip_directory_inner(src_dir, src_dir, &mut zip, &options)?;
    zip.finish()?;
    Ok(())
}

fn zip_directory_inner(
    base: &Path,
    current: &Path,
    zip: &mut zip::ZipWriter<File>,
    options: &SimpleFileOptions,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(base)?.to_string_lossy();
        if path.is_file() {
            zip.start_file(name, *options)?;
            let mut f = File::open(&path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            zip.add_directory(name, *options)?;
            zip_directory_inner(base, &path, zip, options)?;
        }
    }
    Ok(())
}
