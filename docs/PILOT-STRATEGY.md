# Solum Pilot Strategy — Research Findings

## Zweck

Festhalten, was zur Pilotkunden-Suche recherchiert wurde (Web-Recherche,
Stand 2026-07-30), bevor daraus technische/vertriebliche Schritte
abgeleitet werden. Kein Teil dieses Dokuments ist eine Rechtsberatung —
insbesondere die EHDS-Einordnung unten braucht echte anwaltliche
Bestätigung, bevor sie in Vertragssprache oder Marketing verwendet wird.

## Recherchierte Pilotkandidaten-Profile

### Kenia (zwei konkrete Firmen recherchiert, keine Kontaktaufnahme erfolgt)

| | Hanmak Technologies | Medbook (AphiaOne) |
|---|---|---|
| Gegründet | 2008 | 2014, Strathmore-Universität-Joint-Venture |
| Größe | Kleiner, lokaler Anbieter | 10–50 Mitarbeitende, ZEP-RE/DEG-finanziert |
| Reichweite | Krankenhäuser in Kenia (Umfang unklar) | 74–200+ Facilities, 9,8 Mio. Patientenakten |
| Sichtbarkeit | Lokal etabliert | Bloomberg Top-25-African-Startup 2025, Gates-Foundation-KI-Förderung |
| Compliance-Signal | "Certified by the Data Protection Commissioner" — vermutlich DPA-Registrierung, keine Sicherheitszertifizierung | Verweist auf Kenya DPA, keine erkennbare dedizierte Audit-/Krypto-Schicht |

**Akuter Zeitdruck (unverändert aus vorheriger Recherche):** Social Health
Authority 90-Tage-Ultimatum (ab Ende Juni 2026) für DHA-zertifiziertes
HMIS, sonst Verlust der SHA-Akkreditierung — läuft zum Zeitpunkt dieser
Recherche.

### EU (Regulierungs-Recherche, keine firmenspezifische Prüfung)

Kandidatenprofil: kleiner/mittlerer EHR-Anbieter oder CEE-Digital-Health-
Startup unter EHDS-Zeitdruck, ohne eigenes Compliance-Engineering-Team.

## Kern-Fund: gemeinsamer Nenner

Trotz sehr unterschiedlicher Größe/Reife (kleiner Kenia-Anbieter, gut
finanzierter Kenia-Anbieter, EU-EHR-Vendor) ist der regulatorisch
verlangte Kern bei allen drei nahezu identisch:

1. Nachweisbarer, manipulationssicherer Audit-Trail
2. Feldverschlüsselung mit nachweisbarer Schlüsselkontrolle
3. FHIR-basierte Interoperabilität

Das deckt sich fast exakt mit den zwei EHDS-verpflichtenden "harmonisierten
Software-Komponenten" für EHR-Systeme (Art. 25 EHDS): der europäischen
Interoperabilitäts-Komponente und der europäischen Logging-Komponente.
**Der gemeinsame Nenner ist kein neuer, schmalerer Funktionsumfang — es
ist im Wesentlichen der bereits implementierte Solum-Kern** (Audit,
Crypto, FHIR-Interop aus solum-audit/solum-crypto/solum-fhir).

## Zertifizierungsrecherche (Stand 2026-07-30)

### EHDS (EU)

- Regulation (EU) 2025/327, in Kraft seit 26. März 2025. Volle Anwendung
  ab März 2027; EHR-System-Pflichten werden laut EMA-Zeitplan ab
  **März 2029** durchgesetzt.
- CE-Kennzeichnung für EHR-Systeme ist eine **Selbstzertifizierung** nach
  dem New Legislative Framework — Hersteller erstellt technische
  Dokumentation, prüft selbst gegen eine noch nicht existierende
  "European digital testing environment", gibt EU-Konformitätserklärung
  ab. **Keine Notified-Body-Prüfung** wie bei Medizinprodukten.
