//! Bind policy: plaintext HTTP is loopback-only unless an explicit eval override
//! is set. Terminate TLS at a reverse proxy in production.

use std::net::SocketAddr;

/// Fail-closed: the sidecar speaks HTTP on loopback.
///
/// Non-loopback binds require `allow_plaintext_off_loopback` (from
/// `SOLUM_ALLOW_PLAINTEXT_HTTP=1`). Production (`eu-ehds` / `kenya-dpa`) still
/// refuses after profile load — the override is `dev-local` / Docker eval only.
pub fn validate_listen_bind(
    bind: SocketAddr,
    allow_plaintext_off_loopback: bool,
) -> Result<(), String> {
    if bind.ip().is_loopback() {
        return Ok(());
    }
    if allow_plaintext_off_loopback {
        return Ok(());
    }
    Err(format!(
        "non-loopback bind {bind} is refused. solum-sidecar serves plaintext HTTP on loopback only. \
         Bind 127.0.0.1 (or ::1) and terminate TLS at a reverse proxy. \
         Docker eval (dev-local only) may set SOLUM_ALLOW_PLAINTEXT_HTTP=1. \
         This is deliberate: the process is not a TLS terminator."
    ))
}

pub fn plaintext_http_env_allowed() -> bool {
    match std::env::var("SOLUM_ALLOW_PLAINTEXT_HTTP") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn loopback_ok() {
        let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8787);
        assert!(validate_listen_bind(bind, false).is_ok());
    }

    #[test]
    fn non_loopback_refused_without_override() {
        let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8787);
        let err = validate_listen_bind(bind, false).unwrap_err();
        assert!(err.contains("non-loopback"), "{err}");
    }

    #[test]
    fn non_loopback_ok_with_override() {
        let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8787);
        assert!(validate_listen_bind(bind, true).is_ok());
    }
}
