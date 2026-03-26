use crate::ulid::Id;

#[derive(Clone, Debug)]
pub struct Session {
    user_id: Id,
}

impl Session {
    pub fn new(user_id: Id) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> Id {
        self.user_id
    }
}
