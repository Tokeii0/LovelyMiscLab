//! Extra executable forensics: PE resources/certificates/imphash, section
//! entropy, packer hints, and .NET CLR metadata.

use std::collections::HashSet;

use digest::Digest;
use goblin::pe::section_table::SectionTable;
use goblin::pe::PE;
use goblin::Object;
use serde_json::json;

use super::bin_common::{parse, pe_perms};
use super::prelude::*;

fn parse_pe(data: &[u8]) -> Result<PE<'_>, CoreError> {
    match parse(data)? {
        Object::PE(pe) => Ok(pe),
        _ => Err(CoreError::Unsupported("该节点仅支持 PE 文件".into())),
    }
}

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(o)?, *d.get(o + 1)?]))
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
}

fn shannon(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut h = 0.0;
    for &c in &freq {
        if c > 0 {
            let p = c as f64 / len;
            h -= p * p.log2();
        }
    }
    h
}

fn pe_section_name(s: &SectionTable) -> String {
    s.name().unwrap_or("").trim_end_matches('\0').to_string()
}

fn section_rva_span(s: &SectionTable) -> (u64, u64) {
    let start = s.virtual_address as u64;
    let size = s.virtual_size.max(s.size_of_raw_data) as u64;
    (start, start.saturating_add(size))
}

fn rva_to_offset(sections: &[SectionTable], rva: u32) -> Option<usize> {
    let rva = rva as u64;
    for s in sections {
        let (start, end) = section_rva_span(s);
        if rva >= start && rva < end {
            let delta = rva - start;
            return Some((s.pointer_to_raw_data as u64 + delta) as usize);
        }
    }
    None
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = sha2::Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

// ---------------------------------------------------------------- PE resources

const RESOURCE_NAME_IS_STRING: u32 = 0x8000_0000;
const RESOURCE_DATA_IS_DIRECTORY: u32 = 0x8000_0000;
const RESOURCE_MASK: u32 = 0x7fff_ffff;

fn resource_type_name(id: u16) -> &'static str {
    match id {
        1 => "RT_CURSOR",
        2 => "RT_BITMAP",
        3 => "RT_ICON",
        4 => "RT_MENU",
        5 => "RT_DIALOG",
        6 => "RT_STRING",
        7 => "RT_FONTDIR",
        8 => "RT_FONT",
        9 => "RT_ACCELERATOR",
        10 => "RT_RCDATA",
        11 => "RT_MESSAGETABLE",
        12 => "RT_GROUP_CURSOR",
        14 => "RT_GROUP_ICON",
        16 => "RT_VERSION",
        17 => "RT_DLGINCLUDE",
        19 => "RT_PLUGPLAY",
        20 => "RT_VXD",
        21 => "RT_ANICURSOR",
        22 => "RT_ANIICON",
        23 => "RT_HTML",
        24 => "RT_MANIFEST",
        _ => "RT_UNKNOWN",
    }
}

fn read_resource_string(root: &[u8], off: usize) -> Option<String> {
    let len = le16(root, off)? as usize;
    let start = off + 2;
    let mut words = Vec::with_capacity(len);
    for i in 0..len {
        words.push(le16(root, start + i * 2)?);
    }
    Some(String::from_utf16_lossy(&words))
}

fn resource_entry_name(root: &[u8], name_or_id: u32, depth: usize) -> String {
    if name_or_id & RESOURCE_NAME_IS_STRING != 0 {
        let off = (name_or_id & RESOURCE_MASK) as usize;
        return read_resource_string(root, off).unwrap_or_else(|| format!("name@0x{off:x}"));
    }
    let id = name_or_id as u16;
    if depth == 0 {
        format!("{}({id})", resource_type_name(id))
    } else {
        id.to_string()
    }
}

#[derive(Debug, Clone)]
struct ResourceLeaf {
    path: Vec<String>,
    data_rva: u32,
    offset: Option<usize>,
    size: u32,
    code_page: u32,
}

