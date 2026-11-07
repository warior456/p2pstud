// src-tauri/src/p2p/anchor_voucher.rs
// Anchor Attestation Vouchers: Cryptographically signed uptime & seeding vouchers issued by anchor nodes.

use anyhow::{bail, Context, Result};
use iroh::{PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorVoucher {
    pub anchor_node_id: String,
    pub subject_node_id: String,
    pub voucher_type: String,
    pub bytes_served: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

impl AnchorVoucher {
    /// Canonical signing payload for an attestation voucher.
    pub fn payload_string(
        anchor_node_id: &str,
        subject_node_id: &str,
        voucher_type: &str,
        bytes_served: u64,
        issued_at: u64,
        expires_at: u64,
    ) -> String {
        format!(
            "voucher:{anchor_node_id}:{subject_node_id}:{voucher_type}:{bytes_served}:{issued_at}:{expires_at}"
        )
    }

    /// Issue and sign a new attestation voucher with the anchor's private Ed25519 key.
    pub fn issue(
        anchor_key: &SecretKey,
        subject_node_id: &str,
        voucher_type: &str,
        bytes_served: u64,
        ttl_secs: u64,
    ) -> Result<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_at = now + ttl_secs;
        let anchor_node_id = anchor_key.public().to_string();

        let payload = Self::payload_string(
            &anchor_node_id,
            subject_node_id,
            voucher_type,
            bytes_served,
            now,
            expires_at,
        );

        let signature = anchor_key.sign(payload.as_bytes());
        let sig_bytes = signature.to_bytes();
        let signature_hex: String = sig_bytes.iter().map(|b| format!("{b:02x}")).collect();

        Ok(Self {
            anchor_node_id,
            subject_node_id: subject_node_id.to_string(),
            voucher_type: voucher_type.to_string(),
            bytes_served,
            issued_at: now,
            expires_at,
            signature: signature_hex,
        })
    }

    /// Verify signature and timestamp of this voucher.
    pub fn verify(&self) -> Result<bool> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now > self.expires_at {
            bail!("Anchor voucher expired (expired at {})", self.expires_at);
        }

        let public_key: PublicKey = self
            .anchor_node_id
            .parse()
            .context("Invalid anchor public key")?;

        if self.signature.len() != 128 {
            bail!("Invalid signature length: expected 128 hex chars");
        }

        let decoded = hex::decode(&self.signature)
            .map_err(|e| anyhow::anyhow!("Invalid hex signature: {e}"))?;
        if decoded.len() != 64 {
            bail!("Invalid signature byte length: expected 64 bytes");
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&decoded);

        let signature = Signature::from_bytes(&sig_bytes);

        let payload = Self::payload_string(
            &self.anchor_node_id,
            &self.subject_node_id,
            &self.voucher_type,
            self.bytes_served,
            self.issued_at,
            self.expires_at,
        );

        match public_key.verify(payload.as_bytes(), &signature) {
            Ok(_) => Ok(true),
            Err(e) => bail!("Voucher signature verification failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_voucher_round_trip() {
        let anchor_key = SecretKey::generate();
        let student_key = SecretKey::generate();
        let student_id = student_key.public().to_string();

        let voucher = AnchorVoucher::issue(
            &anchor_key,
            &student_id,
            "seeding",
            104_857_600,
            86400,
        )
        .expect("issue voucher");

        assert_eq!(voucher.subject_node_id, student_id);
        assert_eq!(voucher.bytes_served, 104_857_600);
        assert!(voucher.verify().expect("voucher should verify"));
    }

    #[test]
    fn test_tampered_anchor_voucher_rejected() {
        let anchor_key = SecretKey::generate();
        let student_key = SecretKey::generate();
        let student_id = student_key.public().to_string();

        let mut voucher = AnchorVoucher::issue(
            &anchor_key,
            &student_id,
            "uptime",
            50_000_000,
            3600,
        )
        .expect("issue voucher");

        // Tamper with bytes served
        voucher.bytes_served = 999_999_999;
        assert!(voucher.verify().is_err());
    }
}
