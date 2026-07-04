//! 二进制分析节点共用：把字节解析成 `goblin::Object`，并给出格式/架构/类型的中文串。
//! 这是内部模块（不注册为节点）。
#![allow(dead_code)]
use goblin::Object;

use super::prelude::*;

/// 解析为可执行/目标文件对象；非可执行/未知格式给出友好错误。
pub(super) fn parse(data: &[u8]) -> Result<Object<'_>, CoreError> {
    match Object::parse(data) {
        Ok(Object::Unknown(magic)) => Err(CoreError::Unsupported(format!(
            "未知格式（幻数 0x{magic:x}）——不是 ELF / PE / Mach-O"
        ))),
        Ok(obj) => Ok(obj),
        Err(e) => Err(CoreError::Parse(format!("无法解析为可执行文件: {e}"))),
    }
}

/// ELF `e_type` → 中文类型。
pub(super) fn elf_type(e_type: u16) -> &'static str {
    use goblin::elf::header::*;
    match e_type {
        ET_REL => "可重定位(.o)",
        ET_EXEC => "可执行",
        ET_DYN => "共享库/PIE",
        ET_CORE => "core dump",
        _ => "其它",
    }
}

/// PE COFF machine → 架构名（用原始值，避免依赖 goblin 常量命名）。
pub(super) fn pe_machine(m: u16) -> &'static str {
    match m {
        0x8664 => "x86-64",
        0x014c => "x86",
        0xaa64 => "ARM64",
        0x01c0 => "ARM",
        0x01c4 => "ARM Thumb-2",
        0x0200 => "IA-64",
        _ => "其它",
    }
}

/// PE 子系统 → 中文名。
pub(super) fn pe_subsystem(s: u16) -> &'static str {
    match s {
        1 => "Native",
        2 => "Windows GUI",
        3 => "Windows 控制台",
        5 => "OS/2",
        7 => "POSIX",
        9 => "Windows CE",
        10 => "EFI 应用",
        11 => "EFI 引导驱动",
        12 => "EFI 运行时驱动",
        13 => "EFI ROM",
        16 => "Boot 应用",
        _ => "其它/未知",
    }
}

/// PE 节区特征位 → rwx 字符串。
pub(super) fn pe_perms(ch: u32) -> String {
    let r = if ch & 0x4000_0000 != 0 { 'r' } else { '-' };
    let w = if ch & 0x8000_0000 != 0 { 'w' } else { '-' };
    let x = if ch & 0x2000_0000 != 0 { 'x' } else { '-' };
    format!("{r}{w}{x}")
}