fn collect_resources(
    root: &[u8],
    sections: &[SectionTable],
    dir_off: usize,
    depth: usize,
    path: &mut Vec<String>,
    seen: &mut HashSet<usize>,
    out: &mut Vec<ResourceLeaf>,
) {
    if depth > 8 || !seen.insert(dir_off) || dir_off + 16 > root.len() || out.len() >= 4096 {
        return;
    }
    let named = le16(root, dir_off + 12).unwrap_or(0) as usize;
    let ids = le16(root, dir_off + 14).unwrap_or(0) as usize;
    let count = named.saturating_add(ids).min(4096);
    let entries = dir_off + 16;
    for idx in 0..count {
        let off = entries + idx * 8;
        if off + 8 > root.len() {
            break;
        }
        let name_or_id = le32(root, off).unwrap_or(0);
        let target = le32(root, off + 4).unwrap_or(0);
        path.push(resource_entry_name(root, name_or_id, depth));
        if target & RESOURCE_DATA_IS_DIRECTORY != 0 {
            collect_resources(
                root,
                sections,
                (target & RESOURCE_MASK) as usize,
                depth + 1,
                path,
                seen,
                out,
            );
        } else {
            let data_entry = (target & RESOURCE_MASK) as usize;
            if data_entry + 16 <= root.len() {
                let data_rva = le32(root, data_entry).unwrap_or(0);
                let size = le32(root, data_entry + 4).unwrap_or(0);
                let code_page = le32(root, data_entry + 8).unwrap_or(0);
                out.push(ResourceLeaf {
                    path: path.clone(),
                    data_rva,
                    offset: rva_to_offset(sections, data_rva),
                    size,
                    code_page,
                });
            }
        }
        path.pop();
    }
}

fn pe_resource_leaves(data: &[u8], pe: &PE<'_>) -> Result<Vec<ResourceLeaf>, CoreError> {
    let Some(optional) = pe.header.optional_header.as_ref() else {
        return Ok(Vec::new());
    };
    let Some(dir) = optional.data_directories.get_resource_table() else {
        return Ok(Vec::new());
    };
    if dir.virtual_address == 0 || dir.size == 0 {
        return Ok(Vec::new());
    }
    let Some(root_off) = rva_to_offset(&pe.sections, dir.virtual_address) else {
        return Err(CoreError::Parse(format!(
            "无法映射资源表 RVA 0x{:x}",
            dir.virtual_address
        )));
    };
    let end = root_off.saturating_add(dir.size as usize).min(data.len());
    if root_off >= end {
        return Ok(Vec::new());
    }
    let root = &data[root_off..end];
    let mut out = Vec::new();
    collect_resources(
        root,
        &pe.sections,
        0,
        0,
        &mut Vec::new(),
        &mut HashSet::new(),
        &mut out,
    );
    Ok(out)
}

struct PeResources;

impl Node for PeResources {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let pe = parse_pe(&data)?;
        let leaves = pe_resource_leaves(&data, &pe)?;

        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut names = Vec::new();
        for r in &leaves {
            let path = r.path.join("/");
            names.push(path.clone());
            rows.push(format!(
                "{path:<42} rva=0x{:08x} off={} size={} codepage={}",
                r.data_rva,
                r.offset
                    .map(|o| format!("0x{o:08x}"))
                    .unwrap_or_else(|| "-".into()),
                r.size,
                r.code_page
            ));
            arr.push(json!({
                "path": path,
                "dataRva": r.data_rva,
                "offset": r.offset,
                "size": r.size,
                "codePage": r.code_page,
            }));
        }

        let text = if rows.is_empty() {
            "（无资源目录或无叶子资源）".to_string()
        } else {
            rows.join("\n")
        };
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("resources".into(), PortValue::StringList(names));
        m.insert("count".into(), PortValue::Number(leaves.len() as f64));
        Ok(m)
    }
}

// ------------------------------------------------------------- PE certs

struct PeCertificates;

impl Node for PeCertificates {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let pe = parse_pe(&data)?;
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        for (idx, cert) in pe.certificates.iter().enumerate() {
            let digest = sha256_hex(cert.certificate);
            rows.push(format!(
                "#{idx} len={} data={} revision={:?} type={:?} sha256={}",
                cert.length,
                cert.certificate.len(),
                cert.revision,
                cert.certificate_type,
                digest
            ));
            arr.push(json!({
                "index": idx,
                "length": cert.length,
                "dataLength": cert.certificate.len(),
                "revision": format!("{:?}", cert.revision),
                "type": format!("{:?}", cert.certificate_type),
                "sha256": digest,
            }));
        }
        let text = if rows.is_empty() {
            "（无证书表）".to_string()
        } else {
            rows.join("\n")
        };
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert(
            "signed".into(),
            PortValue::Bool(!pe.certificates.is_empty()),
        );
        m.insert(
            "count".into(),
            PortValue::Number(pe.certificates.len() as f64),
        );
        Ok(m)
    }
}

