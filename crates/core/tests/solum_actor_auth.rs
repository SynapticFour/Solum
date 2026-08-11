//! Sprint-2: AuthClaims → SolumActor mapping (feature `ferrum-companion` on
//! solum-identity, enabled by solum-core). Reuses the Mode-B Sprint-1 Jwt
//! fixture field values from `examples/ferrum-companion`.

use solum_core::crypto::ferrum_core::auth::{AuthClaims, PassportClaims};
use solum_core::{ActorSource, SolumActor};

#[test]
fn try_from_sprint1_jwt_fixture() {
    let claims = AuthClaims::Jwt {
        sub: "researcher@example.org".into(),
        iss: Some("https://passports.example/issuer".into()),
        exp: 4_102_444_800,
        jti: Some("smoke-jti-1".into()),
        scope: Some("drs.read ferrum:analyst".into()),
        raw_token: None,
    };

    let actor = SolumActor::try_from(&claims).expect("sub present");
    assert_eq!(actor.subject_id, "researcher@example.org");
    assert_eq!(actor.source, ActorSource::FerrumPassport);
    assert_eq!(
        actor.scopes,
        vec!["drs.read".to_string(), "ferrum:analyst".to_string()]
    );
    assert_eq!(
        actor.to_audit_string(),
        "ferrum:passport:researcher@example.org"
    );
}

#[test]
fn try_from_passport_fixture() {
    let claims = AuthClaims::Passport {
        claims: PassportClaims {
            sub: Some("researcher@example.org".into()),
            iss: Some("https://passports.example/issuer".into()),
            exp: Some(4_102_444_800),
            iat: None,
            jti: Some("passport-jti-1".into()),
            ga4gh_passport_v1: None,
            scope: Some("drs.read ferrum:analyst".into()),
            aud: None,
        },
        visas: vec![],
        raw_token: None,
    };

    let actor = SolumActor::try_from(&claims).expect("passport sub present");
    assert_eq!(actor.subject_id, "researcher@example.org");
    assert_eq!(actor.source, ActorSource::FerrumPassport);
    assert_eq!(
        actor.scopes,
        vec!["drs.read".to_string(), "ferrum:analyst".to_string()]
    );
    assert_eq!(
        actor.to_audit_string(),
        "ferrum:passport:researcher@example.org"
    );
}

#[test]
fn try_from_passport_missing_sub_errors() {
    let claims = AuthClaims::Passport {
        claims: PassportClaims {
            sub: None,
            iss: None,
            exp: None,
            iat: None,
            jti: None,
            ga4gh_passport_v1: None,
            scope: Some("drs.read".into()),
            aud: None,
        },
        visas: vec![],
        raw_token: None,
    };
    let err = SolumActor::try_from(&claims).expect_err("missing sub");
    assert!(matches!(
        err,
        solum_core::ActorFromClaimsError::MissingSubject
    ));
}

#[test]
fn from_string_preserves_legacy_actor_export() {
    let original = String::from("practitioner/7");
    let actor = SolumActor::from(original.clone());
    assert_eq!(actor.to_audit_string(), original);
}
