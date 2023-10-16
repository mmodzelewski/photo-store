use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Ctx {
    user_id: Uuid,
}

impl Ctx {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
}
