//! JWT/JWKS verification for Solum (Sprint 5).
//!
//! # Why `solum-auth-verify` (not `solum-identity`)
//!
//! [`solum_identity`] owns the slim in-memory actor adapter ([`SolumActor`] /
//! [`ActorSource`]) and the additive [`TryFrom`]`<&AuthClaims>` mapping from
//! Sprint 2. That crate must stay free of network and crypto stacks so audit
//! and consent can keep depending on a data-only identity type.
//!
//! Verification is a different dependency class: `jsonwebtoken` (RSA/EC),
//! optional `reqwest` JWKS fetch, and algorithm/issuer/audience policy. Putting
//! it in `solum-identity` would drag those crates into every Mode-A caller that
//! only needs `SolumActor::from("practitioner/7")`. A dedicated crate mirrors
//! the Sprint-2 identity-vs-audit boundary: mapping stays light; crypto-heavy
//! verification lives next door and depends *on* identity for the actor handoff.
//!
//! # Independence from ferrum-core (baseline note)
//!
//! This is a **standalone** implementation. It does **not** call private
//! ferrum-core helpers (`decode_jwt_or_passport`, `decode_passport_jwt`,
//! middleware). It only uses publicly documented JWT/JWKS practices and maps
//! verified claims into [`SolumActor`] via [`ActorSource`] chosen by the caller
//! (preset), never by auto-detecting the token.
//!
//! Behaviour intentionally **mirrors** Ferrum's Passport/JWKS path at pin
//! `6444469a…` (Ferrum v0.3.0) for the knobs Sprint 5 cares about (RS256+ES256, `validate_exp`,
//! no audience check, default jsonwebtoken leeway, no `nbf`), but can **drift**
//! if Ferrum changes private decode logic — there is no public compare API.
//!
//! Explicitly **not** replicated (accepted deviation):
//! - Ferrum A07 `max_token_age_hours` / `iat` max-age check (default 24h)
//! - Ferrum NTP clock skew probe (`DEFAULT_MAX_SKEW_SECS` = 300; not JWT decode)

#![forbid(unsafe_code)]

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::Deserialize;
use solum_identity::{ActorSource, SolumActor};
use thiserror::Error;

pub use jsonwebtoken::Algorithm;

/// Configuration for [`JwksVerifier::verify`].
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub allowed_algorithms: Vec<Algorithm>,
    pub validate_exp: bool,
    /// `None` → do not validate `aud` (Ferrum Passport style).
    /// `Some(aud)` → require this audience (Standalone OIDC).
    pub validate_aud: Option<String>,
    pub expected_issuer: Option<String>,
    /// Set by presets; caller-chosen auth world for [`VerifiedClaims::into_solum_actor`].
    pub actor_source: ActorSource,
}

impl VerifyConfig {
    /// Mirrors researched Ferrum Passport/JWKS verify knobs at pin `6444469a…` (Ferrum v0.3.0):
    /// RS256+ES256, `validate_exp`, no audience check, no `nbf` (jsonwebtoken default).
    pub fn for_ferrum_passport() -> Self {
        Self {
            allowed_algorithms: vec![Algorithm::RS256, Algorithm::ES256],
            validate_exp: true,
            validate_aud: None,
            expected_issuer: None,
            actor_source: ActorSource::FerrumPassport,
        }
    }

    /// Standalone / SMART-on-FHIR-shaped OIDC: issuer + audience required.
    pub fn for_standalone_oidc(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            allowed_algorithms: vec![Algorithm::RS256, Algorithm::ES256],
            validate_exp: true,
            validate_aud: Some(audience.into()),
            expected_issuer: Some(issuer.into()),
            actor_source: ActorSource::Standalone,
        }
    }
}

