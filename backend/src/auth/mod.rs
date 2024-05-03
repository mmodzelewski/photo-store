pub(crate) mod error;
pub(crate) mod google;
mod handlers;
pub(crate) mod middleware;
mod repository;
mod routes;

pub(crate) use routes::routes;

struct AuthorizationRequest {
    state: String,
    pkce: String,
}
