<div align="center">

<br />

<img src="src-tauri/icons/Square310x310Logo.png" alt="LovelyMiscLab Logo" width="160" height="160" />

# 🧪 LovelyMiscLab

### 面向取证的节点式可视化工作台

**像 ComfyUI 一样搭工作流，专为编码解密 · 隐写取证 · 图像分析 · 密码学而生。**

把杂项操作拖成节点、连成一张数据流图，本地实时执行。

<br />

![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)
![Vite](https://img.shields.io/badge/Vite-7-646CFF?logo=vite&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green)

<br />

</div>

---

## 这是什么

LovelyMiscLab 是一个桌面应用，核心是一个**类型化的节点图引擎（typed node-graph engine）**，专为取、编码等场景打造。

设计上**做的是类似于 ComfyUI 的节点式工作流**——你在画布上拖拽「节点」（Base64 解码、XOR、LSB 隐写提取、二维码解码、AES 解密、图像滤镜……），用连线把它们组成一条数据流 DAG，后端按拓扑序执行，并把每个节点的结果实时回显到画布上。同样是数据驱动、可视化编排、节点可自由扩展，只是把「生成图像」换成了「Misc 分析」。

它可以当作：

- **快捷工具箱** —— 单节点执行，把任意一个节点当独立小工具用
- **流程编排器** —— 整图执行，带内容寻址的增量缓存（改哪算哪的「实时模式」）
- **AI 助手** —— 用自然语言描述任务，一键生成节点图
- **可扩展平台** —— 把选中子图封装为「复合模块」，或把外部脚本 / 程序（python、tshark、7z……）接入为自定义节点
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/7af12c9e-05dd-4e87-ad4d-31f0fe836df6" />
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/ea5ddf9d-e176-4527-89a3-0ee942943567" />

## 主要特性

- **100+ 内置节点**，覆盖十余个分类：
  - 编码：Base32/45/58/62/64/85/92、Hex、URL、ROT13、摩斯、培根、盲文、A1Z26…
  - 加解密：AES / DES / Blowfish / RC4 / ChaCha / Salsa / RSA，以及维吉尼亚、仿射、Atbash、Playfair、Enigma、ADFGVX、栅栏、PGP、JWT…
  - 哈希：MD5 / SHA 系列 / SHA3 / RIPEMD / BLAKE2 / Whirlpool / SM3 / CRC32 / HMAC / bcrypt
  - 隐写：零宽字符（二进制 / 四进制 / 变体选择符 / Unicode 标签，含自动识别）、StegCloak（加密零宽，与官方工具双向字节兼容）、空白 / SNOW（空格·制表符）、LSB 图像隐写
  - 压缩包：ZIP / 7z / TAR / GZIP / RAR 解包
  - 图像处理：通道 / 混合 / 滤镜 / 几何变换 / 色彩空间 / 差分 / 频域（FFT）/ GIF…
  - 文本处理、进制转换、控制逻辑、分析工具（熵值、字频、hexdump、EXIF、文件类型识别…）
  - AI 节点：文本判断（`ai_judge`）、视觉识图（`ai_vision`）
- **类型化端口**：连线携带强类型值（Text / Number / Bool / Json / Bytes / Image / Fingerprint…），连接时自动做类型校验
- **增量执行**：内容寻址缓存，只重算受影响的节点
- **AI 生成流程**：自然语言 → 校验过的真实节点图，自动分层布局
- **自定义扩展**：复合模块（子图打包成一个节点，防递归深度 16 层）+ 脚本节点（外部进程接入，支持 stdin/argv/临时文件三种投递方式）
- **本地优先**：所有计算在本机完成，数据不出设备

## 架构

采用 Cargo workspace，把与 Tauri 无关的分析引擎和薄适配层彻底分离：

```
LovelyMiscLab/
├── crates/misclab-core/     # 纯 Rust 分析引擎（可无头单测、可被 CLI 复用）
│   ├── src/graph/           # 图引擎：端口类型、模型、执行器、缓存、复合/脚本节点
│   ├── src/node/            # Node trait、NodeDescriptor（数据驱动 UI 的单一真相源）、注册表
│   ├── src/nodes/           # 100+ 内置节点实现（一文件一节点）
│   ├── src/model/           # IPC 数据契约 & 取证分析领域模型
│   ├── src/ai.rs            # OpenAI 兼容的 chat / vision 调用
│   └── tests/               # 集成测试（含大量编码解密对照用例）
├── src-tauri/               # Tauri 应用外壳（薄适配器，over misclab-core）
│   ├── src/commands/        # 暴露的 Tauri command
│   ├── src/db.rs            # SQLite 持久化（projects / files / reports / findings / artifacts…）
│   └── src/lib.rs           # 应用入口：注册插件、构建注册表、打开数据库
├── src/                     # React 前端
│   ├── flow/                # 画布、节点、连线、检查器、右键菜单、模板加载
│   ├── app/                 # 标题栏、左侧栏、实时执行器、各类对话框
│   ├── views/               # 画布 / 模块 / 模板 / 设置 等视图
│   ├── store/               # zustand 状态（graph / run / descriptors / ai / view…）
│   └── lib/                 # Tauri IPC 绑定、类型、模板、工具
└── dist/                    # 前端构建产物
```

### 数据流

```
前端画布 (React + @xyflow/react)
        │  SerializedGraph (JSON)
        ▼
Tauri commands (src-tauri)  ──Channel──▶  流式进度 / 每节点结果
        │
        ▼
GraphExecutor (misclab-core)  拓扑排序 → 逐节点 run → 增量缓存
        │
        ▼
NodeRegistry + NodeDescriptor  ──▶  前端据此渲染调色板 / 节点体 / 连接校验
```

`NodeDescriptor` 是**单一真相源**：后端声明节点的输入 / 输出 / 参数控件，前端完全据此渲染 UI，无需为每个节点写前端代码。

### 关键 Tauri Commands

| 分类 | Command |
|---|---|
| 系统 | `ping` · `app_info` · `db_health` |
| 图执行 | `list_node_descriptors` · `run_node` · `run_graph` · `cancel_job` · `reset_run` |
| 设置 | `get_settings` · `set_settings` · `detect_tool` |
| AI | `generate_workflow` |
| 复合模块 | `list_composite_modules` · `save_composite_module` · `delete_composite_module` |
| 脚本节点 | `list_script_modules` · `save_script_module` · `delete_script_module` |
| 工程存取 | `save_project` · `load_project` |

## 技术栈

**前端**：React 19 · TypeScript · Vite 7 · Tailwind CSS v4 · @xyflow/react（节点图）· zustand（状态）· TanStack Query · React Router

**后端**：Rust (edition 2021) · Tauri 2 · petgraph（图算法）· rusqlite（SQLite，bundled）· image / imageproc · RustCrypto 全家桶 · rustfft · rPGP · encoding_rs

**Tauri 插件**：opener · dialog · fs · shell · store · notification · log

## 开发

### 前置依赖

- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- [Rust 工具链](https://www.rust-lang.org/tools/install)
- Tauri 2 的系统依赖（参见 [Tauri 前置要求](https://tauri.app/start/prerequisites/)）

### 常用命令

```bash
# 安装前端依赖
pnpm install

# 启动桌面开发模式（Tauri + Vite 热更新）
pnpm tauri dev

# 仅在浏览器预览前端（无 Tauri IPC，使用 mock 节点 + demo 图）
pnpm dev

# 构建发布版桌面应用
pnpm tauri build

# 运行引擎单元 / 集成测试（无需 Tauri）
cargo test -p misclab-core
```

`crates/misclab-core` 完全独立于 Tauri，可单独测试与复用。

## 推荐 IDE

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 许可

[MIT](./Cargo.toml)
