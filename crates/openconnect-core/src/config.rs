pub struct Config {
    pub username: Option<String>,
    pub password: Option<String>,
    pub server: Option<String>,
}

pub struct ConfigBuilder {
    username: Option<String>,
    password: Option<String>,
    server: Option<String>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            server: None,
        }
    }

    pub fn username(&mut self, username: &str) -> &mut Self {
        self.username = Some(username.to_string());
        self
    }

    pub fn password(&mut self, password: &str) -> &mut Self {
        self.password = Some(password.to_string());
        self
    }

    pub fn server(&mut self, server: &str) -> &mut Self {
        self.server = Some(server.to_string());
        self
    }

    pub fn build(&self) -> Config {
        Config {
            username: self.username.clone(),
            password: self.password.clone(),
            server: self.server.clone(),
        }
    }
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
