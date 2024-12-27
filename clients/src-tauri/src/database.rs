use crate::{
    error::Result,
    files::{FileDescriptor, SyncStatus},
    state::User,
};
use anyhow::Context;
use log::debug;
use rusqlite::{
    types::{FromSql, FromSqlError},
    Connection, ToSql,
};
use rusqlite_migration::{Migrations, M};
use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn init(path: PathBuf) -> Result<Database> {
        debug!("Databae initialization");
        let migrations = Migrations::new(vec![
            M::up(
                "CREATE TABLE app_settings (
                    id INTEGER PRIMARY KEY,
                    key TEXT NOT NULL UNIQUE,
                    value TEXT NOT NULL
                );",
            ),
            M::up(
                "CREATE TABLE directory (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE ON CONFLICT IGNORE
                );",
            ),
            M::up(
                "CREATE TABLE file (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    uuid BLOB NOT NULL UNIQUE,
                    date TEXT NOT NULL,
                    sha256 TEXT NOT NULL,
                    key TEXT NOT NULL,
                    status TEXT NOT NULL
                );",
            ),
        ]);
        let mut conn =
            Connection::open(path.join("data.db3")).context("Failed to open db connection")?;
        debug!("Database connection opened");
        // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
        migrations
            .to_latest(&mut conn)
            .context("Failed to apply DB migrations")?;
        Ok(Database {
            connection: Mutex::new(conn),
        })
    }

    pub fn get_user(&self) -> Result<Option<User>> {
        let conn = self.get_connection();

        let mut statement = conn
            .prepare("SELECT key, value FROM app_settings")
            .context("Could not prepare statement for getting settings")?;
        let map = statement
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })
            .context("Could not get settings from DB")?
            .collect::<std::result::Result<HashMap<String, String>, _>>()
            .context("Could not map settings to result")?;
        let user_id = map.get("user_id");
        let user_name = map.get("user_name");

        let user = match (user_id, user_name) {
            (Some(id), Some(name)) => Some(User {
                id: Uuid::parse_str(id).with_context(|| format!("Couldn't parse UUID {:?}", id))?,
                name: name.clone(),
            }),
            _ => None,
        };
        Ok(user)
    }

    pub fn save_directories(&self, dirs: &[&str]) -> Result<()> {
        let mut conn = self.get_connection();

        let tx = conn
            .transaction()
            .context("Could not start DB transaction")?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO directory (path) VALUES (?1)")
                .context("Could not prepare statement for saving directories")?;
            dirs.iter()
                .map(|dir| stmt.execute([dir]))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("Could not save directories, statement failed")?;
        }
        tx.commit().context("Could not commit DB transaction")?;
        Ok(())
    }

    pub fn has_images_dirs(&self) -> Result<bool> {
        let conn = self.get_connection();

        let dirs_count = conn
            .query_row("SELECT COUNT(1) FROM directory", (), |row| {
                row.get::<usize, usize>(0)
            })
            .context("Could not read directories count")?;

        Ok(dirs_count > 0)
    }

    pub fn get_directories(&self) -> Result<Vec<String>> {
        let conn = self.get_connection();

        let mut statement = conn
            .prepare("SELECT path FROM directory")
            .context("Could not read directories")?;

        let rows = statement
            .query_map([], |row| row.get(0))
            .context("Could not map directories")?;

        let mut dirs = Vec::new();
        for row in rows {
            dirs.push(row.context("Could not map directory")?);
        }
        Ok(dirs)
    }

    pub fn index_files(&self, descriptors: &Vec<FileDescriptor>) -> Result<()> {
        let mut conn = self.get_connection();
        let tx = conn
            .transaction()
            .context("Could not start DB transaction")?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO file (path, uuid, date, sha256, key, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            ).context("Could not prepare statement for inserting descriptors")?;
            for desc in descriptors {
                stmt.execute((
                    &desc.path,
                    desc.uuid,
                    desc.date,
                    &desc.sha256,
                    &desc.key,
                    &desc.status,
                ))
                .with_context(|| format!("Could not execute insert for descriptor {:?}", desc))?;
            }
        }

        tx.commit().context("Could not commit DB transaction")?;
        Ok(())
    }

    pub fn find_files_by_sync_status(&self, status: SyncStatus) -> Result<Vec<FileDescriptor>> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare("SELECT path, uuid, date, sha256, key, status FROM file WHERE status = ?1 ORDER BY date DESC")
            .context("Could not prepare statement for finding files by status")?;
        let rows = statement
            .query_map([status], |row| {
                Ok(FileDescriptor {
                    path: row.get(0)?,
                    uuid: row.get(1)?,
                    date: row.get(2)?,
                    sha256: row.get(3)?,
                    key: row.get(4)?,
                    status: row.get(5)?,
                })
            })
            .context("Could not map indexed images to file descriptors")?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row.context("Failed mapping an item to file descriptor")?);
        }
        debug!("Got {} files from index", descriptors.len());
        Ok(descriptors)
    }

    pub fn get_indexed_images(&self) -> Result<Vec<FileDescriptor>> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare("SELECT path, uuid, date, sha256, key, status FROM file ORDER BY date DESC")
            .context("Could not get indexed images from DB")?;
        let rows = statement
            .query_map([], |row| {
                Ok(FileDescriptor {
                    path: row.get(0)?,
                    uuid: row.get(1)?,
                    date: row.get(2)?,
                    sha256: row.get(3)?,
                    key: row.get(4)?,
                    status: row.get(5)?,
                })
            })
            .context("Could not map indexed images to file descriptors")?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row.context("Failed mapping an item to file descriptor")?);
        }
        debug!("Got {} files from index", descriptors.len());
        Ok(descriptors)
    }

    fn get_connection(&self) -> MutexGuard<'_, Connection> {
        return self.connection.lock().unwrap();
    }

    pub(crate) fn save_user(&self, user: &User) -> Result<()> {
        let mut conn = self.get_connection();

        let tx = conn
            .transaction()
            .context("Could not start a DB transaction")?;

        {
            let mut statement = tx.prepare("INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2").context("Could not prepare DB statement for saving settings")?;
            statement
                .execute(("user_id", user.id.to_string()))
                .context("Could not insert user id")?;
            statement
                .execute(("user_name", user.name.clone()))
                .context("Could not insert user name")?;
        }

        tx.commit().context("Could not commit DB transaction")?;

        Ok(())
    }

    pub fn update_file_status(&self, uuid: &Uuid, status: SyncStatus) -> Result<()> {
        let conn = self.get_connection();
        conn.execute(
            "UPDATE file SET status = ?1 WHERE uuid = ?2",
            (status, uuid.as_bytes()),
        )
        .context("Failed to update file status")?;
        Ok(())
    }
}

impl ToSql for SyncStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let val: &'static str = self.into();
        Ok(val.into())
    }
}

impl FromSql for SyncStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let sync_status = SyncStatus::from_str(value.as_str()?)
            .map_err(|err| FromSqlError::Other(Box::new(err)))?;
        Ok(sync_status)
    }
}
