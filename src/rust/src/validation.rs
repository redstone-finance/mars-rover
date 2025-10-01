use std::rc::Rc;

use anyhow::{anyhow, bail, ensure, Result};
use ed25519_dalek::{Verifier, VerifyingKey};
use soroban_env_common::xdr::{LedgerKey, LedgerKeyAccount, SignerKey, Uint256};
use soroban_env_host::{
    xdr::{
        AccountEntry, DecoratedSignature, Preconditions, PublicKey, SignatureHint,
        TransactionV1Envelope,
    },
    LedgerInfo,
};

use crate::{memory::Memory, utils::tx_hash};

pub struct TxValidation {
    memory: Rc<Memory>,
}

impl TxValidation {
    pub fn new(memory: Rc<Memory>) -> Self {
        Self { memory }
    }

    pub fn validate(
        &self,
        envelope: &TransactionV1Envelope,
        ledger_info: &LedgerInfo,
    ) -> Result<()> {
        let account_id = envelope.tx.source_account.clone().account_id();
        let key = LedgerKey::from(LedgerKeyAccount { account_id });

        let entry = self
            .memory
            .get_account(Rc::new(key))?
            .ok_or_else(|| anyhow!("account not found"))?;

        if entry.seq_num.0 + 1 != envelope.tx.seq_num.0 {
            bail!(
                "sequence number mismatch, got {}, expected {}",
                envelope.tx.seq_num.0,
                entry.seq_num.0 + 1
            );
        }

        if entry.balance < envelope.tx.fee as i64 {
            bail!(
                "insufficient balance: has {} needs {}",
                entry.balance,
                envelope.tx.fee
            );
        }

        self.verify_time_conds(&envelope.tx.cond, ledger_info)?;

        let hash = tx_hash(envelope, ledger_info)?;

        let mut weight = 0;

        for signature in envelope.signatures.iter() {
            let pk = self
                .get_public_key(&entry, &signature.hint)
                .ok_or(anyhow!("no matching signer found for signature hint"))?;

            self.verify_decorated_signature(&hash, signature, &pk)?;

            weight += 1;
        }

        if weight != 1 {
            bail!("invalid weight: got {}, expected {}", weight, 1);
        }

        Ok(())
    }

    fn verify_time_conds(&self, conds: &Preconditions, ledger_info: &LedgerInfo) -> Result<()> {
        match conds {
            Preconditions::None => return Ok(()),
            Preconditions::Time(time) => {
                let now = ledger_info.timestamp;
                ensure!(
                    now <= time.max_time.0 && now >= time.min_time.0,
                    "Current time {now} not within time bounds: [{}, {}]",
                    time.min_time.0,
                    time.max_time.0
                );
            },
            Preconditions::V2(v2) => {
                eprintln!("not supported, will go through {v2:?}");
            },
        };

        Ok(())
    }

    fn get_public_key(
        &self,
        account_entry: &AccountEntry,
        hint: &SignatureHint,
    ) -> Option<PublicKey> {
        let pk = &account_entry.account_id.0;

        if self.public_key_matches_hint(pk, hint.as_ref()) {
            return Some(pk.clone());
        }

        for signer in account_entry.signers.iter() {
            let pk = match &signer.key {
                SignerKey::Ed25519(pk) => PublicKey::PublicKeyTypeEd25519(pk.clone()),
                _ => return None,
            };

            if self.public_key_matches_hint(&pk, hint.as_ref()) {
                return Some(pk);
            }
        }

        None
    }

    fn public_key_matches_hint(&self, public_key: &PublicKey, hint: &[u8; 4]) -> bool {
        match public_key {
            PublicKey::PublicKeyTypeEd25519(Uint256(key_bytes)) => {
                let key_suffix = &key_bytes[key_bytes.len() - 4..];
                key_suffix == hint
            },
        }
    }

    fn verify_decorated_signature(
        &self,
        transaction_hash: &[u8; 32],
        signature: &DecoratedSignature,
        pk: &PublicKey,
    ) -> Result<()> {
        let bytes = match pk {
            PublicKey::PublicKeyTypeEd25519(Uint256(key_bytes)) => key_bytes,
        };

        let verifying_key = VerifyingKey::from_bytes(bytes)?;
        let signature = ed25519_dalek::Signature::from_bytes(
            &(signature
                .signature
                .0
                .clone()
                .try_into()
                .map_err(|e| anyhow!("{e}"))?),
        );

        verifying_key.verify(transaction_hash, &signature)?;

        Ok(())
    }
}
