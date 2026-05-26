//! Field-level encryption for properties marked `encrypted: true`.

use crate::crypto::Sealer;
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{ObjectType, Record};
use std::collections::BTreeMap;

/// Encrypt values for properties marked `encrypted: true`.
pub fn encrypt_sensitive_fields(
    values: &mut BTreeMap<ApiName, Value>,
    ot: &ObjectType,
    sealer: &dyn Sealer,
) -> Result<(), Error> {
    for prop in &ot.properties {
        if prop.encrypted != Some(true) {
            continue;
        }
        if let Some(val) = values.get(&prop.api_name) {
            if val.is_null() {
                continue;
            }
            let plaintext = serde_json::to_vec(val)
                .map_err(|e| Error::internal(format!("serialize for encryption: {}", e)))?;
            let ciphertext = sealer.seal(&plaintext)?;
            let encoded = B64.encode(&ciphertext);
            values.insert(prop.api_name.clone(), Value::from(encoded));
        }
    }
    Ok(())
}

/// Decrypt record fields for properties marked `encrypted: true`.
pub fn decrypt_record_fields(
    record: &mut Record,
    ot: &ObjectType,
    sealer: &dyn Sealer,
) -> Result<(), Error> {
    for prop in &ot.properties {
        if prop.encrypted != Some(true) {
            continue;
        }
        if let Some(val) = record.values.get(&prop.api_name) {
            let encoded = match val.as_str() {
                Some(s) => s,
                None => continue,
            };
            let ciphertext = B64.decode(encoded).map_err(|e| {
                Error::internal(format!(
                    "base64 decode for field '{}': {}",
                    prop.api_name, e
                ))
            })?;
            let plaintext = sealer.open(&ciphertext)?;
            let original: Value = serde_json::from_slice(&plaintext).map_err(|e| {
                Error::internal(format!("deserialize decrypted '{}': {}", prop.api_name, e))
            })?;
            record.values.insert(prop.api_name.clone(), original);
        }
    }
    Ok(())
}
