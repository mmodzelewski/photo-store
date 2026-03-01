use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(ulid::Ulid);

impl Id {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl From<ulid::Ulid> for Id {
    fn from(ulid: ulid::Ulid) -> Self {
        Self(ulid)
    }
}

impl From<Id> for ulid::Ulid {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl From<uuid::Uuid> for Id {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(ulid::Ulid::from_bytes(*uuid.as_bytes()))
    }
}

impl From<Id> for uuid::Uuid {
    fn from(id: Id) -> Self {
        uuid::Uuid::from_bytes(id.0.to_bytes())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Id {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ulid::Ulid::from_string(s).map(Self)
    }
}
