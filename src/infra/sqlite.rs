use std::collections::HashSet;

use anyhow::Context as _;
use diesel::{
    connection::SimpleConnection as _,
    r2d2::{ConnectionManager, CustomizeConnection, Pool},
    sqlite::Sqlite,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness as _};

use crate::repository::io::ScopedPath;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub type ConnectionPool = Pool<ConnectionManager<diesel::sqlite::SqliteConnection>>;
pub type PooledConnection =
    diesel::r2d2::PooledConnection<ConnectionManager<diesel::sqlite::SqliteConnection>>;

#[derive(Clone)]
pub struct SqliteConnection(ConnectionPool);

#[derive(Debug)]
struct ConnectionOptions;

impl CustomizeConnection<diesel::sqlite::SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(
        &self,
        conn: &mut diesel::sqlite::SqliteConnection,
    ) -> Result<(), diesel::r2d2::Error> {
        conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")
            .map_err(diesel::r2d2::Error::QueryError)
    }
}

impl SqliteConnection {
    pub async fn connect(base_dir: &ScopedPath) -> anyhow::Result<Self> {
        let file_name = base_dir.join("db.sqlite");

        tokio::task::spawn_blocking(move || Self::connect_blocking(&file_name))
            .await
            .context("DB connect task panicked")?
    }

    fn connect_blocking(file_name: &ScopedPath) -> anyhow::Result<Self> {
        let manager = ConnectionManager::<diesel::sqlite::SqliteConnection>::new(
            file_name.use_path().to_string_lossy()
        );

        let pool = Pool::builder()
            // WAL mode (enabled below via PRAGMA) allows many concurrent readers
            // alongside a single writer, so a pool of 1 would serialize every
            // request (reads included) behind one connection. SQLite itself still
            // serializes writers via file locking + busy_timeout, so this stays safe.
            .max_size(32)
            .connection_customizer(Box::new(ConnectionOptions))
            .build(manager)
            .context("Failed to prepare Sqlite database")?;

        let mut conn = pool.get().context("Failed to prepare Sqlite database")?;

        let applied_migration = conn
            .applied_migrations()
            .map_err(|error| anyhow::anyhow!("Failed to check applied migrations: {error}"))?
            .into_iter()
            .collect::<HashSet<_>>();

        let registered_migration = diesel::migration::MigrationSource::<Sqlite>::migrations(&MIGRATIONS)
            .map_err(|error| anyhow::anyhow!("Failed to list registered migrations: {error}"))?
            .into_iter()
            .map(|migration| migration.name().version().as_owned())
            .collect::<HashSet<_>>();

        let difference = applied_migration
            .difference(&registered_migration)
            .collect::<HashSet<_>>();

        if !difference.is_empty() {
            tracing::error!("{}", indoc::formatdoc! {"
                The migration is not up to date!

                The following is being applied already:
                  {:?}

                But these are expected:
                  {:?}
            ", applied_migration, registered_migration});

            anyhow::bail!("The migration is not up to date");
        }

        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|error| anyhow::anyhow!("Failed to run migrations: {error}"))?;

        tracing::debug!("{} migrations are applied!", registered_migration.len());

        Ok(SqliteConnection(pool))
    }

    pub fn pool(&self) -> ConnectionPool {
        self.0.clone()
    }
}