// ------------------------------------------------------------- PE imphash

fn normalize_import_dll(dll: &str) -> String {
    let lower = dll.to_ascii_lowercase();
    for suffix in [".dll", ".ocx", ".sys"] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return stripped.to_string();
        }
    }
    lower
}

fn pe_imphash_parts(pe: &PE<'_>) -> Vec<String> {
    pe.imports
        .iter()
        .map(|im| {
            let dll = normalize_import_dll(im.dll);
            let name = if im.name.starts_with("ORDINAL ") {
                format!("ordinal{}", im.ordinal)
            } else {
                im.name.to_ascii_lowercase()
            };
            format!("{dll}.{name}")
        })
        .collect()
}

struct PeImphash;

impl Node for PeImphash {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let pe = parse_pe(&data)?;
        let parts = pe_imphash_parts(&pe);
        let joined = parts.join(",");
        let mut h = md5::Md5::new();
        h.update(joined.as_bytes());
        let hash = hex::encode(h.finalize());
        let text = format!("imphash: {hash}\nimports: {}\n{joined}", parts.len());

        let mut m = PortMap::new();
        m.insert("hash".into(), PortValue::Text(hash.clone()));
        m.insert("text".into(), PortValue::Text(text));
        m.insert("imports".into(), PortValue::StringList(parts.clone()));
        m.insert("count".into(), PortValue::Number(parts.len() as f64));
        m.insert(
            "json".into(),
            PortValue::Json(json!({ "imphash": hash, "imports": parts })),
        );
        Ok(m)
    }
}

// ------------------------------------------------------------- Section entropy

struct SectionEntropy;

impl Node for SectionEntropy {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut names = Vec::new();

        match parse(&data)? {
            Object::PE(pe) => {
                for s in &pe.sections {
                    let name = pe_section_name(s);
                    let start = s.pointer_to_raw_data as usize;
                    let end = start
                        .saturating_add(s.size_of_raw_data as usize)
                        .min(data.len());
                    let bytes = data.get(start..end).unwrap_or(&[]);
                    let entropy = shannon(bytes);
                    let high = entropy >= 7.2 && bytes.len() >= 512;
                    names.push(name.clone());
                    rows.push(format!(
                        "{name:<12} entropy={entropy:.3} raw={} virt={} off=0x{:08x} {}",
                        bytes.len(),
                        s.virtual_size,
                        s.pointer_to_raw_data,
                        if high { "HIGH" } else { "" }
                    ));
                    arr.push(json!({
                        "name": name,
                        "entropy": entropy,
                        "rawSize": bytes.len(),
                        "virtualSize": s.virtual_size,
                        "offset": s.pointer_to_raw_data,
                        "high": high,
                    }));
                }
            }
            Object::Elf(e) => {
                for sh in &e.section_headers {
                    let name = e.shdr_strtab.get_at(sh.sh_name).unwrap_or("").to_string();
                    let start = sh.sh_offset as usize;
                    let end = start.saturating_add(sh.sh_size as usize).min(data.len());
                    let bytes = data.get(start..end).unwrap_or(&[]);
                    let entropy = shannon(bytes);
                    let high = entropy >= 7.2 && bytes.len() >= 512;
                    names.push(name.clone());
                    rows.push(format!(
                        "{name:<16} entropy={entropy:.3} size={} off=0x{:08x} {}",
                        bytes.len(),
                        sh.sh_offset,
                        if high { "HIGH" } else { "" }
                    ));
                    arr.push(json!({
                        "name": name,
                        "entropy": entropy,
                        "size": bytes.len(),
                        "offset": sh.sh_offset,
                        "high": high,
                    }));
                }
            }
            _ => return Err(CoreError::Unsupported("节区熵目前支持 PE/ELF".into())),
        }

        let text = if rows.is_empty() {
            "（无节区）".to_string()
        } else {
            rows.join("\n")
        };
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("names".into(), PortValue::StringList(names));
        Ok(m)
    }
}

