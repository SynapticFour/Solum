Vorlage für kundenspezifische Integrationen, kein fertiges Produkt für ein bestimmtes KIS-System.

Track A bleibt der HTTP-Sidecar neben dem bestehenden KIS (`docs/customer/SIDECAR-INTEGRATION.md`). Dieses Verzeichnis ist die Rust-Schnittstelle davor: Consent-Abgleich, Feld-Mapping, Fehlerbehandlung. Es öffnet keinen Socket, vergibt kein Consent und nennt kein KIS-Produkt.

`MockKisConnector` ist der einzige Stand. Die Feldnamen (`patient_id`, `given_name`, `family_name`) sind eine Vorlage, keine Vendor-Schnittstelle. Ein Abgleich ist nur in Ordnung, wenn KIS-Angabe und Sidecar-Datensatz dieselbe Entscheidung tragen. Eine fehlende Angabe, ein Widerspruch, ein unbekanntes Feld oder ein leerer Wert bricht ab. `classify_error` sagt in jedem Fall Stopp und startet keinen zweiten Versuch.

Der FHIR-Zuschnitt für Patient bleibt `solum_fhir::to_kis_patient_adapter`. Der ist kein ISiK-Nachweis. Dieser Konnektor ersetzt ihn nicht.
