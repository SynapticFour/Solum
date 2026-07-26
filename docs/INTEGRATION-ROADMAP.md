# Solum ⨯ Ferrum Integration Roadmap

## Leitprinzip

Solum bleibt ein eigenständiger Compliance-Layer — kein durabler Storage per Default, kein Medizinprodukt, keine Ferrum-Abhängigkeit für Standalone-Betrieb. Jede Ferrum-Integration in diesem Dokument ist:

- **additiv** (bestehende APIs bleiben unverändert nutzbar)
- **optional** (per Cargo-Feature-Flag oder separatem Konstruktor, nie Default-Pfad)
- **gegen beide Betriebsmodi validiert**, nicht nur gegen den, für den sie gebaut wurde

Jeder Sprint endet mit: Diff-Review vor Commit → `cargo test --workspace` grün → `./scripts/verify.sh` grün → CI grün (alle 4 Checks: CI, CodeQL, Secret Scan, Quality Gate) → neuer Baseline-Tag. Kein Sprint gilt als "fertig", ohne dass beide Referenz-Deployments (siehe Sprint 1) weiterhin grün sind.

## Betriebsmodi (Referenz)

| Modus | Storage | Auth | Ferrum-Abhängigkeit |
|---|---|---|---|
| **A. Standalone** | Kunde (BYO) | Kunde/SMART-on-FHIR | keine |
| **B. Ferrum-Companion** | Kunde/`ferrum-storage` (optional) | `ferrum-core::auth` (optional) | git-gepinnt, wie heute bei Crypto |

## Erkenntnisse aus dem Ferrum-Repo-Review (2026-07-26)

- `ferrum-storage`: vollständige Storage-Abstraktion (`LocalStorage`, `S3Storage` mit Multipart, optional `OpenDAL` für GCS/Azure/iRODS/etc.) — kein Scaffold.
- `ferrum-meta-connect`: validiert Forschungs-Submission-Metadaten (Studies/Samples/Individuals/Experiments/Datasets) gegen `Core`/`Pathogen`/`H3Africa`-Profile inkl. DUO-Codes — Genomik-Forschungsmetadatik, nicht klinisch.
- `ferrum-mii-connect`: validiert bereits **klinische FHIR-Ressourcen** gegen deutsche MII-Kerndatensatz-Profile (17 Module: Person, Encounter, Consent, Diagnose, Labor, Medikation, Onkologie, Pathologie, molekulare Berichte, Bildgebung, ICU, Biobank, Research Study u.a.) — explizit als "technical conformance, not legal advice about regulatory compliance" positioniert. Direkte fachliche Nachbarschaft zu Solums `solum-fhir`.
- `ferrum-passports`: vollständiger GA4GH Passport Broker + Visa Issuer (OIDC-Discovery, JWKS, `/authorize`, `/token`, `/userinfo`, Admin-Endpunkte).
- `ferrum-core::auth` (öffentlich exportiert, bereits gepinnt via `crates/crypto`): fertiges Claims-Modell `AuthClaims`/`PassportClaims`/`VisaObject` mit `has_dataset_grant()`, `has_scope()`, `is_admin()`, `issuer()`.

Diese Erkenntnisse verkleinern den ursprünglich angenommenen Scope erheblich: Auth- und Storage-Interop bedeuten größtenteils **Wiederverwendung bestehenden, geprüften Ferrum-Codes**, nicht Neubau paralleler Solum-Abstraktionen.

---

## Sprint 1 — Referenz-Deployments (Validierungs-Fundament)

**Ziel:** Beweisen, dass der heutige Stand (`stage1-baseline-cli-2026-07-26`) in beiden Modi tatsächlich funktioniert — als lebender Regressionstest für alle folgenden Sprints, nicht nur als Behauptung.

**Deliverables:**
- `examples/standalone/` — Shell-Skript + README: `Deployment` rein gegen eine fiktive bestehende Patienten-DB, keinerlei Ferrum-Bezug.
- `examples/ferrum-companion/` — nutzt den bereits gepinnten `ferrum-core`-Import **echt** (nicht gemockt): zeigt, dass ein Ferrum-seitig Crypt4GH-verschlüsseltes Objekt und ein Solum-seitig Crypt4GH-verschlüsseltes `patient_summary`-Feld für denselben Patienten dasselbe Schlüsselformat teilen.
- Smoke-Test: Solum kann einen `ferrum_core::auth::AuthClaims`-Wert konstruieren/parsen (noch keine Logik darauf).
- Beide Beispiele als neuer Abschnitt "7. Reference deployments" in `verify.sh`.