/// Successfully verified access-token claims (Solum-owned; not Ferrum `AuthClaims`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    pub subject: String,
    pub issuer: Option<String>,
    pub exp: Option<i64>,
    /// Whitespace-split OAuth `scope` claim (unchanged Sprint-5 behaviour).
    pub scopes: Vec<String>,
    /// Top-level `groups` claim when present (string or string array).
    pub groups: Vec<String>,
    /// Full JWT claims object for org-IAM `claim_path` extraction.
    pub claims: serde_json::Map<String, serde_json::Value>,
    pub actor_source: ActorSource,
}

impl VerifiedClaims {
    /// Build a [`SolumActor`] using the preset-chosen [`ActorSource`].
    ///
    /// Scopes come from the JWT `scope` claim only (not org-IAM group mapping).
    pub fn into_solum_actor(self) -> SolumActor {
        SolumActor {
            subject_id: self.subject,
            display: None,
            source: self.actor_source,
            scopes: self.scopes,
        }
    }

    /// Claim values at `path` (ADS-style), for org-IAM CAP mapping.
    pub fn claim_values(&self, path: &str) -> Vec<String> {
        solum_identity::claim_values_from_map(&self.claims, path)
    }
}

/// JWKS-backed JWT verifier (in-memory key set + policy).
#[derive(Debug, Clone)]
pub struct JwksVerifier {
    jwks: JwkSet,
    config: VerifyConfig,
}

impl JwksVerifier {
    /// Offline / test-friendly — mirrors Ferrum's `jwks_file` pattern (load once).
    pub fn from_jwks_json(jwks_json: &str, config: VerifyConfig) -> Result<Self, AuthVerifyError> {
        let jwks: JwkSet = serde_json::from_str(jwks_json)
            .map_err(|e| AuthVerifyError::InvalidJwks(e.to_string()))?;
        if jwks.keys.is_empty() {
            return Err(AuthVerifyError::InvalidJwks("JWKS contains no keys".into()));
        }
        Ok(Self { jwks, config })
    }

