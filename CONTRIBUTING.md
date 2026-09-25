# 开发手册 · Contributing Guide

面向人类与 AI 助手（vibe coding）的贡献指南。动手前请通读本文，尤其是 [模块提交规范](#模块提交规范必读)。

> 一句话规矩：**给仓库贡献节点/功能只能用 Rust；依赖只用 Rust crate；想移植 Python 工具，先用 Rust 重写。**

---

## 目录

- [黄金准则](#黄金准则)
- [开发环境](#开发环境)
- [项目结构与核心概念](#项目结构与核心概念)
- [如何新增一个节点](#如何新增一个节点)
- [模块提交规范（必读）](#模块提交规范必读)
- [Python → Rust 依赖对照](#python--rust-依赖对照)
- [如何提交 PR](#如何提交-pr)
- [vibe coding（AI 辅助）须知](#vibe-codingai-辅助须知)
- [提交前检查清单](#提交前检查清单)

---

## 黄金准则

1. **纯 Rust**：仓库内的节点与引擎逻辑全部用 Rust 实现，依赖只允许 [crates.io](https://crates.io) 上的 Rust crate。**不接受**把 Python / 外部可执行程序作为运行期依赖引入仓库。
2. **单一真相源**：一个节点 = 一份 `NodeDescriptor`（声明输入/输出/参数）+ 一个 `Node::run`（实现逻辑）+ 一次 `register`。前端完全据 descriptor 渲染，**无需写任何前端代码**。
3. **可测**：编码/解密类节点必须带对照测试（test vectors）。`crates/misclab-core` 不依赖 Tauri，可 `cargo test` 直接跑。
4. **本地优先**：不引入需要联网的运行期依赖（AI 节点除外，且走用户自配的 OpenAI 兼容端点）。

## 开发环境

前置：[Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/) · [Rust 工具链](https://www.rust-lang.org/tools/install) · [Tauri 2 系统依赖](https://tauri.app/start/prerequisites/)。

```bash
pnpm install                              # 前端依赖
pnpm tauri dev                            # 桌面开发模式（热更新）

# 引擎相关只用 cargo 即可，无需起 Tauri：
cargo test -p misclab-core                # 引擎单元/集成测试
cargo fmt --all                           # 格式化
cargo clippy --all-targets -- -D warnings # 静态检查（零告警）
```

## 项目结构与核心概念

绝大多数贡献只会碰 **`crates/misclab-core/src/nodes/`**（一文件一节点）。

```
crates/misclab-core/src/
├── node/
│   ├── mod.rs          # Node trait、NodeCtx、NodeEnv
│   ├── descriptor.rs   # NodeDescriptor / PortSpec / ParamSpec / ParamWidget
│   └── registry.rs     # NodeRegistry（id → descriptor + 工厂）
├── graph/
│   ├── port.rs         # PortType / PortValue（连线上的强类型值）
│   └── executor.rs     # GraphExecutor（拓扑执行、单节点执行、缓存）
└── nodes/
    ├── prelude.rs      # 写节点的「配方」：desc()/req()/opt()/t_in()/out_text()/in_bytes()/pstr()/颜色·分类常量…
    ├── mod.rs          # 声明并注册所有节点（新增节点要在这里加两行）
    └── <各节点>.rs      # 具体实现，照抄相近的即可
```

- `Node` trait：`fn run(&self, inputs: &PortMap, params: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError>`
- `PortValue`：`Text / Number / Bool / Json / StringList / Candidates / Bytes / Image / Fingerprint / …`
- 端口类型在连接时自动校验；`Text` 可接受 Number/Bool/StringList（自动转字符串）。

## 如何新增一个节点

三步。以下是内置 `hash` 节点的真实结构（`nodes/hash.rs`），照它改即可：

```rust
// crates/misclab-core/src/nodes/hash.rs
use super::prelude::*;                     // 引入配方：desc/req/out_text/in_bytes/pstr/颜色·分类常量…

struct HashNode;                           // 1) 一个（通常无状态的）结构体

impl Node for HashNode {                   // 2) 实现 run：读输入 + 参数 → 算 → 写输出
    fn run(&self, inputs: &PortMap, params: &serde_json::Value, _ctx: &mut NodeCtx)
        -> Result<PortMap, CoreError>
    {
        let data = in_bytes(inputs, "data")?;                 // 读名为 "data" 的输入端口
        let algo = pstr(params, "algorithm", "SHA256");       // 读字符串参数，带默认值
        Ok(out_text(hash_hex(algo, &data)?))                  // 写名为 "text" 的输出端口
    }
}

pub fn register(reg: &mut NodeRegistry) {  // 3) 声明 descriptor 并注册
    reg.register(
        desc(
            "hash",                        // 全局唯一 id（snake_case）
            HASH,                          // 分类常量（见 prelude.rs：ENC/HASH/IMG/STEG…）
            "哈希计算",                     // 显示名
            CYAN,                          // 节点颜色常量
            vec![req("data", "输入", PortType::Any)],           // 输入端口
            vec![req("text", "摘要(hex)", PortType::Text)],     // 输出端口
            vec![ParamSpec::select("algorithm", "算法",         // 参数（会自动渲染成 UI 控件）
                &["MD5", "SHA1", "SHA256", "SHA512", "SHA3-256"], "SHA256")],
        ),
        Arc::new(|| Arc::new(HashNode)),   // 工厂：每次执行 new 一个实例
    );
}
```

然后在 `nodes/mod.rs` 里加两行把它接上：

```rust
mod hash;            // 顶部：声明模块
// …在 default_registry() 里：
hash::register(reg); // 注册
```

保存后节点**自动出现在前端调色板**，无需改任何前端代码。

**必须写测试**（`crates/misclab-core/tests/` 或节点文件内 `#[cfg(test)]`）：

```rust
let reg = default_registry();
let inputs = [("data".into(), PortValue::Text("hello".into()))].into();
let out = GraphExecutor::run_node(&reg, "hash", &inputs, &json!({"algorithm":"MD5"}),
                                  &NullSink, &CancellationToken::new())?;
assert_eq!(out["text"].as_text().unwrap(), "5d41402abc4b2a76b9719d911017c592");
```

> 小抄：`ParamSpec::select / number / text / toggle`；`req()` 必填端口、`opt()` 可选端口、`t_in()` 常见文本输入；分类/颜色常量都在 `prelude.rs`。拿不准就 `grep` 一个相近节点照抄。

## 模块提交规范（必读）

> 这里的「模块」= 想合并进**本仓库**的节点/功能。

### 1. 只允许 Rust 依赖

- 新代码用 **Rust** 写，依赖只加 **crates.io 上的 Rust crate**。
- **禁止**把 Python 脚本、`python`/`pip` 包、或任何外部可执行程序作为**运行期依赖**引入仓库。
- 加新 crate 前先看现有依赖（`Cargo.toml`）是否已能满足；确需新增，在 PR 里说明用途、体积、license（须与 GPL-3.0 兼容）。

### 2. 移植 Python 工具 = 用 Rust 重写

想把一个现成的 Python misc 工具/项目做成节点，按此步骤：

1. **找等价 crate**：查[下表](#python--rust-依赖对照)，多数常见依赖都已在用或有对应 Rust crate。
2. **无等价就手写**：misc 算法大多是位/字节操作，直接用 Rust 实现即可（别 shell 调 Python）。
3. **对照测试保正确**：把原 Python 工具的测试向量搬成 Rust `assert`，保证行为等价。
4. 按 [如何新增一个节点](#如何新增一个节点) 封装成节点提交。

### 3. 脚本节点 / 复合模块 ≠ 仓库贡献

应用内置的**脚本节点**（外部进程）和**复合模块**（子图）是给用户**本地自定义扩展**用的，会引入非 Rust 依赖，**不作为仓库 PR 提交**。要贡献进仓库，请落成上面的 Rust 内置节点。

### 4. 质量门槛

- `cargo fmt --all` 已格式化；`cargo clippy --all-targets -- -D warnings` 零告警。
- `cargo test -p misclab-core` 全绿；新节点带测试（编码/解密必须有对照向量）。
- `pnpm check:utf8` 通过（源码统一 UTF-8，无 BOM）。
- 若动了应用层/前端：`cargo test -p misclab-app --features mcp`、`pnpm build`、`pnpm test`（vitest）也要过。
- 以上检查 CI 都会执行（含 `cargo fmt --all --check` 与 `pnpm check:utf8`）。

## Python → Rust 依赖对照

移植时优先用这些（多数**已在仓库依赖里**）：

| Python | Rust crate | 备注 |
|---|---|---|
| pycryptodome / cryptography | RustCrypto：`aes` `des` `blowfish` `chacha20` `salsa20` `cbc` `ctr` `ecb` `rsa`(num-bigint) | 已在用 |
| hashlib | `sha1` `sha2` `sha3` `md-5` `md4` `ripemd` `blake2` `whirlpool` `sm3` `hmac` `crc32fast` | 已在用 |
| Pillow / PIL | `image` `imageproc` | 已在用 |
| numpy.fft / scipy.fft | `rustfft` | 已在用 |
| numpy（数组/矩阵） | `ndarray` | 如需，PR 说明 |
| base64 / binascii | `base64` `hex` | 已在用 |
| gmpy2 / 大整数 | `num-bigint` `num-integer` `num-traits` | 已在用 |
| re / regex | `regex` | 已在用 |
| zipfile / py7zr / tarfile / rarfile | `zip` `sevenz-rust` `tar` `flate2` `unrar` | 已在用 |
| qrcode / pyzbar / zxing | `qrcode` `rxing` | 已在用 |
| exifread / piexif | `kamadak-exif` | 已在用 |
| requests / urllib | `ureq`（轻）/ `reqwest` | ureq 已在用 |
| chardet / codecs | `encoding_rs` | 已在用 |
| pgpy | `pgp`（rPGP） | 已在用 |

找不到等价库？misc 算法通常几十行位运算就能自己写，**手写 + 测试向量**即可。

## 如何提交 PR

1. **Fork** 本仓库，`git clone` 你的 fork。
2. 从最新 `main` 切分支：`git switch -c feat/<简短描述>`（如 `feat/node-railfence`）。
3. 开发 + 自测（跑上面的质量门槛）。
4. 提交，信息用 `类型: 描述`（`feat` / `fix` / `docs` / `chore` / `refactor` / `test`），可用中文：
   ```
   feat: 新增栅栏密码节点（railfence，含加解密与对照测试）
   ```
5. `git push` 到你的 fork，在 GitHub 上对着本仓库 `main` 开 PR，说明：**做了什么 · 为什么 · 怎么测的**（贴命令输出）。
6. 保持 PR 聚焦单一主题；有冲突时 rebase 到最新 `main`。

## vibe coding（AI 辅助）须知

用 Claude Code / Cursor / Codex 等写贡献时：

- **先喂上下文**：让 AI 读 `CONTRIBUTING.md`（本文）与仓库根的 `CLAUDE.md`（若有）。
- **反复强调硬规矩**：「只能用 Rust，禁止引入 Python / 外部程序依赖；要移植的话用 Rust 重写」。
- **给模板**：让 AI 照抄 `crates/misclab-core/src/nodes/` 里最相近的节点，套 `prelude.rs` 的配方，别自创结构。
- **务必让 AI 自测**：生成后跑 `cargo test -p misclab-core`、`cargo clippy -- -D warnings`、`cargo fmt`，全绿再提。
- **可用内置 MCP 自测节点**：在「设置 → MCP 服务」启用后，让 AI 客户端连上用 `run_node` 直接验证你新写的节点行为（详见 [README](./README.md#mcp-与-ai-调用)）。

## 提交前检查清单

- [ ] 全 Rust，无新增 Python / 外部程序依赖；新 crate 已说明且 license 与 GPL-3.0 兼容
- [ ] 新节点：`Node` + `NodeDescriptor` + `register`，并在 `nodes/mod.rs` 接上
- [ ] 带测试（编码/解密含对照向量）
- [ ] `cargo fmt --all` · `cargo clippy --all-targets -- -D warnings` · `cargo test -p misclab-core` 全过
- [ ] `pnpm check:utf8` 通过
- [ ] 动了应用层/前端则额外过 `cargo test -p misclab-app --features mcp`、`pnpm build`
- [ ] PR 描述写清「做了什么 / 怎么测的」

感谢贡献！
