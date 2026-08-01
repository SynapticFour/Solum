# Solum Go-to-Market Readiness

## Ziel

Solum so weit bringen, dass ein erster Pilotkunde (Klinik/Health-Tech, on-premise, standalone ODER Ferrum-Companion) das Produkt real einsetzen kann — nicht als gehosteter SaaS-Dienst (das bleibt Stage 2, siehe roadmap.md), sondern als installierbares, vertrauenswürdiges On-Premise-Produkt.

## Leitprinzip

Jede GTM-Erweiterung bleibt additiv zu beiden Betriebsmodi (Standalone, Ferrum-Companion) — dieselbe Disziplin wie in INTEGRATION-ROADMAP.md. Neue externe Abhängigkeiten (z.B. AWS KMS) werden erst recherchiert, dann implementiert — nicht aus Annahmen heraus gebaut (Lehre aus Sprint 4/5).

## Identifizierte Blocker

1. **Schlüsselverwaltung (teilweise geschlossen).** CLI-Evaluationspfad: CustomerHeld via `crypto keygen` + `--keypair`. Ephemeral nur mit `SOLUM_ALLOW_EPHEMERAL=1` + `dev-local`. Pilot-Profile verweigern `EphemeralTest`. AWS-KMS-Envelope bleibt library-only (GTM-3). HSM-Anbindung / Zeroize bleiben offen.
2. **Keine durchgesetzte Autorisierung.** `SolumActor.scopes` wird aufgezeichnet, aber nirgends geprüft — jeder Actor-String kann jede Operation auslösen.
3. **Keine kundenlesbare Sicherheits-/Compliance-Doku.** Bestehende Docs sind entwicklerorientiert.
4. **Kein Deployment-Runbook.** `scripts/verify.sh` ist ein Entwicklertool, kein Kunden-Onboarding.

## Sprints

### GTM-1 — Rollenbasierte Autorisierung

`Deployment`-Operationen (grant/revoke/encrypt/decrypt) prüfen vor Ausführung, ob `SolumActor.scopes` die nötige Capability enthält. Additiv zu den `*_as`-Methoden aus Sprint 2; bestehende `&str`-Pfade bleiben unautorisiert/Legacy, klar dokumentiert. Kein externes API-Risiko.

### GTM-2 — AWS-KMS-Recherche (nur Analyse, kein Code)

Klären, bevor implementiert wird:
- AWS KMS unterstützt keine Curve25519/X25519-Schlüssel (nur RSA, NIST-P-Kurven, symmetrisch) — Crypt4GH basiert auf X25519-ECDH. Direkte Schlüsselhaltung in KMS funktioniert vermutlich nicht.
- Envelope-Modell prüfen: KMS schützt den Crypt4GH-Private-Key at rest (symmetrische `Encrypt`/`Decrypt`-Operation oder `GenerateDataKey`), Solum entschlüsselt kurzzeitig im Prozess für die eigentliche ECDH-Operation.
- Verhältnis zu bestehendem `CustomerHeldKeyProvider` klären: neue Provider-Implementierung oder Erweiterung.

### GTM-3 — AWS-KMS-Envelope-Implementierung

Basiert auf GTM-2-Rechercheergebnis. Feature-gated, analog zum `ferrum-storage-backend`-Muster aus Sprint 4 (default aus).

### GTM-4 — Kunden-Doku

Sicherheits-/Compliance-Whitepaper (nicht-technisches Publikum) + Deployment-Runbook. Keine Code-Abhängigkeit, kann parallel laufen.

## Danach (wichtig, nicht blockierend für ersten Verkauf)

- FHIR/IPS-Struktur-Review durch Fachperson (bereits als offen markiert seit Sprint 3)
- Multi-Writer-Audit-Backend (falls Pilotkunde mehr als eine Instanz braucht)
- CI-Abdeckung für `ferrum-storage-backend`-Feature-Pfad
- Web-Dashboard, Live-HELIOS-Signing, weitere Jurisdiktionen — eigene Produktentscheidungen, analog zu Sprint 6 (Turnkey)