// ------------------------------------------------------------- Packer hints

fn entry_section_name(pe: &PE<'_>) -> Option<String> {
    pe.sections.iter().find_map(|s| {
        let (start, end) = section_rva_span(s);
        ((pe.entry as u64) >= start && (pe.entry as u64) < end).then(|| pe_section_name(s))
    })
}

struct PePackerHints;

impl Node for PePackerHints {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let pe = parse_pe(&data)?;
        let mut hints = Vec::new();
        let mut score = 0.0;

        let packer_names = [
            "upx",
            "aspack",
            "mpress",
            "themida",
            "vmp",
            "vmprotect",
            "fsg",
            "petite",
            "pec2",
            "pecompact",
            "enigma",
        ];
        for s in &pe.sections {
            let name = pe_section_name(s);
            let lower = name.to_ascii_lowercase();
            if packer_names.iter().any(|p| lower.contains(p)) {
                hints.push(format!("可疑节区名：{name}"));
                score += 3.0;
            }
            let start = s.pointer_to_raw_data as usize;
            let end = start
                .saturating_add(s.size_of_raw_data as usize)
                .min(data.len());
            let bytes = data.get(start..end).unwrap_or(&[]);
            let entropy = shannon(bytes);
            if entropy >= 7.2 && bytes.len() >= 512 {
                hints.push(format!("{name} 高熵 {entropy:.2}"));
                score += 1.5;
            }
            let perms = pe_perms(s.characteristics);
            if perms.contains('w') && perms.contains('x') {
                hints.push(format!("{name} 同时可写可执行 ({perms})"));
                score += 1.5;
            }
            if s.size_of_raw_data == 0 && s.virtual_size > 0 {
                hints.push(format!("{name} 原始大小为 0 但虚拟大小非 0"));
                score += 1.0;
            }
        }

        if let Some(ep) = entry_section_name(&pe) {
            if let Some(last) = pe.sections.last().map(pe_section_name) {
                if ep == last && pe.sections.len() > 1 {
                    hints.push(format!("入口点位于最后一个节区：{ep}"));
                    score += 1.0;
                }
            }
        }
        if pe.imports.len() <= 3 {
            hints.push(format!("导入函数很少：{}", pe.imports.len()));
            score += 0.5;
        }
        if pe.tls_data.is_some() {
            hints.push("存在 TLS 目录，需检查 TLS callback".into());
            score += 0.5;
        }
        let raw_end = pe
            .sections
            .iter()
            .map(|s| (s.pointer_to_raw_data as usize).saturating_add(s.size_of_raw_data as usize))
            .max()
            .unwrap_or(0);
        if data.len() > raw_end.saturating_add(4096) {
            hints.push(format!("存在较大 overlay：{} bytes", data.len() - raw_end));
            score += 0.5;
        }

        let packed = score >= 3.0;
        let text = if hints.is_empty() {
            "未发现明显壳特征".to_string()
        } else {
            format!(
                "score: {score:.1}\npackedHint: {packed}\n{}",
                hints.join("\n")
            )
        };
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert(
            "json".into(),
            PortValue::Json(json!({ "score": score, "packed": packed, "hints": hints })),
        );
        m.insert("hints".into(), PortValue::StringList(hints));
        m.insert("packed".into(), PortValue::Bool(packed));
        m.insert("score".into(), PortValue::Number(score));
        Ok(m)
    }
}

// ------------------------------------------------------------- .NET metadata

fn clr_flags_text(flags: u32) -> Vec<&'static str> {
    let mut out = Vec::new();
    if flags & 0x0000_0001 != 0 {
        out.push("ILONLY");
    }
    if flags & 0x0000_0002 != 0 {
        out.push("32BITREQUIRED");
    }
    if flags & 0x0000_0004 != 0 {
        out.push("IL_LIBRARY");
    }
    if flags & 0x0000_0008 != 0 {
        out.push("STRONGNAMESIGNED");
    }
    if flags & 0x0000_0010 != 0 {
        out.push("NATIVE_ENTRYPOINT");
    }
    if flags & 0x0001_0000 != 0 {
        out.push("TRACKDEBUGDATA");
    }
    if flags & 0x0002_0000 != 0 {
        out.push("32BITPREFERRED");
    }
    out
}

