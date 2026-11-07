# KU Leuven Student Identity & Groep T Swarm Authentication Specification

> **Status:** Proposed / Design Specification (Not Implemented)  
> **Target Network:** `groept` Campus P2P Swarm  
> **Author:** Antigravity Architect & Security Team  
> **Date:** 2026-11-04  

---

## 1. Executive Summary & Objective

Currently, swarm isolation in `p2pstud` relies on a Pre-Shared Key (PSK / Network Secret) that derives a dynamic QUIC ALPN. While effective against casual discovery, a static PSK suffers from the **"Discord/WhatsApp Leak" problem**: if one student posts the secret in a public chat or online forum, any non-KU Leuven user can join the network indefinitely.

This document specifies an individual, cryptographically bound authentication model using **Decentralized Capability Tickets (Individual Attestations)**. Instead of sharing a common passphrase, each student authenticates their institutional identity (`@student.kuleuven.be`) once and receives a non-transferable, digitally signed ticket bound strictly to their local device's Ed25519 key (`NodeId`).

---

## 2. Threat Model & Security Properties

| Property | Threat Mitigated | Mechanism |
|---|---|---|
| **Non-Transferability** | Student Alice copies her key and sends it to Bob on Discord. | Ticket is cryptographically bound to Alice's `NodeId`. Bob cannot use it because he does not possess Alice's private key (`iroh_secret.key`). |
| **Institutional Proof** | Strangers or bots joining the Groep T swarm. | Only users capable of receiving a verification code at `rXXXXXXX@student.kuleuven.be` (guarded by KU Leuven Central Login + 2FA) are issued tickets. |
| **Privacy & Pseudonymity** | Classmates scraping a directory of students' real names/emails. | Memory-Hard Key Derivation (Argon2id, m=64MB, t=3, p=4 with campus pepper) or Blind Credential Issuance. Plaintext emails and low-entropy BLAKE3 hashes are strictly forbidden. |
| **Time-Bound Validity** | Departed students or lost devices retaining access indefinitely. | Tickets carry an expiration timestamp (e.g. 30 days or semester end). |
| **Abuse & Sybil Resistance** | A student uploading spam or malware. | The Anchor Authority can revoke tickets or block the offending `email_hash` from receiving future passes. |

---

## 3. Cryptographic Architecture

```
                               ┌──────────────────────────────────────────────┐
                               │       Anchor Auth Authority (VPS)            │
                               │  Private Key: Ed25519_PrivKey (Secret)       │
                               │  Public Key:  Ed25519_PubKey  (Embedded)     │
                               └──────────────────────┬───────────────────────┘
                                                      │
                       1. Student submits r-number    │ 2. Sends 6-digit OTP to
                          + local NodeId              │    rXXXXXXX@student.kuleuven.be
                                                      │
                                                      ▼
┌─────────────────────────────────┐           ┌────────────────────────────────┐
│      Student Device (p2pstud)   │           │    KU Leuven Outlook Inbox     │
│   • Owns Ed25519 iroh_secret    │           │    (Guarded by Central Login)  │
│   • Public Key: NodeId          │           └────────────────┬───────────────┘
└────────────────┬────────────────┘                            │
                 │ 3. Enters OTP in App                        │
                 ▼                                             ▼
                 └──────────────────► Validates OTP ───────────┘
                                             │
                                             │ 4. Issues Digitally Signed
                                             │    Membership Ticket
                                             ▼
                               ┌──────────────────────────────────────────────┐
                               │          Signed Membership Ticket            │
                               │  • node_id:   <Student's Public Key>         │
                               │  • id_hash:   Argon2id(pepper, email, salt)  │
                               │  • network:   "groept"                       │
                               │  • expires:   1773000000                     │
                               │  • signature: Ed25519_Sign(TicketBytes)      │
                               └──────────────────────────────────────────────┘
```

---

## 4. Ticket Data Format

