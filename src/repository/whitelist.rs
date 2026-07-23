use anyhow::Context;
use diesel::{BoolExpressionMethods as _, ExpressionMethods as _, QueryDsl as _, RunQueryDsl, dsl};

use crate::{infra::{schema::*, sqlite::SqliteConnection}, model::{EntityName, Identifier}};

#[derive(thiserror::Error, Debug)]
pub enum WhitelistRepositoryError {
    #[error("The entity is not registered")]
    NotReigstered,

    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

pub struct WhitelistRepository {
    db: SqliteConnection,
}

impl WhitelistRepository {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn photo_allowed(&self, name: &EntityName, photo_id: &Identifier) -> Result<bool, WhitelistRepositoryError> {
        let pool = self.db.pool();
        let name = name.clone();
        let photo_id = photo_id.clone();

        let exists = tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;
            let mut query = whitelist::table.into_boxed();

            diesel::select(dsl::exists(
                whitelist::table.filter(
                    whitelist::entity.eq(name.to_string())
                        .and(whitelist::photo_id.eq(photo_id.to_string()))
                )
            ))
                .get_result(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(exists)
    }
}

mod old_json_impl {
    use anyhow::Context as _;
    use super::WhitelistRepositoryError;
    
    use crate::{model::{EntityName, Whitelist}, repository::io::ScopedPath};

    pub struct WhitelistRepository {
        base_dir: ScopedPath,
    }
    
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    #[serde(tag = "_v", rename_all = "lowercase")]
    pub enum WhitelistEntry {
        V1(WhitelistEntryV1)
    }
    
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    pub struct WhitelistEntryV1 {
        list: Whitelist,
    }
    
    
    impl WhitelistRepository {
        pub fn new(base_dir: &ScopedPath) -> Self {
            Self {
                base_dir: base_dir.clone(),
            }
        }
    
        pub fn new_entry(&self, name: &EntityName) -> anyhow::Result<Whitelist> {
            let whitelist = Whitelist::new_selective();
            let entry = WhitelistEntry::V1(
                WhitelistEntryV1 {
                    list: whitelist.clone(),
                }
            );
    
            self.save(name, &entry)?;
    
            Ok(whitelist)
        }
    
        pub fn get_whitelist(&self, name: &EntityName) -> Result<Whitelist, WhitelistRepositoryError> {
            match self.load(name) {
                Ok(Some(WhitelistEntry::V1(entry))) => Ok(entry.list),
                Ok(None) => Ok(self.new_entry(name)?),
                Err(err) => Err(err.into()),
            }
        }
    
        pub fn update_whitelist(&self, name: &EntityName, whitelist: &Whitelist) -> anyhow::Result<()> {
            let entry = WhitelistEntry::V1(
                WhitelistEntryV1 {
                    list: whitelist.clone(),
                }
            );
    
            self.save(name, &entry)
        }
    
        fn load(&self, name: &EntityName) -> anyhow::Result<Option<WhitelistEntry>> {
            let file = self.path_for(name);
            if !file.exists() {
                return Ok(None);
            }
    
            let bytes = file
                .read_binary()
                .context("Failed to open a file for all image index")?;
    
            serde_json::from_slice(&bytes)
                .context("The all image index contains invalid content")
        }
    
        fn save(&self, name: &EntityName, content: &WhitelistEntry) -> anyhow::Result<()> {
            let bytes = serde_json::to_vec_pretty(content)
                .with_context(|| format!("Failed to serialize the whitelist for {}", &**name))?;
    
            let path = self.path_for(name);
            path.parent().and_then(|dir| dir.create_dir_all().ok());
    
            path
                .write(bytes)
                .with_context(|| format!("Failed to write the whitelist for {}", &**name))
        }
    
        fn path_for(&self, name: &EntityName) -> ScopedPath {
            self.base_dir.join("whitelists").join(format!("{name}.json"))
        }
    }
}
