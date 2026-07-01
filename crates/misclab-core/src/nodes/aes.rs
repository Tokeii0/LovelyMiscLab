//! AES (128/192/256) in CBC / ECB / CTR modes, PKCS#7 padding for block modes.
//! Key/IV/input/output each carry a format so it slots into CTF pipelines.
use aes::cipher::{
    block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit, StreamCipher,
};

use super::prelude::*;

fn cbc(enc: bool, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if iv.len() != 16 {
        return Err(CoreError::Parse("CBC 需要 16 字节 IV".into()));
    }
    macro_rules! go {
        ($a:ty) => {
            if enc {
                Ok(cbc::Encryptor::<$a>::new_from_slices(key, iv)
                    .map_err(|_| CoreError::Parse("密钥或 IV 长度不正确".into()))?
                    .encrypt_padded_vec_mut::<Pkcs7>(data))
            } else {
                cbc::Decryptor::<$a>::new_from_slices(key, iv)
                    .map_err(|_| CoreError::Parse("密钥或 IV 长度不正确".into()))?
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .map_err(|_| CoreError::Parse("解密失败：密文长度或填充无效".into()))
            }
        };
    }
    match key.len() {
        16 => go!(aes::Aes128),
        24 => go!(aes::Aes192),
        _ => go!(aes::Aes256),
    }
}

fn ecb(enc: bool, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CoreError> {
    macro_rules! go {
        ($a:ty) => {
            if enc {
                Ok(ecb::Encryptor::<$a>::new_from_slice(key)
                    .map_err(|_| CoreError::Parse("密钥长度不正确".into()))?
                    .encrypt_padded_vec_mut::<Pkcs7>(data))
            } else {
                ecb::Decryptor::<$a>::new_from_slice(key)
                    .map_err(|_| CoreError::Parse("密钥长度不正确".into()))?
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .map_err(|_| CoreError::Parse("解密失败：密文长度或填充无效".into()))
            }
        };
    }
    match key.len() {
        16 => go!(aes::Aes128),
        24 => go!(aes::Aes192),
        _ => go!(aes::Aes256),
    }
}

fn ctr(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if iv.len() != 16 {
        return Err(CoreError::Parse("CTR 需要 16 字节 IV(nonce)".into()));
    }
    let mut buf = data.to_vec();
    macro_rules! go {
        ($a:ty) => {{
            let mut c = ctr::Ctr128BE::<$a>::new_from_slices(key, iv)
                .map_err(|_| CoreError::Parse("密钥或 IV 长度不正确".into()))?;
            c.apply_keystream(&mut buf);
        }};
    }
    match key.len() {
        16 => go!(aes::Aes128),
        24 => go!(aes::Aes192),
        _ => go!(aes::Aes256),
    }
    Ok(buf)
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key = parse_bytes(pstr(params, "key", ""), pstr(params, "keyFormat", "Hex"))?;
        if ![16usize, 24, 32].contains(&key.len()) {
            return Err(CoreError::Parse(format!(
                "AES 密钥须为 16/24/32 字节(当前 {})",
                key.len()
            )));
        }
        let iv = parse_bytes(pstr(params, "iv", ""), pstr(params, "ivFormat", "Hex"))?;
        let data = parse_bytes(in_text(inputs, "text")?, pstr(params, "inputFormat", "UTF8"))?;
        let enc = pstr(params, "operation", "加密") != "解密";

        let out = match pstr(params, "mode", "CBC") {
            "ECB" => ecb(enc, &key, &data)?,
            "CTR" => ctr(&key, &iv, &data)?,
            _ => cbc(enc, &key, &iv, &data)?,
        };
        let text = format_bytes(&out, pstr(params, "outputFormat", "Hex"));
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(text));
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(out.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "aes",
            CRYPTO,
            "AES",
            ROSE,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select("operation", "操作", &["加密", "解密"], "加密"),
                ParamSpec::select("mode", "模式", &["CBC", "ECB", "CTR"], "CBC"),
                ParamSpec::text("key", "密钥", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::text("iv", "IV", "", false),
                ParamSpec::select("ivFormat", "IV 格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::select("inputFormat", "输入格式", &["UTF8", "Hex", "Base64"], "UTF8"),
                ParamSpec::select("outputFormat", "输出格式", &["Hex", "Base64", "UTF8"], "Hex"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
