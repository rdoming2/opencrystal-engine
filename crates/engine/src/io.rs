use std::fs::File;
use std::path::Path;

use serde::de::DeserializeOwned;

pub fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|err| format!("{}: {}", path.display(), err))?;
    serde_json::from_reader(file).map_err(|err| format!("{}: {}", path.display(), err))
}
