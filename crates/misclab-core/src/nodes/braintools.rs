//! BrainTools：把 PNG 里的 Brainfuck 程序解出来（Brainloller / Braincopter），并可选执行。
//!
//! - **Braincopter**：每像素 `cmd = (R*65536 + G*256 + B) % 11`。
//! - **Brainloller**：每像素按特定颜色映射为命令。
//!
//! 两者都是 2D 语言：指令指针从左上角向右出发，命令 8/9 顺/逆时针转向，10（或未知色）
//! 为 NOP。命令 0..7 依次是 `> < + - . , [ ]`。按指针轨迹线性化为 Brainfuck 源码。
use super::image_util::load_image;
use super::prelude::*;

const OPS: [u8; 8] = [b'>', b'<', b'+', b'-', b'.', b',', b'[', b']'];

/// Brainloller 颜色 → 命令（0..10）。返回 None 视作 NOP。
fn brainloller_cmd(r: u8, g: u8, b: u8) -> Option<usize> {
    Some(match (r, g, b) {
        (255, 0, 0) => 0,   // >
        (128, 0, 0) => 1,   // <
        (0, 255, 0) => 2,   // +
        (0, 128, 0) => 3,   // -
        (0, 0, 255) => 4,   // .
        (0, 0, 128) => 5,   // ,
        (255, 255, 0) => 6, // [
        (128, 128, 0) => 7, // ]
        (0, 255, 255) => 8, // 顺时针
        (0, 128, 128) => 9, // 逆时针
        _ => return None,   // NOP
    })
}

/// 沿 2D 指针轨迹把图像解码为 Brainfuck 源码。
fn decode(img: &image::RgbaImage, braincopter: bool) -> String {
    let (w, h) = (img.width() as i64, img.height() as i64);
    // 方向：东、南、西、北（y 向下）；顺时针 = +1。
    let dirs = [(1i64, 0i64), (0, 1), (-1, 0), (0, -1)];
    let (mut x, mut y, mut dir) = (0i64, 0i64, 0usize);
    let mut out = String::new();
    let max_steps = (w * h) as usize * 8 + 1024; // 防御性上限
    let mut steps = 0usize;
    while x >= 0 && x < w && y >= 0 && y < h && steps < max_steps {
        steps += 1;
        let px = img.get_pixel(x as u32, y as u32).0;
        let cmd = if braincopter {
            Some((px[0] as usize * 65536 + px[1] as usize * 256 + px[2] as usize) % 11)
        } else {
            brainloller_cmd(px[0], px[1], px[2])
        };
        match cmd {
            Some(c) if c < 8 => out.push(OPS[c] as char),
            Some(8) => dir = (dir + 1) % 4,
            Some(9) => dir = (dir + 3) % 4,
            _ => {} // 10 或未知色：NOP
        }
        x += dirs[dir].0;
        y += dirs[dir].1;
    }
    out
}

/// 极简 Brainfuck 解释器（30000 环形单元、字节回绕），带步数上限。
fn run_bf(src: &str, input: &[u8], max_steps: usize) -> Result<Vec<u8>, String> {
    let code: Vec<u8> = src.bytes().filter(|c| OPS.contains(c)).collect();
    // 预匹配括号。
    let mut jumps = vec![0usize; code.len()];
    let mut stack = Vec::new();
    for (i, &c) in code.iter().enumerate() {
        if c == b'[' {
            stack.push(i);
        } else if c == b']' {
            let j = stack.pop().ok_or("方括号不匹配")?;
            jumps[i] = j;
            jumps[j] = i;
        }
    }
    if !stack.is_empty() {
        return Err("方括号不匹配".into());
    }

    let mut tape = vec![0u8; 30000];
    let (mut ptr, mut pc, mut inp, mut steps) = (0usize, 0usize, 0usize, 0usize);
    let mut out = Vec::new();
    while pc < code.len() {
        steps += 1;
        if steps > max_steps {
            return Err("执行步数超限（可能死循环）".into());
        }
        match code[pc] {
            b'>' => ptr = (ptr + 1) % tape.len(),
            b'<' => ptr = (ptr + tape.len() - 1) % tape.len(),
            b'+' => tape[ptr] = tape[ptr].wrapping_add(1),
            b'-' => tape[ptr] = tape[ptr].wrapping_sub(1),
            b'.' => out.push(tape[ptr]),
            b',' => {
                tape[ptr] = input.get(inp).copied().unwrap_or(0);
                inp += 1;
            }
            b'[' if tape[ptr] == 0 => pc = jumps[pc],
            b']' if tape[ptr] != 0 => pc = jumps[pc],
            _ => {}
        }
        pc += 1;
    }
    Ok(out)
}

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let braincopter = pstr(p, "mode", "Braincopter") != "Brainloller";
        let src = decode(&img, braincopter);
        if src.is_empty() {
            return Err(CoreError::Parse(
                "未解出任何 Brainfuck 命令（换个模式试试？）。".into(),
            ));
        }
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(src.clone()));
        if pbool(p, "run", true) {
            let input = pstr(p, "input", "").as_bytes().to_vec();
            match run_bf(&src, &input, 5_000_000) {
                Ok(o) => {
                    m.insert(
                        "output".into(),
                        PortValue::Text(String::from_utf8_lossy(&o).into_owned()),
                    );
                    m.insert(
                        "bytes".into(),
                        PortValue::Bytes(Arc::from(o.into_boxed_slice())),
                    );
                }
                Err(e) => {
                    m.insert("output".into(), PortValue::Text(format!("[执行失败] {e}")));
                }
            }
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "braintools_decode",
            STEG,
            "BrainTools (Brainfuck 图)",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "Brainfuck 源码", PortType::Text),
                opt("output", "运行输出", PortType::Text),
                opt("bytes", "输出字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select(
                    "mode",
                    "模式",
                    &["Braincopter", "Brainloller"],
                    "Braincopter",
                ),
                ParamSpec::toggle("run", "执行程序", true),
                ParamSpec::text("input", "标准输入(可选)", "", false),
            ],
        ),
        Arc::new(|| Arc::new(Decode)),
    );
}