**Validierung:** Beide Beispiele laufen in CI durch. **Nicht-Ziel:** noch keine neue Business-Logik, reine Beweisführung.

---

## Sprint 2 — Actor-Identität vereinheitlichen (Auth-Adapter)

**Ziel:** `actor: String` additiv durch eine strukturierte Identität ergänzen, die aus beiden Auth-Welten befüllbar ist.

**Deliverables:**
- Neuer Typ `SolumActor`: `subject_id`, `display`, `source: ActorSource::{FerrumPassport, Standalone, LocalDev}`, `scopes: Vec<String>`.
- `impl From<String> for SolumActor` — bestehende Aufrufe bleiben unverändert gültig.
- `SolumActor::from_ferrum_claims(&ferrum_core::auth::AuthClaims)` — feature-gated (`ferrum-companion`-Cargo-Feature).
- Minimal-Äquivalent für Standalone (SMART-on-FHIR-artige Claims), noch keine Live-Token-Verifikation (Sprint 5).

**Validierung:** Alle bestehenden Consent-/Audit-Tests unverändert grün mit `SolumActor::from(String)`. Neue Tests: Ferrum-Claims-Fixture und Standalone-Fixture erzeugen äquivalente `SolumActor`s in identischen Audit-Records.

---

## Sprint 3 — FHIR/MII-Grenze dokumentieren + leichte Kopplung

**Ziel:** Arbeitsteilung zwischen `ferrum-mii-connect` (strukturelle FHIR-Konformität) und Solums Compliance-Schicht (Verschlüsselung, Consent, Audit) explizit festhalten.

**Deliverables:**
- Neuer Abschnitt in `docs/architecture.md`.
- `solum-fhir`: optionales Feld `mii_validation_ref: Option<String>` an `PatientSummary`.
- Audit-Event bei Verarbeitung eines bereits MII-validierten Feldes.

**Validierung:** Bestehende `solum-fhir`-Tests unverändert grün, neue Tests für additives Feld.

**Hinweis:** Keine Änderungen an Ferrum selbst in diesem Sprint — separates Repo, eigener Rhythmus.

---

## Sprint 4 — Optionale Storage-Wiederverwendung (`ferrum-storage`)

**Ziel:** `Deployment` um eine rein optionale Fähigkeit erweitern, verschlüsselte Felder über Ferrums getestete Storage-Backends zu persistieren.

**Deliverables:**
- Cargo-Feature `ferrum-storage-backend` (default: aus).
- `Deployment::with_storage(impl ferrum_storage::Storage)` — additiv.
- Zuerst nur `LocalStorage`, `S3Storage`/OpenDAL später.

**Validierung:** `examples/standalone/` beweist: Default-Build ohne `ferrum-storage`-Abhängigkeit. `examples/ferrum-companion/` beweist: mit Feature an funktioniert Persistenz durchgängig.

---

## Sprint 5 — Echte Auth-Verifikation (Live-IdP)

**Ziel:** Tokens gegen echte JWKS-Endpunkte verifizieren (GA4GH Passport Broker + SMART-on-FHIR-OIDC-Provider).

**Bewusst zuletzt:** höchstes externes Abhängigkeitsrisiko (Netzwerk, Schlüssel-Rotation, Uhrzeit-Skew). Ferrums bestehendes `DEFAULT_JWKS_CACHE_TTL`/Skew-Handling zuerst studieren, bevor Solum etwas Eigenes baut.

---

## Sprint 6 — Turnkey-Modus (Mode 3) — Produktentscheidung, kein reiner Engineering-Task

Nicht automatisch nach Sprint 5 gestartet — Vertrauensmodell-Änderung (Solum besitzt Storage), die bewusst kommuniziert werden muss, bevor Code entsteht. Wird erst geplant, wenn 1–5 stehen und explizit angestoßen wird.
