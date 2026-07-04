//! Rabbit 流密码（eSTREAM / RFC 4503）。手写实现——`crates.io` 无现成 crate。
//! 128 位密钥 + 可选 64 位 IV，逐 128 位块生成密钥流并 XOR。密钥/IV 按 RFC 参考实现的
//! **小端**字节序装载（`key[0]` 为最低字节）。加解密对称（都是 XOR 密钥流）。
//! 正确性由 RFC 4503 附录 A 的全部 6 组测试向量比对保证（见文件末测试）。
use super::prelude::*;

const A: [u32; 8] = [
    0x4D34D34D, 0xD34D34D3, 0x34D34D34, 0x4D34D34D, 0xD34D34D3, 0x34D34D34, 0x4D34D34D, 0xD34D34D3,
];

struct Rabbit {
    x: [u32; 8],
    c: [u32; 8],
    carry: u32,
}

impl Rabbit {
    fn new(key: &[u8; 16]) -> Self {
        // 子密钥 k_i = K[16i+15 .. 16i]，其中 128 位密钥按大端（key[0] 为最高字节）解释，
        // 故 k0 取末两字节、k7 取首两字节，各自大端 16 位。（对拍 RFC 4503 附录 B 调试向量）
        let k: [u32; 8] =
            std::array::from_fn(|i| ((key[14 - 2 * i] as u32) << 8) | (key[15 - 2 * i] as u32));
        let mut r = Rabbit {
            x: [0; 8],
            c: [0; 8],
            carry: 0,
        };
        for j in 0..8 {
            if j % 2 == 0 {
                r.x[j] = (k[(j + 1) % 8] << 16) | k[j];
                r.c[j] = (k[(j + 4) % 8] << 16) | k[(j + 5) % 8];
            } else {
                r.x[j] = (k[(j + 5) % 8] << 16) | k[(j + 4) % 8];
                r.c[j] = (k[j] << 16) | k[(j + 1) % 8];
            }
        }
        for _ in 0..4 {
            r.next_state();
        }
        for j in 0..8 {
            r.c[j] ^= r.x[(j + 4) % 8];
        }
        r
    }

    fn iv_setup(&mut self, iv: &[u8; 8]) {
        // IV 同样按大端解释：IV[63..0]，iv[0] 为最高字节。
        let iv_lo = u32::from_be_bytes([iv[4], iv[5], iv[6], iv[7]]); // IV[31..0]
        let iv_hi = u32::from_be_bytes([iv[0], iv[1], iv[2], iv[3]]); // IV[63..32]
        let w1 = (iv_hi & 0xFFFF_0000) | (iv_lo >> 16); // IV[63..48] || IV[31..16]
        let w3 = ((iv_hi & 0xFFFF) << 16) | (iv_lo & 0xFFFF); // IV[47..32] || IV[15..0]
        self.c[0] ^= iv_lo;
        self.c[1] ^= w1;
        self.c[2] ^= iv_hi;
        self.c[3] ^= w3;
        self.c[4] ^= iv_lo;
        self.c[5] ^= w1;
        self.c[6] ^= iv_hi;
        self.c[7] ^= w3;
        for _ in 0..4 {
            self.next_state();
        }
    }

    fn next_state(&mut self) {
        // 计数器系统（带进位链）。
        let mut carry = self.carry;
        for (i, &a) in A.iter().enumerate() {
            let t = self.c[i] as u64 + a as u64 + carry as u64;
            self.c[i] = t as u32;
            carry = (t >> 32) as u32;
        }
        self.carry = carry;
        // g 函数：((x+c)^2) 的高 32 ^ 低 32。
        let g: [u32; 8] = std::array::from_fn(|i| {
            let uv = self.x[i].wrapping_add(self.c[i]) as u64;
            let sq = uv.wrapping_mul(uv);
            ((sq >> 32) ^ (sq & 0xFFFF_FFFF)) as u32
        });
        // 状态更新（RFC §2.5）。
        self.x[0] = g[0]
            .wrapping_add(g[7].rotate_left(16))
            .wrapping_add(g[6].rotate_left(16));
        self.x[1] = g[1].wrapping_add(g[0].rotate_left(8)).wrapping_add(g[7]);
        self.x[2] = g[2]
            .wrapping_add(g[1].rotate_left(16))
            .wrapping_add(g[0].rotate_left(16));
        self.x[3] = g[3].wrapping_add(g[2].rotate_left(8)).wrapping_add(g[1]);
        self.x[4] = g[4]
            .wrapping_add(g[3].rotate_left(16))
            .wrapping_add(g[2].rotate_left(16));
        self.x[5] = g[5].wrapping_add(g[4].rotate_left(8)).wrapping_add(g[3]);
        self.x[6] = g[6]
            .wrapping_add(g[5].rotate_left(16))
            .wrapping_add(g[4].rotate_left(16));
        self.x[7] = g[7].wrapping_add(g[6].rotate_left(8)).wrapping_add(g[5]);
    }

