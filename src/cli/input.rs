use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::{Error, Result};

const MAX_INPUT_BYTES: u64 = 1_048_576;

pub(super) fn read_json_input<T: DeserializeOwned>(input: &str) -> Result<T> {
    let (bytes, label) = if input == "-" {
        (read_stdin()?, Path::new("stdin"))
    } else {
        let path = Path::new(input);
        reject_symlink_chain(path)?;
        let metadata = fs::symlink_metadata(path)
            .map_err(|source| Error::io("inspect structured input", path, source))?;
        if !metadata.is_file() {
            return Err(Error::invalid("structured input", "must be a regular file"));
        }
        if metadata.len() > MAX_INPUT_BYTES {
            return Err(input_budget());
        }
        let mut file =
            File::open(path).map_err(|source| Error::io("open structured input", path, source))?;
        (read_limited(&mut file)?, path)
    };
    serde_json::from_slice(&bytes).map_err(|_| Error::MalformedJson {
        path: label.to_path_buf(),
    })
}

fn read_stdin() -> Result<Vec<u8>> {
    read_limited(&mut io::stdin().lock())
}

fn read_limited(reader: &mut impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| Error::io("read structured input", Path::new("input"), source))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(input_budget());
    }
    Ok(bytes)
}

fn input_budget() -> Error {
    Error::Budget(format!(
        "structured input exceeds the {MAX_INPUT_BYTES} byte budget"
    ))
}

fn reject_symlink_chain(path: &Path) -> Result<()> {
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    && !is_allowed_platform_alias(component, &metadata) =>
            {
                return Err(Error::invalid(
                    "structured input",
                    format!("symlink is forbidden: {}", component.display()),
                ));
            }
            Ok(_) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(Error::io("inspect structured input", component, source)),
        }
    }
    Ok(())
}

fn is_allowed_platform_alias(_path: &Path, _metadata: &fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        _path == Path::new("/var")
            && fs::read_link(_path).is_ok_and(|target| target == Path::new("private/var"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
