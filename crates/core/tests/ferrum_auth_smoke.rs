//! Sprint-1 smoke: `ferrum_core::auth::AuthClaims` is constructible via the
//! git-pinned re-export from `solum-crypto`.
//!
//! No JWT/JWKS verification (Sprint 5). API inspected at pin `f28f2780…` (Ferrum v0.3.1):
//! `AuthClaims` is a public enum with `Jwt` / `Passport` variants — not serde
//! on the enum itself, so we construct fields directly.

use solum_core::crypto::ferrum_core::auth::AuthClaims;

#[test]
fn auth_claims_jwt_variant_is_constructible() {
    let claims = AuthClaims::Jwt {
        sub: "patient/42".into(),
        iss: Some("https://idp.example/oidc".into()),
        exp: 4_102_444_800,
        jti: Some("core-smoke-1".into()),
        scope: Some("patient/*.read".into()),
        raw_token: None,
    };

    assert_eq!(claims.sub(), Some("patient/42"));
    assert_eq!(claims.issuer(), Some("https://idp.example/oidc"));
    assert!(claims.has_scope("patient/*.read"));
    assert!(!claims.is_admin());
    assert_eq!(claims.jti(), Some("core-smoke-1"));
}