    /// Production — fetch JWKS over HTTP and retain it for subsequent [`Self::verify`].
    ///
    /// Requires feature `http`. No TTL refresh in this sprint: the set is cached
    /// for the lifetime of `Self` (caller constructs a new verifier to rotate).
    #[cfg(feature = "http")]
    pub async fn from_url(url: &str, config: VerifyConfig) -> Result<Self, AuthVerifyError> {
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| AuthVerifyError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AuthVerifyError::Http(format!(
                "JWKS fetch HTTP {}",
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| AuthVerifyError::Http(e.to_string()))?;
        Self::from_jwks_json(&body, config)
    }

    /// Verify a compact JWT against the loaded JWKS and [`VerifyConfig`].
    pub fn verify(&self, token: &str) -> Result<VerifiedClaims, AuthVerifyError> {
        let header =
            decode_header(token).map_err(|e| AuthVerifyError::InvalidToken(e.to_string()))?;
        if !self.config.allowed_algorithms.contains(&header.alg) {
            return Err(AuthVerifyError::InvalidAlgorithm(format!(
                "{:?}",
                header.alg
            )));
        }

        let kid = header
            .kid
            .as_deref()
            .ok_or_else(|| AuthVerifyError::UnknownKid("token header missing kid".into()))?;
        let jwk = self
            .jwks
            .find(kid)
            .ok_or_else(|| AuthVerifyError::UnknownKid(kid.to_string()))?;
        let key =
            DecodingKey::from_jwk(jwk).map_err(|e| AuthVerifyError::InvalidJwks(e.to_string()))?;

        // Policy may allow RS256+ES256, but jsonwebtoken 10.x rejects a Validation
        // whose `algorithms` list mixes key families (RSA vs EC). Pin to the token
        // header algorithm after the allow-list check above.
        let mut validation = Validation::new(header.alg);
        validation.algorithms = vec![header.alg];
        validation.validate_exp = self.config.validate_exp;
        // jsonwebtoken default: validate_nbf = false, leeway = 60s — leave defaults.
        validation.validate_nbf = false;

        match &self.config.validate_aud {
            Some(aud) => {
                validation.validate_aud = true;
                validation.set_audience(&[aud]);
                // Without this, missing `aud` is silently accepted when validate_aud=true.
                validation.required_spec_claims.insert("aud".to_string());
            }
            None => {
                validation.validate_aud = false;
            }
        }

        if let Some(iss) = &self.config.expected_issuer {
            validation.set_issuer(&[iss.as_str()]);
            validation.required_spec_claims.insert("iss".to_string());
        } else {
            validation.iss = None;
        }

        let data = decode::<RawClaims>(token, &key, &validation).map_err(map_decode_error)?;
        let subject = data
            .claims
            .sub
            .filter(|s| !s.is_empty())
            .ok_or(AuthVerifyError::MissingSubject)?;
        let scopes = data
            .claims
            .scope
            .as_deref()
            .map(|s| {
                s.split_whitespace()
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let mut claims_map = data.claims.rest;
        claims_map
            .entry("sub".to_string())
            .or_insert_with(|| serde_json::Value::String(subject.clone()));
        if let Some(ref iss) = data.claims.iss {
            claims_map
                .entry("iss".to_string())
                .or_insert_with(|| serde_json::Value::String(iss.clone()));
        }
        if let Some(exp) = data.claims.exp {
            claims_map
                .entry("exp".to_string())
                .or_insert_with(|| serde_json::json!(exp));
        }
        if let Some(ref scope) = data.claims.scope {
            claims_map
                .entry("scope".to_string())
                .or_insert_with(|| serde_json::Value::String(scope.clone()));
        }
        if let Some(ref aud) = data.claims.aud {
            claims_map
                .entry("aud".to_string())
                .or_insert_with(|| aud.clone());
        }

        let claims_value = serde_json::Value::Object(claims_map.clone());
        let groups = solum_identity::claim_values_from_json(&claims_value, "groups");

        Ok(VerifiedClaims {
            subject,
            issuer: data.claims.iss,
            exp: data.claims.exp,
            scopes,
            groups,
            claims: claims_map,
            actor_source: self.config.actor_source.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: Option<String>,
    iss: Option<String>,
    exp: Option<i64>,
    #[serde(default)]
    scope: Option<String>,
    /// Present for serde completeness; audience is enforced by [`Validation`].
    #[serde(default)]
    #[allow(dead_code)]
    aud: Option<serde_json::Value>,
    /// Remaining claims (`groups`, nested objects, custom IdP fields).
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

fn map_decode_error(err: jsonwebtoken::errors::Error) -> AuthVerifyError {
    use jsonwebtoken::errors::ErrorKind;
    match err.kind() {
        ErrorKind::ExpiredSignature => AuthVerifyError::Expired,
        ErrorKind::InvalidSignature => AuthVerifyError::InvalidSignature,
        ErrorKind::InvalidAudience => AuthVerifyError::InvalidAudience,
        ErrorKind::InvalidIssuer => AuthVerifyError::InvalidIssuer,
        ErrorKind::MissingRequiredClaim(claim) if claim == "aud" => {
            AuthVerifyError::InvalidAudience
        }
        ErrorKind::MissingRequiredClaim(claim) if claim == "iss" => AuthVerifyError::InvalidIssuer,
        other => AuthVerifyError::InvalidToken(format!("{other:?}")),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthVerifyError {
    #[error("invalid JWKS: {0}")]
    InvalidJwks(String),
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("algorithm not allowed: {0}")]
    InvalidAlgorithm(String),
    #[error("unknown or missing kid: {0}")]
    UnknownKid(String),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("token expired")]
    Expired,
    #[error("invalid audience")]
    InvalidAudience,
    #[error("invalid issuer")]
    InvalidIssuer,
    #[error("token missing subject (sub)")]
    MissingSubject,
    #[cfg(feature = "http")]
    #[error("JWKS HTTP fetch failed: {0}")]
    Http(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use rand::rngs::OsRng;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestKey {
        encoding: EncodingKey,
        kid: String,
        jwks_json: String,
        /// Second keypair JWKS-only (for unknown-kid cases we mint with primary).
        #[allow(dead_code)]
        n: String,
        #[allow(dead_code)]
        e: String,
    }

    fn mint_rsa_material() -> TestKey {
        let mut rng = OsRng;
        let private = RsaPrivateKey::new(&mut rng, 2048).expect("rsa key");
        let public = RsaPublicKey::from(&private);
        let pem = private
            .to_pkcs8_pem(LineEnding::LF)
            .expect("pem")
            .to_string();
        let encoding = EncodingKey::from_rsa_pem(pem.as_bytes()).expect("encoding key");

        let n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
        let kid = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(public.n().to_bytes_be()));

        let jwks_json = json!({
            "keys": [{
                "kty": "RSA",
                "kid": kid,
                "use": "sig",
                "alg": "RS256",
                "n": n,
                "e": e,
            }]
        })
        .to_string();

        TestKey {
            encoding,
            kid,
            jwks_json,
            n,
            e,
        }
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs() as i64
    }

    fn sign_rs256(key: &TestKey, claims: serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(key.kid.clone());
        encode(&header, &claims, &key.encoding).expect("sign")
    }

    #[test]
    fn ferrum_passport_round_trip_accepts_token_without_aud() {
        let key = mint_rsa_material();
        let t = now_secs();
        let token = sign_rs256(
            &key,
            json!({
                "sub": "researcher@example.org",
                "iss": "https://passports.example/issuer",
                "iat": t,
                "exp": t + 3600,
                "scope": "drs.read ferrum:analyst",
            }),
        );

        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .expect("jwks");
        let claims = verifier.verify(&token).expect("verify");
        assert_eq!(claims.subject, "researcher@example.org");
        assert_eq!(claims.actor_source, ActorSource::FerrumPassport);
        assert_eq!(
            claims.scopes,
            vec!["drs.read".to_string(), "ferrum:analyst".to_string()]
        );

        let actor = claims.into_solum_actor();
        assert_eq!(
            actor.to_audit_string(),
            "ferrum:passport:researcher@example.org"
        );
    }

    #[test]
    fn extracts_groups_claim_array() {
        let key = mint_rsa_material();
        let t = now_secs();
        let token = sign_rs256(
            &key,
            json!({
                "sub": "alice",
                "exp": t + 3600,
                "groups": ["solum-consent-ops", "staff"],
            }),
        );
        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        let claims = verifier.verify(&token).expect("verify");
        assert_eq!(
            claims.groups,
            vec!["solum-consent-ops".to_string(), "staff".to_string()]
        );
        assert_eq!(
            claims.claim_values("groups"),
            vec!["solum-consent-ops".to_string(), "staff".to_string()]
        );
    }

    #[test]
    fn extracts_dotted_realm_access_roles() {
        let key = mint_rsa_material();
        let t = now_secs();
        let token = sign_rs256(
            &key,
            json!({
                "sub": "bob",
                "exp": t + 3600,
                "realm_access": { "roles": ["researcher", "admin"] },
            }),
        );
        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        let claims = verifier.verify(&token).expect("verify");
        assert_eq!(
            claims.claim_values("realm_access.roles"),
            vec!["researcher".to_string(), "admin".to_string()]
        );
    }

    #[test]
    fn rejects_tampered_signature() {
        let key = mint_rsa_material();
        let t = now_secs();
        let token = sign_rs256(
            &key,
            json!({
                "sub": "alice",
                "exp": t + 3600,
            }),
        );
        // Flip the last character of the signature segment.
        let (head, sig) = token.rsplit_once('.').expect("compact jwt");
        let mut sig_bytes = sig.as_bytes().to_vec();
        let last = sig_bytes.last_mut().unwrap();
        *last = if *last == b'A' { b'B' } else { b'A' };
        let token = format!("{head}.{}", String::from_utf8(sig_bytes).unwrap());

        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        let err = verifier.verify(&token).expect_err("must fail");
        assert!(
            matches!(
                err,
                AuthVerifyError::InvalidSignature | AuthVerifyError::InvalidToken(_)
            ),
            "unexpected err: {err:?}"
        );
    }

    #[test]
    fn rejects_expired_token_beyond_default_leeway() {
        let key = mint_rsa_material();
        let t = now_secs();
        // More than jsonwebtoken's default 60s leeway in the past.
        let token = sign_rs256(
            &key,
            json!({
                "sub": "alice",
                "iat": t - 10_000,
                "exp": t - 120,
            }),
        );

        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        assert_eq!(
            verifier.verify(&token).unwrap_err(),
            AuthVerifyError::Expired
        );
    }

    #[test]
    fn rejects_unknown_kid() {
        let key = mint_rsa_material();
        let t = now_secs();
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("kid-not-in-jwks".into());
        let token = encode(
            &header,
            &json!({
                "sub": "alice",
                "exp": t + 3600,
            }),
            &key.encoding,
        )
        .unwrap();

        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        assert!(matches!(
            verifier.verify(&token).unwrap_err(),
            AuthVerifyError::UnknownKid(_)
        ));
    }

    #[test]
    fn rejects_disallowed_algorithm() {
        let key = mint_rsa_material();
        let t = now_secs();
        // HS256 is outside for_ferrum_passport()'s RS256+ES256 allow-list.
        // Algorithm check runs before kid/JWKS lookup, so the RSA JWKS is unused.
        let token = encode(
            &Header::new(Algorithm::HS256),
            &json!({
                "sub": "alice",
                "exp": t + 3600,
            }),
            &EncodingKey::from_secret(b"test-hmac-secret"),
        )
        .unwrap();

        let verifier =
            JwksVerifier::from_jwks_json(&key.jwks_json, VerifyConfig::for_ferrum_passport())
                .unwrap();
        assert!(matches!(
            verifier.verify(&token).unwrap_err(),
            AuthVerifyError::InvalidAlgorithm(_)
        ));
    }

    #[test]
    fn standalone_oidc_rejects_missing_or_wrong_aud() {
        let key = mint_rsa_material();
        let t = now_secs();
        let cfg = VerifyConfig::for_standalone_oidc("https://idp.example/oidc", "solum-api");
        let verifier = JwksVerifier::from_jwks_json(&key.jwks_json, cfg).unwrap();

        let no_aud = sign_rs256(
            &key,
            json!({
                "sub": "practitioner/7",
                "iss": "https://idp.example/oidc",
                "exp": t + 3600,
                "scope": "patient/*.read",
            }),
        );
        assert!(matches!(
            verifier.verify(&no_aud).unwrap_err(),
            AuthVerifyError::InvalidAudience | AuthVerifyError::InvalidToken(_)
        ));

        let wrong_aud = sign_rs256(
            &key,
            json!({
                "sub": "practitioner/7",
                "iss": "https://idp.example/oidc",
                "aud": "other-client",
                "exp": t + 3600,
            }),
        );
        assert!(matches!(
            verifier.verify(&wrong_aud).unwrap_err(),
            AuthVerifyError::InvalidAudience | AuthVerifyError::InvalidToken(_)
        ));

        let ok = sign_rs256(
            &key,
            json!({
                "sub": "practitioner/7",
                "iss": "https://idp.example/oidc",
                "aud": "solum-api",
                "exp": t + 3600,
                "scope": "patient/*.read launch/patient",
            }),
        );
        let claims = verifier.verify(&ok).expect("valid standalone token");
        assert_eq!(claims.actor_source, ActorSource::Standalone);
        assert_eq!(
            claims.into_solum_actor().to_audit_string(),
            "standalone:practitioner/7"
        );
    }
}
