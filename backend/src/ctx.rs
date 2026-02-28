use crate::ulid::Id;

#[derive(Clone, Debug)]
pub struct Ctx {
    user_id: Id,
}

impl Ctx {
    pub fn new(user_id: Id) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> Id {
        self.user_id
    }
}
