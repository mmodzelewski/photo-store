use crate::{
    error::{Error, Result},
    handlers::FileDesc,
};
use log::debug;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
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
                    date TEXT NOT NULL,
                    sha256 TEXT NOT NULL
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
        return Ok(());
    }

    pub fn has_images_dirs(&self) -> Result<bool> {
        let conn = self.get_connection()?;

        let dirs_count = conn.query_row("SELECT COUNT(1) FROM directory", (), |row| {
            row.get::<usize, usize>(0)
        })?;

        return Ok(dirs_count > 0);
    }

    pub fn get_directories(&self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;

        let mut statement = conn.prepare("SELECT path FROM directory")?;

        let rows = statement.query_map([], |row| row.get(0))?;

        let mut dirs = Vec::new();
        for row in rows {
            dirs.push(row?);
        }
        return Ok(dirs);
    }

    pub fn index_files(&self, descriptors: &Vec<FileDesc>) -> Result<()> {
        let mut conn = self.get_connection()?;
        let tx = conn.transaction()?;

        {
            let mut stmt =
                tx.prepare("INSERT INTO file (path, uuid, date, sha256) VALUES (?1, ?2, ?3, ?4)")?;
            for desc in descriptors {
                stmt.execute((&desc.path, desc.uuid, desc.date, &desc.sha256))?;
            }
        }

        tx.commit()?;
        return Ok(());
    }

    pub fn get_indexed_images(&self) -> Result<Vec<FileDesc>> {
        let conn = self.get_connection()?;
        let mut statement =
            conn.prepare("SELECT path, uuid, date, sha256 FROM file ORDER BY date DESC")?;
        let rows = statement.query_map([], |row| {
            Ok(FileDesc {
                path: row.get(0)?,
                uuid: row.get(1)?,
                date: row.get(2)?,
                sha256: row.get(3)?,
            })
        })?;
        let mut descriptors = Vec::new();
        for row in rows {
            descriptors.push(row?);
        }
        debug!("Got {} files from index", descriptors.len());
        return Ok(descriptors);
    }

    fn get_connection(&self) -> Result<MutexGuard<'_, Connection>> {
        return self
            .connection
            .lock()
            .map_err(|err| Error::Generic(err.to_string()));
    }
}