The ticket is serialized as canonical CBOR or compact JSON before signing:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroepTMemberTicket {
    /// The public Ed25519 NodeId of the student's local Iroh node.
    pub node_id: String,

    /// Memory-hard Argon2id digest of the student's r-number/email with campus pepper.
    /// Defends against sub-second precomputation rainbow attacks on the 10^7 KU Leuven r-number space.
    pub email_hash: String,

    /// Identifier of the target swarm network.
    pub network: String, // "groept"

    /// Unix epoch timestamp (seconds) when the ticket was issued.
    pub issued_at: i64,

    /// Unix epoch timestamp (seconds) when the ticket expires (e.g. +30 days).
    pub expires_at: i64,

    /// Ed25519 signature generated by the Anchor Authority private key.
    pub signature_hex: String,
}
```

---

## 5. Peer-to-Peer Verification Protocol

When two nodes connect over QUIC (ALPN `/p2pstud/sync/1/groept`):

1. **Mutual Challenge:**
   - Both nodes require the remote peer to present a valid `GroepTMemberTicket` on the initial bi-directional sync stream.
2. **Three-Point Verification Checklist:**
   - **Signature Check:** Is `ticket.signature` valid against the hardcoded `GROEPT_AUTHORITY_PUBLIC_KEY`?
   - **Key Binding Check:** Does `ticket.node_id` match the authenticated QUIC transport remote NodeId (`connection.remote_node_id()`)?
   - **Expiration Check:** Is `SystemTime::now() < ticket.expires_at`?
3. **Rejection:**
   - If any check fails, the local peer terminates the QUIC stream with error code `401 Unauthorized` (`ConnectionError::Reset`).

---

## 6. Authentication Flows

### Flow A: Email OTP via KU Leuven Inbox (Recommended)

1. **User Action:**
   - Student clicks **"Join Groep T Campus Swarm"** in settings.
   - Enters their student email: `r0123456@student.kuleuven.be`.
2. **Backend Validation:**
   - Regex check: `^r[0-9]{7}@student\.kuleuven\.be$`.
   - Rate limit: max 3 requests per IP / email per hour.
   - Generates random 6-digit numeric OTP with 10-minute TTL in Redis/SQLite.
   - Transmits email via outbound relay:
     - **Zero-domain approach (No domain required):** Gmail SMTP (`smtp.gmail.com:587`) using a dedicated Google Account with an App Password. Because emails originate directly from Google's servers, they carry authentic Google DKIM and SPF signatures that pass KU Leuven's Microsoft 365 Defender filters without getting junked.
     - **Custom-domain approach:** Resend or Brevo API (requires adding DKIM/SPF DNS records to a registered domain).
3. **Ticket Issuance:**
   - Student enters the code in `p2pstud`.
   - App calls `POST /api/auth/verify-otp` with:
     ```json
     {
       "email": "r0123456@student.kuleuven.be",
       "otp": "492817",
       "node_id": "8b9a1c...ed25519_node_id..."
     }
     ```
   - Server validates code, constructs the ticket, signs it, and returns it to the app.

---

### Flow B: Microsoft Entra ID (Azure AD) OIDC

1. **User Action:**
   - Student clicks **"Sign in with KU Leuven"**.
2. **OAuth2 Redirect:**
   - Opens browser to Microsoft OAuth endpoint (`https://login.microsoftonline.com/common/oauth2/v2.0/authorize`).
   - Tenant resolution routes user to `login.kuleuven.be`.
3. **Token Verification:**
   - Server verifies the JWT signature against Microsoft's public keys (`jwks_uri`).
   - Validates that claim `tid` matches the KU Leuven Azure tenant or `email` ends with `@student.kuleuven.be`.
   - Signs and delivers the `GroepTMemberTicket` back to the app via custom scheme `p2pstud://auth?ticket=...`.

---

## 7. Revocation & Emergency Key Rotation

- **CRL Broadcast:**
  - The Anchor node maintains a Certificate Revocation List (CRL) of blacklisted `email_hash` or `node_id` entries.
  - During the routine 5-second swarm sync, the CRL is synced to all connected peers.
  - If a student is expelled or compromises network integrity, their hash is placed on the CRL, preventing them from exchanging catalog entries.
- **Authority Key Rotation:**
  - The client binary embeds the current Authority Public Key and an emergency fallback secondary key.
  - If the primary authority key is ever compromised, the Anchor node signs a revocation announcement using the secondary key to deprecate the old tickets.

---

## 8. Summary of Advantages

- **Zero Secret Leaking:** Leaking a ticket on Discord gives an attacker nothing, as they lack the private key matching `ticket.node_id`.
- **Zero IT Bureaucracy (Flow A):** Uses standard email delivery without requiring administrative consent from KU Leuven ICTS.
- **Privacy-First:** Classmates verify institutional membership without scraping student names or ID numbers.
- **Automatic Sync:** Preserves fast, local LAN/Eduroam peer transfers while maintaining enterprise-grade access control.

