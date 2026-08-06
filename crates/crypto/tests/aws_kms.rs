//! Feature-gated AWS KMS provider tests (`required-features = ["aws-kms"]`).
//!
//! Kept as an integration test so Cargo skips this target (and its AWS SDK
//! resolution) unless `--features aws-kms` is set — default CI stays on MSRV
//! 1.91.1 without pulling aws-sdk-kms 1.94+ crates.

use solum_crypto::aws_kms::aws_sdk_kms;
use solum_crypto::aws_kms::aws_sdk_kms::operation::decrypt::DecryptOutput;
use solum_crypto::aws_kms::aws_sdk_kms::operation::encrypt::EncryptOutput;
use solum_crypto::aws_kms::aws_sdk_kms::primitives::Blob;
use solum_crypto::aws_kms::aws_sdk_kms::Client;
use solum_crypto::aws_kms::aws_smithy_mocks::{mock, mock_client, RuleMode};
use solum_crypto::aws_kms::AwsKmsKeyProvider;
use solum_crypto::{
    decrypt_field, encrypt_field, Crypt4ghKeyProvider, CryptoError, CustomerHeldKeyProvider,
    EphemeralTestKeyProvider, FieldCategoryGate, KeyRef,
};

fn categories() -> Vec<String> {
    vec!["patient_summary".into()]
}

#[tokio::test]
async fn wrap_then_from_wrapped_matches_customer_held_round_trip() {
    let mut ephemeral = EphemeralTestKeyProvider::new();
    let (pubkey, privkey) = ephemeral
        .generate_test_keypair(KeyRef::new("eph-gen"))
        .expect("ephemeral keygen");
    let seed = privkey[..32].to_vec();
    let key_ref = KeyRef::new("kms/slot-1");
    let kms_key_id = "arn:aws:kms:eu-central-1:111122223333:key/test";
    let wrapped_marker = b"mock-kms-ciphertext-for-crypt4gh-seed".to_vec();

    let encrypt_rule = mock!(Client::encrypt)
        .match_requests({
            let seed = seed.clone();
            let kms_key_id = kms_key_id.to_string();
            move |req| {
                req.key_id() == Some(kms_key_id.as_str())
                    && req.plaintext().map(|b| b.as_ref()) == Some(seed.as_slice())
            }
        })
        .then_output({
            let wrapped = wrapped_marker.clone();
            let kms_key_id = kms_key_id.to_string();
            move || {
                EncryptOutput::builder()
                    .ciphertext_blob(Blob::new(wrapped.clone()))
                    .key_id(kms_key_id.clone())
                    .build()
            }
        });

    let decrypt_rule = mock!(Client::decrypt)
        .match_requests({
            let wrapped = wrapped_marker.clone();
            move |req| req.ciphertext_blob().map(|b| b.as_ref()) == Some(wrapped.as_slice())
        })
        .then_output({
            let seed = seed.clone();
            let kms_key_id = kms_key_id.to_string();
            move || {
                DecryptOutput::builder()
                    .plaintext(Blob::new(seed.clone()))
                    .key_id(kms_key_id.clone())
                    .build()
            }
        });

    let client = mock_client!(
        aws_sdk_kms,
        RuleMode::MatchAny,
        [&encrypt_rule, &decrypt_rule]
    );

    let wrapped = AwsKmsKeyProvider::wrap_seed(&client, kms_key_id, &seed)
        .await
        .expect("wrap_seed");
    assert_eq!(wrapped, wrapped_marker);
    assert_eq!(encrypt_rule.num_calls(), 1);

    let kms_provider = AwsKmsKeyProvider::from_wrapped_seed(&client, key_ref.clone(), &wrapped)
        .await
        .expect("from_wrapped_seed");
    assert_eq!(decrypt_rule.num_calls(), 1);

    let mut customer = CustomerHeldKeyProvider::new();
    customer
        .register_customer_keypair(key_ref.clone(), pubkey, seed.clone())
        .unwrap();

    let cats = categories();
    let gate = FieldCategoryGate::new(&cats);
    let plain = b"patient-summary-kms-demo";
    let enc_kms = encrypt_field(&gate, &kms_provider, "patient_summary", plain, &key_ref)
        .expect("kms encrypt");
    let out_kms = decrypt_field(&kms_provider, &enc_kms, &key_ref).expect("kms decrypt");
    assert_eq!(out_kms, plain);

    let enc_cust = encrypt_field(&gate, &customer, "patient_summary", plain, &key_ref)
        .expect("customer encrypt");
    let out_cust = decrypt_field(&customer, &enc_cust, &key_ref).expect("customer decrypt");
    assert_eq!(out_cust, plain);

    // Same seed material: ciphertext for the customer pubkey decrypts via KMS provider.
    let cross = decrypt_field(&kms_provider, &enc_cust, &key_ref).expect("cross decrypt");
    assert_eq!(cross, plain);
}

#[tokio::test]
async fn wrap_seed_rejects_short_seed() {
    let encrypt_rule = mock!(Client::encrypt).then_output(|| {
        EncryptOutput::builder()
            .ciphertext_blob(Blob::new(b"x".as_slice()))
            .build()
    });
    let client = mock_client!(aws_sdk_kms, [&encrypt_rule]);
    let err = AwsKmsKeyProvider::wrap_seed(&client, "alias/test", &[1u8; 16])
        .await
        .expect_err("short seed");
    assert!(matches!(err, CryptoError::Provider(_)));
    assert_eq!(encrypt_rule.num_calls(), 0);
}

#[tokio::test]
async fn load_aws_kms_from_dir_unwraps_wrapped_seed_file() {
    use solum_crypto::aws_kms::{load_aws_kms_from_dir, WrappedSeedFile};
    use tempfile::tempdir;

    let mut ephemeral = EphemeralTestKeyProvider::new();
    let (_pubkey, privkey) = ephemeral
        .generate_test_keypair(KeyRef::new("eph-gen"))
        .expect("ephemeral keygen");
    let seed = privkey[..32].to_vec();
    let key_ref = "kms/dir-1";
    let wrapped_marker = b"mock-dir-wrapped-seed-blob".to_vec();

    let decrypt_rule = mock!(Client::decrypt)
        .match_requests({
            let wrapped = wrapped_marker.clone();
            move |req| req.ciphertext_blob().map(|b| b.as_ref()) == Some(wrapped.as_slice())
        })
        .then_output({
            let seed = seed.clone();
            move || {
                DecryptOutput::builder()
                    .plaintext(Blob::new(seed.clone()))
                    .key_id("alias/test")
                    .build()
            }
        });
    let client = mock_client!(aws_sdk_kms, [&decrypt_rule]);

    let dir = tempdir().unwrap();
    let path = dir.path().join("slot.json");
    WrappedSeedFile {
        key_ref: key_ref.into(),
        kms_key_id: "alias/test".into(),
        wrapped_seed: wrapped_marker,
    }
    .write(&path)
    .expect("write wrapped");

    let provider = load_aws_kms_from_dir(&client, dir.path())
        .await
        .expect("load dir");
    assert_eq!(decrypt_rule.num_calls(), 1);
    let pub_out = provider
        .recipient_pubkey(&KeyRef::new(key_ref))
        .expect("pubkey");
    assert!(!pub_out.is_empty());
}
