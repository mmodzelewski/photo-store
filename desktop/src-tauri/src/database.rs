use chrono::{DateTime, Utc};
use log::debug;
use rusqlite::{types::ToSqlOutput, Connection, ToSql};
use rusqlite_migration::{Migrations, M};
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use crate::{
    error::{Error, Result},
    FileDesc,
};

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn init(path: PathBuf) -> Result<Database> {
        debug!("Databae initialization");
        let migrations = Migrations::new(vec![
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
                    date TEXT NOT NULL
                );",
            ),
        ]);
        let mut conn = Connection::open(path.join("data.db3"))?;
        debug!("Database connection opened");
        // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
        migrations.to_latest(&mut conn)?;
        return Ok(Database {
            connection: Mutex::new(conn),
        });
    }

    pub fn save_directories(self: &Self, dirs: &Vec<&str>) -> Result<()> {
        let mut conn = self.get_connection()?;

        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO directory (path) VALUES (?1)")?;
            dirs.iter()
                .map(|dir| stmt.execute([dir]))
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        tx.commit()?;
        return Ok(());
    }

    pub fn has_images_dirs(self: &Self) -> Result<bool> {
        let conn = self.get_connection()?;

        let dirs_count = conn.query_row("SELECT COUNT(1) FROM directory", (), |row| {
            row.get::<usize, usize>(0)
        })?;

        return Ok(dirs_count > 0);
    }

    pub fn get_directories(self: &Self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;

        let mut statement = conn.prepare("SELECT path FROM directory")?;

        let rows = statement.query_map([], |row| row.get(0))?;

        let mut dirs = Vec::new();
        for row in rows {
            dirs.push(row?);
        }
        return Ok(dirs);
    }

    pub fn index_files(self: &Self, paths: &Vec<FileDesc>) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare("INSERT INTO file (path, uuid, date) VALUES (?1, ?2, ?3)")?;
            for path in paths {
                stmt.execute((&path.path, path.uuid, SqlDate(path.date)))?;
            }
        }

        tx.commit()?;
        return Ok(());
    }

    pub fn get_indexed_images(self: &Self) -> Result<Vec<FileDesc>> {
        let conn = self.get_connection()?;
        let mut statement = conn.prepare("SELECT path, uuid FROM file")?;
        let rows = statement.query_map([], |row| {
            Ok(FileDesc {
                path: row.get(0)?,
                uuid: row.get(1)?,
                date: DateTime::default(),
            })
        })?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row?);
        }
        debug!("Got {} files from index", descriptors.len());
        return Ok(descriptors);
    }
    pub fn get_indexed_images_paged(self: &Self, page: usize) -> Result<Vec<FileDesc>> {
        let conn = self.get_connection()?;

        let page_size = 20usize;
        let offset = page * page_size;
        let mut statement =
            conn.prepare("SELECT path, uuid FROM file ORDER BY id LIMIT (?1), (?2)")?;
        let rows = statement.query_map([offset, page_size], |row| {
            Ok(FileDesc {
                path: row.get(0)?,
                uuid: row.get(1)?,
                date: DateTime::default(),
            })
        })?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row?);
        }
        debug!("Got {} files from index", descriptors.len());
        return Ok(descriptors);
    }

    fn get_connection(self: &Self) -> Result<MutexGuard<'_, Connection>> {
        return self
            .connection
            .lock()
            .map_err(|err| Error::Generic(err.to_string()));
    }
}

struct SqlDate(DateTime<Utc>);

impl ToSql for SqlDate {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let val = self.0.to_rfc3339();
        return Ok(ToSqlOutput::Owned(rusqlite::types::Value::Text(val)));
    }
}
