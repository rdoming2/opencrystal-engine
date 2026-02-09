use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|err| format!("{}: {}", path.display(), err))?;
    serde_json::from_reader(file).map_err(|err| format!("{}: {}", path.display(), err))
}

pub fn write_json_pretty<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "{}: parent directory does not exist",
                parent.display()
            ));
        }
    }
    let data = serde_json::to_string_pretty(value)
        .map_err(|err| format!("{}: {}", path.display(), err))?;
    let mut file = File::create(path).map_err(|err| format!("{}: {}", path.display(), err))?;
    file.write_all(data.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| format!("{}: {}", path.display(), err))?;
    Ok(())
}
