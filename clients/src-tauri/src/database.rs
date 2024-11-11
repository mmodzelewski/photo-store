use crate::{
    error::{Error, Result},
    files::{FileDescriptor, SyncStatus},
    state::User,
};
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
        let mut conn = Connection::open(path.join("data.db3"))?;
        debug!("Database connection opened");
        // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
        migrations.to_latest(&mut conn)?;
        Ok(Database {
            connection: Mutex::new(conn),
        })
    }

    pub fn get_user(&self) -> Result<Option<User>> {
        let conn = self.get_connection()?;

        let mut statement = conn.prepare("SELECT key, value FROM app_settings")?;
        let map = statement
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })?
            .collect::<std::result::Result<HashMap<String, String>, _>>()?;
        let user_id = map.get("user_id");
        let user_name = map.get("user_name");

        let user = match (user_id, user_name) {
            (Some(id), Some(name)) => Some(User {
                id: Uuid::parse_str(id).map_err(|e| Error::Generic(e.to_string()))?,
                name: name.clone(),
            }),
            _ => None,
        };
        Ok(user)
    }

    pub fn save_directories(&self, dirs: &[&str]) -> Result<()> {
        let mut conn = self.get_connection()?;

        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO directory (path) VALUES (?1)")?;
            dirs.iter()
                .map(|dir| stmt.execute([dir]))
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn has_images_dirs(&self) -> Result<bool> {
        let conn = self.get_connection()?;

        let dirs_count = conn.query_row("SELECT COUNT(1) FROM directory", (), |row| {
            row.get::<usize, usize>(0)
        })?;

        Ok(dirs_count > 0)
    }

    pub fn get_directories(&self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;

        let mut statement = conn.prepare("SELECT path FROM directory")?;

        let rows = statement.query_map([], |row| row.get(0))?;

        let mut dirs = Vec::new();
        for row in rows {
            dirs.push(row?);
        }
        Ok(dirs)
    }

    pub fn index_files(&self, descriptors: &Vec<FileDescriptor>) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO file (path, uuid, date, sha256, key, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for desc in descriptors {
                stmt.execute((
                    &desc.path,
                    desc.uuid,
                    desc.date,
                    &desc.sha256,
                    &desc.key,
                    &desc.status,
                ))?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_indexed_images(&self) -> Result<Vec<FileDescriptor>> {
        let conn = self.get_connection()?;
        let mut statement = conn
            .prepare("SELECT path, uuid, date, sha256, key, status FROM file ORDER BY date DESC")?;
        let rows = statement.query_map([], |row| {
            Ok(FileDescriptor {
                path: row.get(0)?,
                uuid: row.get(1)?,
                date: row.get(2)?,
                sha256: row.get(3)?,
                key: row.get(4)?,
                status: row.get(5)?,
            })
        })?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row?);
        }
        debug!("Got {} files from index", descriptors.len());
        Ok(descriptors)
    }

    fn get_connection(&self) -> Result<MutexGuard<'_, Connection>> {
        return self
            .connection
            .lock()
            .map_err(|err| Error::Generic(err.to_string()));
    }

    pub(crate) fn save_user(&self, user: &User) -> Result<()> {
        let mut conn = self.get_connection()?;

        let tx = conn.transaction()?;

        {
            let mut statement = tx.prepare("INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2")?;
            statement.execute(("user_id", user.id.to_string()))?;
            statement.execute(("user_name", user.name.clone()))?;
        }

        tx.commit()?;

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
