use std::path::PathBuf;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Config<T>
    where T: Serialize
{
    pub cfg: T,
    pub path: PathBuf
}

impl<T> Config<T>
    where T: Serialize + Default + DeserializeOwned
{
    pub fn new(path: PathBuf) -> Self {
        let cfg = confy::load_path(&path).unwrap();

        Self {
            cfg,
            path
        }
    }
    pub fn write(&self) {
        confy::store_path(&self.path, &self.cfg).unwrap();
    }
}