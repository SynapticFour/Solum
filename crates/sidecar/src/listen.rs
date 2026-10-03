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
    env_flag("SOLUM_ALLOW_PLAINTEXT_HTTP")
}

/// `SOLUM_ALLOW_INTERNAL_BIND=1`. Off unless the value is `1`, `true`, or `yes`.
pub fn internal_bind_env_allowed() -> bool {
    env_flag("SOLUM_ALLOW_INTERNAL_BIND")
}

fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        Err(_) => false,
    }
}

/// Inputs for the opt-in non-loopback bind (ADR 0004).
#[derive(Clone, Copy)]
pub struct InternalBindRequest<'a> {
    pub token: &'a str,
    pub profile: &'a str,
    pub keys_dir: bool,
    pub ephemeral: bool,
    pub wrapped_keys: bool,
}

/// Fail closed. The flag is honoured only with a non-empty sidecar token, a
/// pilot profile (`eu-ehds` or `kenya-dpa`), and `--keys-dir`.
pub fn authorize_internal_bind(req: &InternalBindRequest<'_>) -> Result<(), String> {
    if req.token.trim().is_empty() {
        return Err(
            "internal bind refused: sidecar token is empty. Set SOLUM_SIDECAR_TOKEN.".into(),
        );
    }
    if req.ephemeral || req.wrapped_keys || !req.keys_dir {
        return Err(
            "internal bind refused: --keys-dir is required. --ephemeral and --wrapped-keys-dir are refused."
                .into(),
        );
    }
    if req.profile != "eu-ehds" && req.profile != "kenya-dpa" {
        return Err(format!(
            "internal bind refused: profile {} is not a pilot profile (eu-ehds or kenya-dpa)",
            req.profile
        ));
    }
    Ok(())
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

    #[test]
    fn internal_bind_accepts_pilot_keys_dir() {
        for profile in ["eu-ehds", "kenya-dpa"] {
            authorize_internal_bind(&InternalBindRequest {
                token: "sidecar-token",
                profile,
                keys_dir: true,
                ephemeral: false,
                wrapped_keys: false,
            })
            .unwrap();
        }
    }

    #[test]
    fn internal_bind_refuses_incomplete() {
        let base = InternalBindRequest {
            token: "sidecar-token",
            profile: "eu-ehds",
            keys_dir: true,
            ephemeral: false,
            wrapped_keys: false,
        };
        assert!(authorize_internal_bind(&InternalBindRequest {
            token: "  ",
            ..base
        })
        .is_err());
        assert!(authorize_internal_bind(&InternalBindRequest {
            keys_dir: false,
            ..base
        })
        .is_err());
        assert!(authorize_internal_bind(&InternalBindRequest {
            ephemeral: true,
            ..base
        })
        .is_err());
        assert!(authorize_internal_bind(&InternalBindRequest {
            wrapped_keys: true,
            ..base
        })
        .is_err());
        assert!(authorize_internal_bind(&InternalBindRequest {
            profile: "dev-local",
            ..base
        })
        .unwrap_err()
        .contains("pilot"));
    }
}