struct DotnetMetadata;

impl Node for DotnetMetadata {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let pe = parse_pe(&data)?;
        let Some(clr) = pe.clr_data.as_ref() else {
            let mut m = PortMap::new();
            m.insert(
                "text".into(),
                PortValue::Text("（无 CLR/.NET 元数据）".into()),
            );
            m.insert("json".into(), PortValue::Json(json!({ "managed": false })));
            m.insert("managed".into(), PortValue::Bool(false));
            m.insert("streams".into(), PortValue::StringList(Vec::new()));
            return Ok(m);
        };

        let flags = clr_flags_text(clr.cor20_header.flags);
        let mut streams = Vec::new();
        let mut stream_json = Vec::new();
        for s in clr.sections().flatten() {
            streams.push(s.name.to_string());
            stream_json.push(json!({ "name": s.name, "offset": s.offset, "size": s.size }));
        }
        let mvid = clr.mvid().ok().flatten().map(hex::encode);

        let mut rows = Vec::new();
        rows.push(format!(
            "CLR runtime: {}.{}",
            clr.cor20_header.major_runtime_version, clr.cor20_header.minor_runtime_version
        ));
        rows.push(format!("metadata version: {}", clr.metadata_header.version));
        rows.push(format!("signature valid: {}", clr.is_valid()));
        rows.push(format!(
            "flags: 0x{:08x} {}",
            clr.cor20_header.flags,
            flags.join("|")
        ));
        rows.push(format!(
            "entry point token/RVA: 0x{:08x}",
            clr.cor20_header.entry_point_token_or_rva
        ));
        if let Some(id) = &mvid {
            rows.push(format!("MVID: {id}"));
        }
        if !streams.is_empty() {
            rows.push(format!("streams: {}", streams.join(", ")));
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert(
            "json".into(),
            PortValue::Json(json!({
                "managed": true,
                "signatureValid": clr.is_valid(),
                "runtimeMajor": clr.cor20_header.major_runtime_version,
                "runtimeMinor": clr.cor20_header.minor_runtime_version,
                "metadataVersion": clr.metadata_header.version,
                "flags": clr.cor20_header.flags,
                "flagNames": flags,
                "entryPoint": clr.cor20_header.entry_point_token_or_rva,
                "mvid": mvid,
                "streams": stream_json,
            })),
        );
        m.insert("managed".into(), PortValue::Bool(true));
        m.insert("streams".into(), PortValue::StringList(streams));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pe_resources",
            BIN,
            "PE 资源表",
            INDIGO,
            vec![req("data", "PE 字节", PortType::Any)],
            vec![
                req("text", "资源列表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("resources", "资源路径", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PeResources)),
    );
    reg.register(
        desc(
            "pe_certificates",
            BIN,
            "PE 证书表",
            INDIGO,
            vec![req("data", "PE 字节", PortType::Any)],
            vec![
                req("text", "证书列表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("signed", "已签名", PortType::Bool),
                opt("count", "数量", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PeCertificates)),
    );
    reg.register(
        desc(
            "pe_imphash",
            BIN,
            "PE imphash",
            INDIGO,
            vec![req("data", "PE 字节", PortType::Any)],
            vec![
                req("hash", "imphash", PortType::Text),
                opt("text", "详情", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("imports", "导入项", PortType::StringList),
                opt("count", "导入数", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PeImphash)),
    );
    reg.register(
        desc(
            "section_entropy",
            BIN,
            "节区熵",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "熵表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("names", "节区名", PortType::StringList),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(SectionEntropy)),
    );
    reg.register(
        desc(
            "pe_packer_hints",
            BIN,
            "PE 壳特征提示",
            INDIGO,
            vec![req("data", "PE 字节", PortType::Any)],
            vec![
                req("text", "提示", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("hints", "提示列表", PortType::StringList),
                opt("packed", "疑似加壳", PortType::Bool),
                opt("score", "分数", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PePackerHints)),
    );
    reg.register(
        desc(
            "dotnet_metadata",
            BIN,
            ".NET 元数据",
            INDIGO,
            vec![req("data", "PE 字节", PortType::Any)],
            vec![
                req("text", "摘要", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("managed", "托管程序集", PortType::Bool),
                opt("streams", "Metadata Streams", PortType::StringList),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(DotnetMetadata)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn put_u16(buf: &mut [u8], off: usize, n: u16) {
        buf[off..off + 2].copy_from_slice(&n.to_le_bytes());
    }

    fn put_u32(buf: &mut [u8], off: usize, n: u32) {
        buf[off..off + 4].copy_from_slice(&n.to_le_bytes());
    }

    fn make_minimal_pe(upx_names: bool) -> Vec<u8> {
        let mut pe = vec![0u8; 0x600];
        pe[0..2].copy_from_slice(b"MZ");
        put_u32(&mut pe, 0x3c, 0x80);
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        put_u16(&mut pe, coff, 0x14c);
        put_u16(&mut pe, coff + 2, 2);
        put_u16(&mut pe, coff + 16, 0xe0);
        put_u16(&mut pe, coff + 18, 0x010f);

        let opt = coff + 20;
        put_u16(&mut pe, opt, 0x10b);
        put_u32(&mut pe, opt + 16, 0x2000);
        put_u32(&mut pe, opt + 20, 0x1000);
        put_u32(&mut pe, opt + 24, 0x2000);
        put_u32(&mut pe, opt + 28, 0x400000);
        put_u32(&mut pe, opt + 32, 0x1000);
        put_u32(&mut pe, opt + 36, 0x200);
        put_u32(&mut pe, opt + 56, 0x3000);
        put_u32(&mut pe, opt + 60, 0x200);
        put_u16(&mut pe, opt + 68, 3);
        put_u32(&mut pe, opt + 72, 0x100000);
        put_u32(&mut pe, opt + 76, 0x1000);
        put_u32(&mut pe, opt + 80, 0x100000);
        put_u32(&mut pe, opt + 84, 0x1000);
        put_u32(&mut pe, opt + 92, 16);

        let sec = opt + 0xe0;
        let names = if upx_names {
            (*b"UPX0\0\0\0\0", *b"UPX1\0\0\0\0")
        } else {
            (*b".text\0\0\0", *b".data\0\0\0")
        };
        pe[sec..sec + 8].copy_from_slice(&names.0);
        put_u32(&mut pe, sec + 8, 0x1000);
        put_u32(&mut pe, sec + 12, 0x1000);
        put_u32(&mut pe, sec + 16, 0x200);
        put_u32(&mut pe, sec + 20, 0x200);
        put_u32(&mut pe, sec + 36, 0x60000020);

        let sec2 = sec + 40;
        pe[sec2..sec2 + 8].copy_from_slice(&names.1);
        put_u32(&mut pe, sec2 + 8, 0x1000);
        put_u32(&mut pe, sec2 + 12, 0x2000);
        put_u32(&mut pe, sec2 + 16, 0x200);
        put_u32(&mut pe, sec2 + 20, 0x400);
        put_u32(&mut pe, sec2 + 36, 0xe0000020);

        for i in 0..0x200 {
            pe[0x400 + i] = (i.wrapping_mul(37) & 0xff) as u8;
        }
        pe
    }

    fn run(id: &str, data: Vec<u8>) -> PortMap {
        let mut i = PortMap::new();
        i.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    #[test]
    fn section_entropy_reads_minimal_pe() {
        let out = run("section_entropy", make_minimal_pe(false));
        assert!(
            matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains(".text") && t.contains(".data"))
        );
    }

    #[test]
    fn packer_hints_flags_upx_sections() {
        let out = run("pe_packer_hints", make_minimal_pe(true));
        assert!(matches!(out.get("packed"), Some(PortValue::Bool(true))));
        assert!(matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains("UPX")));
    }

    #[test]
    fn pe_empty_tables_are_graceful() {
        let pe = make_minimal_pe(false);
        let cert = run("pe_certificates", pe.clone());
        assert!(matches!(cert.get("count"), Some(PortValue::Number(n)) if *n == 0.0));
        let res = run("pe_resources", pe.clone());
        assert!(matches!(res.get("count"), Some(PortValue::Number(n)) if *n == 0.0));
        let imp = run("pe_imphash", pe);
        assert!(
            matches!(imp.get("hash"), Some(PortValue::Text(h)) if h == "d41d8cd98f00b204e9800998ecf8427e")
        );
    }
}