    /// 提取 128 位密钥流块（16 字节）。RFC 4503 把 S[127..0] 按大端序列化：
    /// 字节 0 = S 的最高字节，故输出为 b3‖b2‖b1‖b0（各自大端），与附录 A 向量一致。
    fn extract(&self) -> [u8; 16] {
        let b0 = self.x[0] ^ (self.x[5] >> 16) ^ (self.x[3] << 16); // S[31..0]
        let b1 = self.x[2] ^ (self.x[7] >> 16) ^ (self.x[5] << 16); // S[63..32]
        let b2 = self.x[4] ^ (self.x[1] >> 16) ^ (self.x[7] << 16); // S[95..64]
        let b3 = self.x[6] ^ (self.x[3] >> 16) ^ (self.x[1] << 16); // S[127..96]
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&b3.to_be_bytes());
        out[4..8].copy_from_slice(&b2.to_be_bytes());
        out[8..12].copy_from_slice(&b1.to_be_bytes());
        out[12..16].copy_from_slice(&b0.to_be_bytes());
        out
    }

    /// 每块先 next_state 再提取，与明文 XOR。
    fn apply(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        for chunk in data.chunks(16) {
            self.next_state();
            let ks = self.extract();
            for (j, &byte) in chunk.iter().enumerate() {
                out.push(byte ^ ks[j]);
            }
        }
        out
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key = parse_bytes(pstr(p, "key", ""), pstr(p, "keyFormat", "Hex"))?;
        if key.len() != 16 {
            return Err(CoreError::Parse(format!(
                "Rabbit 密钥须为 16 字节（当前 {}）",
                key.len()
            )));
        }
        let iv = parse_bytes(pstr(p, "iv", ""), pstr(p, "ivFormat", "Hex"))?;
        if !(iv.is_empty() || iv.len() == 8) {
            return Err(CoreError::Parse(format!(
                "IV 须为 0 或 8 字节（当前 {}）",
                iv.len()
            )));
        }
        let data = parse_bytes(in_text(i, "text")?, pstr(p, "inputFormat", "UTF8"))?;

        let mut key_arr = [0u8; 16];
        key_arr.copy_from_slice(&key);
        let mut r = Rabbit::new(&key_arr);
        if iv.len() == 8 {
            let mut iv_arr = [0u8; 8];
            iv_arr.copy_from_slice(&iv);
            r.iv_setup(&iv_arr);
        }
        let out = r.apply(&data);

        let text = format_bytes(&out, pstr(p, "outputFormat", "Hex"));
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(out.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "rabbit",
            CRYPTO,
            "Rabbit 流密码",
            ROSE,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("key", "密钥(16字节)", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::text("iv", "IV(0或8字节)", "", false),
                ParamSpec::select("ivFormat", "IV 格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::select("inputFormat", "输入格式", &["UTF8", "Hex", "Base64"], "UTF8"),
                ParamSpec::select("outputFormat", "输出格式", &["Hex", "Base64", "UTF8"], "Hex"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keystream(key: [u8; 16], iv: Option<[u8; 8]>, n: usize) -> Vec<u8> {
        let mut r = Rabbit::new(&key);
        if let Some(iv) = iv {
            r.iv_setup(&iv);
        }
        r.apply(&vec![0u8; n])
    }

    fn h(s: &str) -> Vec<u8> {
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        hex::decode(clean).unwrap()
    }

    // RFC 4503 附录 A.1（无 IV）。
    #[test]
    fn rfc_vectors_without_iv() {
        let cases: &[([u8; 16], &str)] = &[
            (
                [0; 16],
                "B15754F036A5D6ECF56B45261C4AF702\
                 88E8D815C59C0C397B696C4789C68AA7\
                 F416A1C3700CD451DA68D1881673D696",
            ),
            (
                h("912813292E3D36FE3BFC62F1DC51C3AC").try_into().unwrap(),
                "3D2DF3C83EF627A1E97FC38487E2519C\
                 F576CD61F4405B8896BF53AA8554FC19\
                 E5547473FBDB43508AE53B20204D4C5E",
            ),
            (
                h("8395741587E0C733E9E9AB01C09B0043").try_into().unwrap(),
                "0CB10DCDA041CDAC32EB5CFD02D0609B\
                 95FC9FCA0F17015A7B7092114CFF3EAD\
                 9649E5DE8BFC7F3F924147AD3A947428",
            ),
        ];
        for (key, expect) in cases {
            assert_eq!(keystream(*key, None, 48), h(expect), "key={key:02x?}");
        }
    }

    // RFC 4503 附录 A.2（全零密钥 + IV）。
    #[test]
    fn rfc_vectors_with_iv() {
        let cases: &[(&str, &str)] = &[
            (
                "0000000000000000",
                "C6A7275EF85495D87CCD5D376705B7ED\
                 5F29A6AC04F5EFD47B8F293270DC4A8D\
                 2ADE822B29DE6C1EE52BDB8A47BF8F66",
            ),
            (
                "C373F575C1267E59",
                "1FCD4EB9580012E2E0DCCC9222017D6D\
                 A75F4E10D12125017B2499FFED936F2E\
                 EBC112C393E738392356BDD012029BA7",
            ),
            (
                "A6EB561AD2F41727",
                "445AD8C805858DBF70B6AF23A151104D\
                 96C8F27947F42C5BAEAE67C6ACC35B03\
                 9FCBFC895FA71C17313DF034F01551CB",
            ),
        ];
        for (iv, expect) in cases {
            let iv_arr: [u8; 8] = h(iv).try_into().unwrap();
            assert_eq!(keystream([0; 16], Some(iv_arr), 48), h(expect), "iv={iv}");
        }
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = h("0123456789ABCDEF0123456789ABCDEF").try_into().unwrap();
        let iv = Some(h("1122334455667788").try_into().unwrap());
        let pt = b"flag{rabbit_stream_cipher}";
        let ct = {
            let mut r = Rabbit::new(&key);
            if let Some(iv) = iv {
                r.iv_setup(&iv);
            }
            r.apply(pt)
        };
        let back = {
            let mut r = Rabbit::new(&key);
            if let Some(iv) = iv {
                r.iv_setup(&iv);
            }
            r.apply(&ct)
        };
        assert_eq!(back, pt);
        assert_ne!(ct, pt);
    }
}
