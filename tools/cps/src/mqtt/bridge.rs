
#[derive(Clone)]
pub struct Config {
    pub broker: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub client_id: Option<String>,
}
