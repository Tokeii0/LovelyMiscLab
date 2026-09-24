//! Blowfish (variable-length key) in CBC / ECB, PKCS#7 padding.
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit};

use super::prelude::*;

type Bf = blowfish::Blowfish;

fn cbc(enc: bool, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if iv.len() != 8 {
        return Err(CoreError::Parse("Blowfish CBC 需要 8 字节 IV".into()));
    }
    if enc {
        Ok(cbc::Encryptor::<Bf>::new_from_slices(key, iv)
            .map_err(|_| CoreError::Parse("密钥或 IV 长度不正确".into()))?
            .encrypt_padded_vec_mut::<Pkcs7>(data))
    } else {
        cbc::Decryptor::<Bf>::new_from_slices(key, iv)
            .map_err(|_| CoreError::Parse("密钥或 IV 长度不正确".into()))?
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .map_err(|_| CoreError::Parse("解密失败：密文或填充无效".into()))
    }
}

fn ecb(enc: bool, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if enc {
        Ok(ecb::Encryptor::<Bf>::new_from_slice(key)
            .map_err(|_| CoreError::Parse("密钥长度不正确".into()))?
            .encrypt_padded_vec_mut::<Pkcs7>(data))
    } else {
        ecb::Decryptor::<Bf>::new_from_slice(key)
            .map_err(|_| CoreError::Parse("密钥长度不正确".into()))?
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .map_err(|_| CoreError::Parse("解密失败：密文或填充无效".into()))
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key = parse_bytes(pstr(params, "key", ""), pstr(params, "keyFormat", "Hex"))?;
        let iv = parse_bytes(pstr(params, "iv", ""), pstr(params, "ivFormat", "Hex"))?;
        let data = parse_bytes(
            in_text(inputs, "text")?,
            pstr(params, "inputFormat", "UTF8"),
        )?;
        let enc = pstr(params, "operation", "加密") != "解密";
        let out = if pstr(params, "mode", "CBC") == "ECB" {
            ecb(enc, &key, &data)?
        } else {
            cbc(enc, &key, &iv, &data)?
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
            "blowfish",
            CRYPTO,
            "Blowfish",
            ROSE,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select("operation", "操作", &["加密", "解密"], "加密"),
                ParamSpec::select("mode", "模式", &["CBC", "ECB"], "CBC"),
                ParamSpec::text("key", "密钥", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::text("iv", "IV", "", false),
                ParamSpec::select("ivFormat", "IV 格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::select(
                    "inputFormat",
                    "输入格式",
                    &["UTF8", "Hex", "Base64"],
                    "UTF8",
                ),
                ParamSpec::select(
                    "outputFormat",
                    "输出格式",
                    &["Hex", "Base64", "UTF8"],
                    "Hex",
                ),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
