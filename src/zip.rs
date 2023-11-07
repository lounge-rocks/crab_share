use fs_extra::dir;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, prelude::*};
use std::io::{Seek, Write};
use std::iter::Iterator;
use std::process::exit;
use zip::result::ZipError;
use zip::write::FileOptions;
use zip::CompressionMethod;

use std::fs::File;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub(crate) fn zip_folder(
    src_dir: &str,
    cmp_mthd: CompressionMethod,
) -> zip::result::ZipResult<Vec<u8>> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound);
    }

    let writer = std::io::Cursor::new(Vec::new());

    let dir_size = match dir::get_size(src_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error getting directory size: {}", e);
            exit(1);
        }
    };
    let progress_bar = ProgressBar::new(dir_size);
    progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap()
        .progress_chars("#>-"));

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    let res = zip_dir(
        &mut it.filter_map(|e| e.ok()),
        src_dir,
        writer,
        cmp_mthd,
        &progress_bar,
    )?;
    let inner = res.into_inner();
    println!(
        "Compression ratio: {}",
        inner.len() as f64 / dir_size as f64
    );
    Ok(inner)
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod,
    progress_bar: &ProgressBar,
) -> zip::result::ZipResult<T>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path
            .strip_prefix(Path::new(prefix))
            .expect("Path should start with prefix. This is a bug.");

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            // println!("adding file {path:?} as {name:?} ...");
            progress_bar.println(format!("adding file {path:?} as {name:?} ...",));
            // check, if file is larger than 4GB, if so, set large_file flag
            let options = if path.metadata().unwrap().len() >= 2u64.pow(32) {
                options.large_file(true)
            } else {
                options
            };
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            progress_bar.inc(buffer.len() as u64);

            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            // println!("adding dir {path:?} as {name:?} ...");
            progress_bar.println(format!("adding dir {path:?} as {name:?} ...",));
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()
}

pub fn zip_file(src_file: &str, cmp_mthd: CompressionMethod) -> zip::result::ZipResult<Vec<u8>> {
    let src_file = Path::new(src_file);
    if !src_file.is_file() {
        eprintln!("error: trying to zip a directory as a file");
        return Err(ZipError::FileNotFound);
    }

    let writer = std::io::Cursor::new(Vec::new());

    let file_size = match std::fs::metadata(src_file) {
        Ok(s) => s.len(),
        Err(e) => {
            eprintln!("error getting file size: {}", e);
            exit(1);
        }
    };
    let progress_bar = ProgressBar::new(file_size);
    progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap()
        .progress_chars("#>-"));

    let mut zip = zip::ZipWriter::new(writer);
    let mut options = FileOptions::default()
        .compression_method(cmp_mthd)
        .unix_permissions(0o755);
    if file_size >= 2u64.pow(32) {
        options = options.large_file(true);
    }

    let mut buffer = Vec::new();
    let path = Path::new("/").join(src_file.file_name().unwrap());

    // Write file or directory explicitly
    // Some unzip tools unzip files with directory paths correctly, some do not!
    println!("Reading {path:?} ...");
    #[allow(deprecated)]
    zip.start_file_from_path(&path, options)?;
    let f = File::open(src_file)?;

    io::copy(&mut progress_bar.wrap_read(f), &mut buffer)?;

    println!("Zipping {path:?} ...");
    let progress_bar = ProgressBar::new(buffer.len() as u64);
    progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap()
        .progress_chars("#>-"));
    io::copy(
        &mut progress_bar.wrap_read(&mut buffer.as_slice()),
        &mut zip,
    )?;
    buffer.clear();

    let zip = zip.finish()?;
    let inner = zip.into_inner();
    println!(
        "Compression ratio: {}",
        inner.len() as f64 / file_size as f64
    );
    Ok(inner)
}
