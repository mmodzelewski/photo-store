use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::{path::PathBuf, sync::Mutex};

use crate::error::{Error, Result};

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn init(path: PathBuf) -> Result<Database> {
        let migrations = Migrations::new(vec![M::up(
            "CREATE TABLE directory (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE ON CONFLICT IGNORE
        );",
        )]);
        let mut conn = Connection::open(path.join("data.db3"))?;
        // conn.pragma_update(None, "journal_mode", &"WAL").unwrap(); // verify
        migrations.to_latest(&mut conn)?;
        return Ok(Database {
            connection: Mutex::new(conn),
        });
    }

    pub fn save_directories(self: &Self, dirs: Vec<&str>) -> Result<()> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|err| Error::Generic(err.to_string()))?;

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
}
