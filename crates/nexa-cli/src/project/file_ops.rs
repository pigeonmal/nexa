use std::{
    fs,
    io::{BufReader, Read},
    path::Path,
};

/// Writes bytes to `path` only when they differ from the file on disk.
pub(crate) fn write_bytes_if_changed(path: &Path, contents: &[u8]) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "refusing to overwrite generated file symlink: {}",
                path.display()
            ));
        }
        Ok(metadata)
            if metadata.is_file() && file_equals_bytes(path, metadata.len(), contents)? =>
        {
            return Ok(());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", path.display())),
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn files_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let left_metadata =
        fs::metadata(left).map_err(|error| format!("{}: {error}", left.display()))?;
    let right_metadata =
        fs::metadata(right).map_err(|error| format!("{}: {error}", right.display()))?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }
    let mut left = BufReader::new(
        fs::File::open(left).map_err(|error| format!("{}: {error}", left.display()))?,
    );
    let mut right = BufReader::new(
        fs::File::open(right).map_err(|error| format!("{}: {error}", right.display()))?,
    );
    let mut left_chunk = [0_u8; 16 * 1024];
    let mut right_chunk = [0_u8; 16 * 1024];
    loop {
        let left_len = left
            .read(&mut left_chunk)
            .map_err(|error| format!("could not read file: {error}"))?;
        let right_len = right
            .read(&mut right_chunk)
            .map_err(|error| format!("could not read file: {error}"))?;
        if left_len != right_len || left_chunk[..left_len] != right_chunk[..right_len] {
            return Ok(false);
        }
        if left_len == 0 {
            return Ok(true);
        }
    }
}

fn file_equals_bytes(path: &Path, file_len: u64, contents: &[u8]) -> Result<bool, String> {
    if file_len != contents.len() as u64 {
        return Ok(false);
    }
    let mut file = BufReader::new(
        fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?,
    );
    let mut chunk = [0_u8; 16 * 1024];
    let mut offset = 0;
    loop {
        let count = file
            .read(&mut chunk)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if count == 0 {
            return Ok(offset == contents.len());
        }
        if contents.get(offset..offset + count) != Some(&chunk[..count]) {
            return Ok(false);
        }
        offset += count;
    }
}
