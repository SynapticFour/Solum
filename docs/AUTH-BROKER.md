# Broker-fronted hospital login

The person still logs in at the hospital identity provider. A token broker can sit in front of that provider and mint the token Solum sees.

Solum's trust anchor is the **broker** issuer and the broker JWKS. It is not the hospital identity provider's JWKS. A token whose `iss` is the hospital identity provider is rejected once Solum is configured for the broker issuer.

Solum accepts the broker token only when `aud` is the configured audience and `groups` carries a value the org-IAM map already knows. One example is `data-steward@demo.invalid` mapped to `solum:consent:grant`. Nested `ga4gh_passport_v1` is not what this path reads. With the broker's optional visa-claim feature off, Solum does not grow a visa decoder.

Actor storage stays `standalone:<sub>`. Solum still does not mint Passports.

This is not a new default profile. Hospital profiles in [AUTH-HOSPITAL.md](AUTH-HOSPITAL.md) stay as they are. Point `--oidc-issuer` and `--jwks-url` (or `--jwks-file`) at the broker, and point `--org-iam-config` at a map whose claim values match the broker's `groups`.
