use std::env;
use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            other => Err(format!("expected json or pretty, got {other}")),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub db_pool_max: u32,
    pub log_format: LogFormat,
}

fn optional<T>(name: &str, default: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Display,
{
    let raw = env::var(name).unwrap_or_else(|_| default.to_owned());
    raw.parse().map_err(|e| format!("{name}: {e}"))
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").map_err(|_| "DATABASE_URL: not set")?,
            bind_addr: optional("BIND_ADDR", "0.0.0.0:8080")?,
            db_pool_max: optional("DB_POOL_MAX", "10")?,
            log_format: optional("LOG_FORMAT", "pretty")?,
        })
    }
}