- Nur zwei verpflichtende Komponenten: europäische Interoperabilitäts-
  Komponente + europäische Logging-Komponente (Annex II) — nicht das
  gesamte EHR-System muss neu zertifiziert werden.
- "EHR-System" ist definiert als das Gesamtprodukt, das vom Hersteller
  dafür bestimmt ist, von Gesundheitsdienstleistern/Patienten genutzt zu
  werden. Pflichten treffen laut Recherche "Manufacturers of EHR
  systems... importers, authorised representatives, distributors" — eine
  eingebettete Unterkomponente (wie Solum, integriert in ein fremdes
  Produkt) fällt strukturell nicht unter diese Herstellerdefinition; der
  integrierende Vendor ist der Hersteller. **ANNAHME, bitte anwaltlich
  bestätigen** — nicht nur aus Sekundärquellen abgeleitet, keine
  Rechtsberatung.

### ISO 27001

Weltweit freiwillig, kein Gesetz verlangt es — weder EU noch (soweit
recherchiert) Kenia. Markt-/Vertragssignal, keine rechtliche Pflicht.

### NIS2 (EU)

Echte Rechtspflicht (EU-Richtlinie, national umgesetzt, Schwellenwerte
nach Größe/Kritikalität pro Land unterschiedlich). Betrifft eher größere
Klinikbetreiber (potenzielle Solum-Endkunden) als einen kleinen
Zulieferer wie Solum selbst — pro Zielland zu prüfen, nicht abschließend
geklärt.

### Kenia

DHA-/SHA-Akkreditierung betrifft das HMIS-Produkt des Kunden (Hanmak,
Medbook), nicht eine Zulieferer-Komponente wie Solum direkt.

### Partnerschaftsmodell

Realistischer Weg: Solum liefert die technische Dokumentation (Audit-,
Verschlüsselungs-, Interop-Nachweis) als Teil des Lieferumfangs, der
Kunde nutzt das für seine eigene Selbsterklärung/Zertifizierung. Sollte
explizit als Lieferbestandteil im Pilotvertrag stehen.

## Identifizierte Gaps (vor ernsthaftem Outreach zu klären)

1. Rechtsgutachten zur "Hersteller"-Frage unter EHDS fehlt
2. Haftungsfrage bei Datenleck trotz Solum-Einsatz ungeklärt
3. Kein Preismodell definiert
4. Solum läuft technisch noch nicht produktiv beim Kunden (Demo-Schlüssel
   default, AWS-KMS nur Library, kein CLI-Wrapper)

## Identifizierte Opportunities

1. Positionierung "Solum = EHDS-Logging- und Interoperabilitäts-
   Komponente, fertig zum Einbetten" ist schärfer als generisches
   "Compliance-Layer"
2. Die "European digital testing environment" existiert noch nicht —
   Chance zur frühen Mitwirkung/Referenzimplementierung
3. Kenias SHA-90-Tage-Frist ist der akuteste Zeitdruck-Hebel aller drei
   Kandidaten
4. Medbook (Gates-Foundation-/DEG-Förderung, Bloomberg-Sichtbarkeit) wäre
   eine sehr vorzeigbare Referenz, falls ein Pilot zustande kommt

## Vorgeschlagener nächster technischer Schritt

Kein neuer Funktionsumfang nötig (Kern deckt sich bereits mit dem
recherchierten Bedarf). Stattdessen: ein "Embed-Profil"-Konzept neben den
bestehenden Jurisdiktionsprofilen — beschreibt, WIE ein HMIS/EHR-Vendor
Solum einbindet (Library vs. Sidecar-Service), nicht WAS rechtlich gilt.
Für PHP/Python/Java-basierte HMIS-Systeme (wahrscheinlich bei allen drei
Kandidaten-Profilen) ist ein Sidecar-Service der realistischere
Integrationsweg als eine direkte Rust-Library-Einbindung.
