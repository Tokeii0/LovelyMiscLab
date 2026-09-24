//! Message-digest (hash) and HMAC nodes. Output is lowercase hex.
use super::prelude::*;

use digest::Digest;
use hmac::{Hmac, Mac};

fn digest_hex<D: Digest>(data: &[u8]) -> String {
    let mut h = D::new();
    h.update(data);
    hex::encode(h.finalize())
}

pub(crate) fn hash_hex(algo: &str, data: &[u8]) -> Result<String, CoreError> {
    Ok(match algo {
        "MD5" => digest_hex::<md5::Md5>(data),
        "MD4" => digest_hex::<md4::Md4>(data),
        "SHA1" => digest_hex::<sha1::Sha1>(data),
        "SHA224" => digest_hex::<sha2::Sha224>(data),
        "SHA256" => digest_hex::<sha2::Sha256>(data),
        "SHA384" => digest_hex::<sha2::Sha384>(data),
        "SHA512" => digest_hex::<sha2::Sha512>(data),
        "SHA3-256" => digest_hex::<sha3::Sha3_256>(data),
        "SHA3-512" => digest_hex::<sha3::Sha3_512>(data),
        "Keccak-256" => digest_hex::<sha3::Keccak256>(data),
        "RIPEMD-160" => digest_hex::<ripemd::Ripemd160>(data),
        "BLAKE2b" => digest_hex::<blake2::Blake2b512>(data),
        "BLAKE2s" => digest_hex::<blake2::Blake2s256>(data),
        "Whirlpool" => digest_hex::<whirlpool::Whirlpool>(data),
        "SM3" => digest_hex::<sm3::Sm3>(data),
        "CRC32" => {
            let mut h = crc32fast::Hasher::new();
            h.update(data);
            format!("{:08x}", h.finalize())
        }
        "Adler-32" => {
            let (mut a, mut b) = (1u32, 0u32);
            for &byte in data {
                a = (a + byte as u32) % 65521;
                b = (b + a) % 65521;
            }
            format!("{:08x}", (b << 16) | a)
        }
        other => return Err(CoreError::Parse(format!("未知哈希算法: {other}"))),
    })
}

macro_rules! hmac_hex {
    ($t:ty, $key:expr, $data:expr) => {{
        let mut m = <Hmac<$t>>::new_from_slice($key).expect("HMAC accepts any key length");
        m.update($data);
        hex::encode(m.finalize().into_bytes())
    }};
}

struct HashNode;
impl Node for HashNode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        Ok(out_text(hash_hex(
            pstr(params, "algorithm", "SHA256"),
            &data,
        )?))
    }
}

struct HmacNode;
impl Node for HmacNode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let key = parse_bytes(pstr(params, "key", ""), pstr(params, "keyFormat", "UTF8"))?;
        let out = match pstr(params, "algorithm", "SHA256") {
            "MD5" => hmac_hex!(md5::Md5, &key, &data),
            "SHA1" => hmac_hex!(sha1::Sha1, &key, &data),
            "SHA256" => hmac_hex!(sha2::Sha256, &key, &data),
            "SHA512" => hmac_hex!(sha2::Sha512, &key, &data),
            other => return Err(CoreError::Parse(format!("未知 HMAC 算法: {other}"))),
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "hash",
            HASH,
            "哈希计算",
            CYAN,
            vec![req("data", "输入", PortType::Any)],
            vec![req("text", "摘要(hex)", PortType::Text)],
            vec![ParamSpec::select(
                "algorithm",
                "算法",
                &[
                    "MD5",
                    "MD4",
                    "SHA1",
                    "SHA224",
                    "SHA256",
                    "SHA384",
                    "SHA512",
                    "SHA3-256",
                    "SHA3-512",
                    "Keccak-256",
                    "RIPEMD-160",
                    "BLAKE2b",
                    "BLAKE2s",
                    "Whirlpool",
                    "SM3",
                    "CRC32",
                    "Adler-32",
                ],
                "SHA256",
            )],
        ),
        Arc::new(|| Arc::new(HashNode)),
    );
    reg.register(
        desc(
            "hmac",
            HASH,
            "HMAC",
            CYAN,
            vec![req("data", "输入", PortType::Any)],
            vec![req("text", "摘要(hex)", PortType::Text)],
            vec![
                ParamSpec::select(
                    "algorithm",
                    "算法",
                    &["SHA256", "SHA1", "MD5", "SHA512"],
                    "SHA256",
                ),
                ParamSpec::text("key", "密钥", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["UTF8", "Hex", "Base64"], "UTF8"),
            ],
        ),
        Arc::new(|| Arc::new(HmacNode)),
    );
}
