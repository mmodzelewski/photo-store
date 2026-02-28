use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, Postgres, Type};

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

impl Type<Postgres> for Id {
    fn type_info() -> PgTypeInfo {
        <uuid::Uuid as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <uuid::Uuid as Type<Postgres>>::compatible(ty)
    }
}

impl Encode<'_, Postgres> for Id {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let uuid = uuid::Uuid::from_bytes(self.0.to_bytes());
        <uuid::Uuid as Encode<Postgres>>::encode_by_ref(&uuid, buf)
    }
}

impl Decode<'_, Postgres> for Id {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        let uuid = <uuid::Uuid as Decode<Postgres>>::decode(value)?;
        Ok(Self(ulid::Ulid::from_bytes(*uuid.as_bytes())))
    }
}
