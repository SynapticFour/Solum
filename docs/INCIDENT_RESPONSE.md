# Solum — Incident Response Runbook

**Status:** Living · 2026-08-12
**Audience:** Operators + Synaptic Four support
**Company plan:** Synaptic Four `synapticfour-business/security/incident-response-plan.md` (private)
**Threat model:** [THREAT_MODEL.md](THREAT_MODEL.md) · [customer/SECURITY-OVERVIEW.md](customer/SECURITY-OVERVIEW.md)

---

## 1. What counts as an incident

- Unauthorized access to clinical payloads, consent store, or subject-link store
- Crypt4GH key / KMS-wrapped seed exposure
- Audit chain gaps or suspected tampering
- Sidecar deployed with ephemeral keys in a regulated environment
- EHRbase/CDR breach (Track B)
- Consent revoke failed to block Ferrum access when co-deployed (control failure)

---

## 2. Severity

| Level | Examples |
|-------|----------|
| Critical | Confirmed clinical data exfiltration; key theft; CDR DB dump |
| High | Authz bypass; public sidecar with real data; backup theft |
| Medium | Contained misconfig; failed intrusion; provisional profile used as if PRODUCTION |
| Low | Dependency advisory; documentation gap |

---

## 3. Immediate actions (0–1 h)

1. Preserve consent/audit/subject-link stores and sidecar logs.
2. Contain: disable ingress; revoke OIDC sessions; rotate keys; stop dual-write if integrity unsure.
3. Notify site DPO / security + `contact@synapticfour.com` if under support contract.
4. If Ferrum companion enabled: treat genomic access as potentially affected; coordinate Ferrum IR.

---

## 4. Investigation

- Reconstruct access from audit export (`solum-audit-helios-*` shapes)
- Verify hash-chain continuity
- Check capability / org-IAM mappings
- Track B: include EHRbase DB and backup integrity
- Document whether plaintext could have existed only in process memory vs at rest

---

## 5. Notification

Operator (controller) follows DPA / national rules (often 72h). Synaptic Four assists with technical facts when contracted — does not replace counsel.

---

## 6. Recovery

- Rotate CustomerHeld keys / KMS material; re-encrypt if required by policy
- Restore stores from last known-good backup
- Re-validate fail-closed authz + consent Deny paths (Solum-Demo / Showcase recipes)
- Re-issue HELIOS-oriented export after recovery for evidence continuity

---

## 7. Post-incident

Postmortem + update threat model / BASELINE accepted risks if needed. Escalate profile status issues to counsel (e.g. Kenya provisional).

---

## 8. Contacts

| Role | Contact |
|------|---------|
| Synaptic Four | contact@synapticfour.com |
| Site DPO / security | *(fill)* |
| Counsel | *(fill)* |
