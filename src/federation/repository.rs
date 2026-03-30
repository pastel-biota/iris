use std::{collections::{HashMap, HashSet}, fs::File, path::{Path, PathBuf}};

use anyhow::Context as _;

use crate::model::Identifier;

#[derive(Clone, Debug)]
pub struct FederationRepository {
    base_dir: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct FederationIndex {
    hosts: HashMap<String, FederationState>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
struct FederationState {
    photos: HashSet<Identifier>,
}

impl FederationRepository {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn add_publishing_photo(&mut self, federating_to: &str, id: &Identifier) -> anyhow::Result<()> {
        let mut index = self.load()?;
        let state = index.hosts.entry(federating_to.to_string()).or_insert(Default::default());
        state.photos.insert(id.clone());

        self.save(&index)?;

        Ok(())
    }

    pub fn list_federated_photos(&self, federating_to: &str) -> anyhow::Result<Vec<Identifier>> {
        let index = self.load()?;
        let state = index.hosts.get(federating_to)
            .context("Unknown host")?;

        Ok(state.photos.iter().cloned().collect())
    }

    fn path(&self) -> PathBuf {
        self.base_dir.join("federation.json")
    }

    pub fn load(&self) -> anyhow::Result<FederationIndex> {
        if !self.path().exists() {
            return self.init();
        }

        let file = File::open(self.path()).context("Failed to open a file for all image index")?;

        serde_json::from_reader(file)
            .context("The all image index contains invalid content")
    }

    fn save(&self, content: &FederationIndex) -> anyhow::Result<()> {
        let mut file = File::create(self.path())
            .context("Failed to create a file for all image index")?;

        serde_json::to_writer_pretty(&mut file, content)
            .context("Failed to write a empty entry for all image index")?;

        Ok(())
    }

    fn init(&self) -> anyhow::Result<FederationIndex> {
        let mut file =
            File::create(&self.path()).context("Failed to create a file for all image index")?;

        let value = FederationIndex::default();
        serde_json::to_writer_pretty(&mut file, &value)
            .context("Failed to write a empty entry for all image index")?;

        Ok(value)
    }
}

