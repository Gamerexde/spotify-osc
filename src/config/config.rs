use std::path::PathBuf;
use confy::ConfyError;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Config<T>
    where T: Serialize
{
    pub cfg: Option<T>,
    pub path: PathBuf
}

impl<T> Config<T>
    where T: Serialize + Default + DeserializeOwned
{
    pub fn new(path: PathBuf) -> Self {
        Self {
            cfg: None,
            path
        }
    }

    pub fn reload(&mut self) -> Result<(), ConfyError> {
        let cfg = confy::load_path(&self.path)?;

        self.cfg = Some(cfg);
        Ok(())
    }
    pub fn write(&self) -> Result<(), ConfyError> {
        confy::store_path(&self.path, &self.cfg)?;

        Ok(())
    }
}