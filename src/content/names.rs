use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

const DEFAULT_NAMES_DB_PATH: &str = "./assets/names/names.db";
const MAX_ATTEMPTS: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameGender {
    Male,
    Female,
    Any,
}

impl NameGender {
    fn db_code(self) -> Option<&'static str> {
        match self {
            NameGender::Male => Some("M"),
            NameGender::Female => Some("F"),
            NameGender::Any => None,
        }
    }
}

#[derive(Debug)]
pub enum NameDbError {
    Db(rusqlite::Error),
    NotFound(String),
}

impl std::fmt::Display for NameDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameDbError::Db(err) => write!(f, "database error: {}", err),
            NameDbError::NotFound(table) => write!(f, "no names available in {}", table),
        }
    }
}

impl std::error::Error for NameDbError {}

pub struct NameDb {
    conn: Connection,
    max_forename_rowid: i64,
    max_surname_rowid: i64,
}

impl NameDb {
    pub fn open_default() -> Result<Self, NameDbError> {
        Self::open(Path::new(DEFAULT_NAMES_DB_PATH))
    }

    pub fn open(path: &Path) -> Result<Self, NameDbError> {
        let conn = Connection::open(path).map_err(NameDbError::Db)?;
        conn.execute("PRAGMA query_only = ON;", [])
            .map_err(NameDbError::Db)?;

        let max_forename_rowid: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(rowid), 0) FROM forenames",
                [],
                |row| row.get(0),
            )
            .map_err(NameDbError::Db)?;
        let max_surname_rowid: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(rowid), 0) FROM surnames",
                [],
                |row| row.get(0),
            )
            .map_err(NameDbError::Db)?;

        Ok(Self {
            conn,
            max_forename_rowid,
            max_surname_rowid,
        })
    }

    pub fn random_forename(
        &self,
        rng: &mut u64,
        gender: NameGender,
    ) -> Result<String, NameDbError> {
        self.random_name("forenames", self.max_forename_rowid, rng, gender)
    }

    pub fn random_surname(
        &self,
        rng: &mut u64,
        gender: NameGender,
    ) -> Result<String, NameDbError> {
        self.random_name("surnames", self.max_surname_rowid, rng, gender)
    }

    pub fn random_full_name(
        &self,
        rng: &mut u64,
        gender: NameGender,
    ) -> Result<(String, String), NameDbError> {
        let first = self.random_forename(rng, gender)?;
        let last = self.random_surname(rng, NameGender::Any)?;
        Ok((first, last))
    }

    fn random_name(
        &self,
        table: &str,
        max_rowid: i64,
        rng: &mut u64,
        gender: NameGender,
    ) -> Result<String, NameDbError> {
        if max_rowid <= 0 {
            return Err(NameDbError::NotFound(table.to_string()));
        }

        if let Some(name) = self.try_random_name(table, max_rowid, rng, gender)? {
            return Ok(name);
        }

        if gender != NameGender::Any {
            if let Some(name) = self.try_random_name(table, max_rowid, rng, NameGender::Any)? {
                return Ok(name);
            }
        }

        Err(NameDbError::NotFound(table.to_string()))
    }

    fn try_random_name(
        &self,
        table: &str,
        max_rowid: i64,
        rng: &mut u64,
        gender: NameGender,
    ) -> Result<Option<String>, NameDbError> {
        let gender_code = gender.db_code();

        for _ in 0..MAX_ATTEMPTS {
            let rowid = (next_u64(rng) % (max_rowid as u64)) as i64 + 1;
            let name = if let Some(code) = gender_code {
                self.conn
                    .query_row(
                        &format!(
                            "SELECT name FROM {} WHERE rowid >= ?1 AND gender = ?2 \
                             ORDER BY rowid LIMIT 1",
                            table
                        ),
                        (rowid, code),
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(NameDbError::Db)?
            } else {
                self.conn
                    .query_row(
                        &format!(
                            "SELECT name FROM {} WHERE rowid >= ?1 ORDER BY rowid LIMIT 1",
                            table
                        ),
                        [rowid],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(NameDbError::Db)?
            };

            if name.is_some() {
                return Ok(name);
            }
        }

        let fallback = if let Some(code) = gender_code {
            self.conn
                .query_row(
                    &format!(
                        "SELECT name FROM {} WHERE gender = ?1 ORDER BY rowid LIMIT 1",
                        table
                    ),
                    [code],
                    |row| row.get(0),
                )
                .optional()
                .map_err(NameDbError::Db)?
        } else {
            self.conn
                .query_row(
                    &format!("SELECT name FROM {} ORDER BY rowid LIMIT 1", table),
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(NameDbError::Db)?
        };

        Ok(fallback)
    }
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    *state
}
