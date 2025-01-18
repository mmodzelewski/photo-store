use crate::files::{File, Metadata};
use crate::{
    error::Result,
    files::{FileDescriptor, FileStatus},
    state::User,
};
use anyhow::Context;
use log::debug;
use rusqlite::{
    types::{FromSql, FromSqlError},
    Connection, OptionalExtension, ToSql,
};
use rusqlite_migration::{Migrations, M};
use std::{
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn init(path: PathBuf) -> Result<Database> {
        debug!("Database initialization");
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
                    status TEXT NOT NULL,
                    remote BOOLEAN NOT NULL
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
        let settings_map = statement
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })
            .context("Could not get settings from DB")?
            .collect::<std::result::Result<HashMap<String, String>, _>>()
            .context("Could not map settings to result")?;
        let user_id = settings_map.get("user_id");
        let user_name = settings_map.get("user_name");

        let user = match (user_id, user_name) {
            (Some(id), Some(name)) => Some(User {
                id: Uuid::parse_str(id).with_context(|| format!("Couldn't parse UUID {:?}", id))?,
                name: name.clone(),
            }),
            _ => None,
        };
        Ok(user)
    }

    pub fn get_last_sync_time(&self) -> Result<Option<OffsetDateTime>> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare("SELECT value FROM app_settings where key = ?1")
            .context("Could not prepare statement for getting sync time")?;
        let sync_time = statement
            .query_row(["sync_time"], |row| {
                let value: OffsetDateTime = row.get(0)?;
                Ok(value)
            })
            .optional()
            .context("Could not get sync time from DB")?;
        Ok(sync_time)
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

    pub fn has_files_dirs(&self) -> Result<bool> {
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

    pub fn index_files(&self, descriptors: &Vec<FileDescriptor>, remote: bool) -> Result<()> {
        let mut conn = self.get_connection();
        let tx = conn
            .transaction()
            .context("Could not start DB transaction")?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO file (path, uuid, date, sha256, key, status, remote) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            ).context("Could not prepare statement for inserting descriptors")?;
            for desc in descriptors {
                stmt.execute((
                    &desc.path,
                    desc.uuid,
                    desc.date,
                    &desc.sha256,
                    &desc.key,
                    &desc.status,
                    remote,
                ))
                .with_context(|| format!("Could not execute insert for descriptor {:?}", desc))?;
            }
        }

        tx.commit().context("Could not commit DB transaction")?;
        Ok(())
    }

    pub fn find_files_by_sync_status(&self, status: FileStatus) -> Result<Vec<FileDescriptor>> {
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
            .context("Could not map indexed files to file descriptors")?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row.context("Failed mapping an item to file descriptor")?);
        }
        debug!("Got {} files from index", descriptors.len());
        Ok(descriptors)
    }

    pub fn file_exists(&self, uuid: &Uuid) -> Result<bool> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare("SELECT COUNT(1) FROM file WHERE uuid = ?1")
            .context("Could not prepare statement for checking file existence")?;
        let count = statement
            .query_row([uuid], |row| row.get::<usize, usize>(0))
            .context("Could not map row when checking file existence")?;
        Ok(count > 0)
    }

    pub fn get_indexed_files(&self) -> Result<Vec<FileDescriptor>> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare("SELECT path, uuid, date, sha256, key, status FROM file ORDER BY date DESC")
            .context("Could not get indexed files from DB")?;
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
            .context("Could not map indexed files to file descriptors")?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row.context("Failed mapping an item to file descriptor")?);
        }
        debug!("Got {} files from index", descriptors.len());
        Ok(descriptors)
    }

    pub fn get_file_by_id(&self, uuid: &Uuid) -> Result<File> {
        let conn = self.get_connection();
        let mut statement = conn
            .prepare(
                "SELECT remote, path, uuid, date, sha256, key, status FROM file WHERE uuid = ?1",
            )
            .context("Could not prepare statement for getting file by id")?;
        let file = statement
            .query_row([uuid], |row| {
                let remote: bool = row.get(0)?;
                debug!("File remote: {:?}", remote);
                let metadata = Metadata {
                    uuid: row.get(2)?,
                    date: row.get(3)?,
                    sha256: row.get(4)?,
                    key: row.get(5)?,
                };
                if remote {
                    Ok(File::Remote { metadata })
                } else {
                    Ok(File::Local {
                        path: row.get(1)?,
                        status: row.get(6)?,
                        metadata,
                    })
                }
            })
            .context("Could not map row to file")?;
        Ok(file)
    }

    fn get_connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap()
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

    pub(crate) fn update_last_sync_time(&self, time: &OffsetDateTime) -> Result<()> {
        let mut conn = self.get_connection();

        let tx = conn
            .transaction()
            .context("Could not start a DB transaction")?;

        {
            let mut statement = tx.prepare("INSERT INTO app_settings (key, value) VALUES ('sync_time', ?1) ON CONFLICT(key) DO UPDATE SET value=?1").context("Could not prepare DB statement for saving sync time")?;
            statement
                .execute((time,))
                .context("Could not save sync time")?;
        }

        tx.commit().context("Could not commit DB transaction")?;

        Ok(())
    }

    pub fn update_file_status(&self, uuid: &Uuid, status: FileStatus) -> Result<()> {
        let conn = self.get_connection();
        conn.execute(
            "UPDATE file SET status = ?1 WHERE uuid = ?2",
            (&status, uuid.as_bytes()),
        )
        .context(format!(
            "Failed to change file ({}) status to {:?}",
            uuid, &status
        ))?;
        Ok(())
    }
}

impl ToSql for FileStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let val: &'static str = self.into();
        Ok(val.into())
    }
}

impl FromSql for FileStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let sync_status = FileStatus::from_str(value.as_str()?)
            .map_err(|err| FromSqlError::Other(Box::new(err)))?;
        Ok(sync_status)
    }
}
