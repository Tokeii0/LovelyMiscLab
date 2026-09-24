//! OpenPGP message decryption (via rPGP). Give it an armored private key +
//! an armored PGP message (and passphrase, if the key is protected).
use pgp::composed::{Deserializable, Message, SignedSecretKey};
use pgp::types::Password;

use super::prelude::*;

fn decrypt_pgp(key_armored: &str, msg_armored: &str, pw: &str) -> Result<Vec<u8>, CoreError> {
    let (skey, _) = SignedSecretKey::from_string(key_armored)
        .map_err(|e| CoreError::Other(format!("私钥解析失败: {e}")))?;
    let (msg, _) = Message::from_string(msg_armored)
        .map_err(|e| CoreError::Other(format!("消息解析失败: {e}")))?;
    let password = Password::from(pw);
    let mut dec = msg
        .decrypt(&password, &skey)
        .map_err(|e| CoreError::Other(format!("解密失败（密钥/口令是否正确？）: {e}")))?;
    if dec.is_compressed() {
        dec = dec
            .decompress()
            .map_err(|e| CoreError::Other(format!("解压失败: {e}")))?;
    }
    dec.as_data_vec()
        .map_err(|e| CoreError::Other(format!("读取明文失败: {e}")))
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let msg = in_text(inputs, "text")?;
        let key = in_text(inputs, "key")?;
        let plain = decrypt_pgp(key, msg, pstr(params, "passphrase", ""))?;
        let mut m = PortMap::new();
        m.insert(
            "text".to_string(),
            PortValue::Text(String::from_utf8_lossy(&plain).into_owned()),
        );
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(plain.clone().into_boxed_slice())),
        );
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&plain)));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pgp_decrypt",
            CRYPTO,
            "PGP 解密",
            ROSE,
            vec![
                req("text", "PGP 消息", PortType::Text),
                req("key", "私钥(armored)", PortType::Text),
            ],
            vec![
                req("text", "明文", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
                opt("hex", "hex", PortType::Text),
            ],
            vec![ParamSpec::text("passphrase", "口令(可空)", "", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::composed::{
        ArmorOptions, EncryptionCaps, KeyType, MessageBuilder, SecretKeyParamsBuilder,
        SignedPublicKey, SubkeyParamsBuilder,
    };
    use pgp::crypto::ecc_curve::ECCCurve;
    use pgp::crypto::sym::SymmetricKeyAlgorithm;
    use rand::thread_rng;

    /// Ed25519 primary + one Curve25519 ECDH encryption subkey — fast to generate.
    fn gen_key() -> SignedSecretKey {
        let mut enc = SubkeyParamsBuilder::default();
        enc.key_type(KeyType::ECDH(ECCCurve::Curve25519Legacy))
            .can_sign(false)
            .can_encrypt(EncryptionCaps::All)
            .can_authenticate(false);
        let mut params = SecretKeyParamsBuilder::default();
        params
            .key_type(KeyType::Ed25519Legacy)
            .can_certify(true)
            .can_sign(false)
            .can_encrypt(EncryptionCaps::None)
            .primary_user_id("Test <t@e.com>".into())
            .subkeys(vec![enc.build().unwrap()]);
        params.build().unwrap().generate(thread_rng()).unwrap()
    }

    #[test]
    fn generate_encrypt_decrypt_roundtrip() {
        let skey = gen_key();
        let key_armored = skey.to_armored_string(ArmorOptions::default()).unwrap();

        let pubkey = SignedPublicKey::from(skey.clone());
        let enc_subkey = &pubkey.public_subkeys[0];
        let mut builder = MessageBuilder::from_bytes("", b"TOP SECRET FLAG".to_vec())
            .seipd_v1(thread_rng(), SymmetricKeyAlgorithm::AES256);
        builder.encrypt_to_key(thread_rng(), &enc_subkey).unwrap();
        let msg_armored = builder
            .to_armored_string(thread_rng(), ArmorOptions::default())
            .unwrap();

        let plain = decrypt_pgp(&key_armored, &msg_armored, "").unwrap();
        assert_eq!(plain, b"TOP SECRET FLAG");
    }
}
