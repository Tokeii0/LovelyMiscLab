# 更新日志

本项目所有值得注意的变更都会记录在此文件。版本遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.2.8] - 2026-07-04

### 新增

- **补齐 16 个常见 CTF「小技巧」节点**：
  - 经典/键盘密码：键盘移位(QWERTY) · 手机键盘 T9 多击 · 敲击码(Tap) · Polybius 方阵 · NATO 音标 · **凯撒爆破**(试全 25 位移、按字母频率排序)。
  - 文本：Leetspeak · **全角/半角**(CJK 常见) · 罗马数字 · IP 地址转换(点分/十进制/十六进制互转) · DNA(ATCG) 编码。
  - 数论(RSA)：**数论运算**(gcd/lcm/模逆/模幂/egcd) · **整数分解**(试除+Fermat+Pollard rho) · **中国剩余定理(CRT)**——大整数，复用 num-bigint。
  - 字节：字节序交换(大小端) · **Base64 隐写提取**(尾部 `=` 空闲低位)。
- **全新「可视化分析」分类**：把字节流画成图直接在节点上看。**字节直方图**（256 个字节值分布、按类别着色）、**熵曲线**（滑窗香农熵随偏移变化，一眼定位加密/压缩/内嵌区，binwalk 式）、**字节分布图**（每字节一像素、按类别着色的 binvis 式整体结构图）。均输出 `image`（直接在节点上显示、可点开放大）+ PNG 字节。
- **全新「二进制分析」分类（ELF / PE / Mach-O）**：用纯 Rust `goblin` 解析可执行文件，补齐逆向/取证的一大空白。6 个节点——**可执行信息**（格式/架构/位数/入口/类型/PE 子系统/ELF 解释器）、**节区列表**（名称/地址/大小/偏移/权限，出 Text 表 + Json + 名称列表）、**符号/导入/导出**（PE `dll!func` 导入表、ELF 动态符号+依赖库）、**字符串提取(strings)**（ASCII/UTF-16LE，任意字节可用）、**节区提取**（按名抠 `.rodata` 等原始字节）、**附加数据/overlay**（真二进制末尾之后的藏点，超常见 CTF 套路）。首批内置节点用上 `Json` 结构化输出端口。
- **「控制/逻辑」补齐 10 个节点**，让分支/合流/数值/列表这套控制流真正闭环：
  - **数值比较** / **数学运算** —— 补上 `比较`(只能比文本) 的缺口，分支可由 `数值范围`/计数驱动，支持 `+ - × ÷ mod pow min max abs …`。
  - **首个有效值(coalesce)** —— 缺失的「合流」原语，把多路 `条件选择`/`条件门` 分支合回一路（取第一个非空值）。
  - **空值判断** —— 判断上游是否有产出，配合条件门做「有数据才继续」。
  - **正则路由** —— 按正则依次匹配、一步「分类 + 分流」到不同输出端口，强于只认数字索引的 `多路分支`。
  - **列表操作**(反转/切片/取N) · **列表取值**(按索引，桥回标量) · **列表归约**(求和/计数/极值/平均…) · **列表配对(zip)**（拼 `user:pass` 喂爆破）。
  - **注释** —— 画布说明便签。

### 修复

- 小屏启动时窗口不再超出屏幕（窗口配置加 `center` + `preventOverflow`；感谢 @Xeonacid，[#3](https://github.com/Tokeii0/LovelyMiscLab/pull/3)）。

## [0.2.7] - 2026-07-04

### 新增

- **bkcrack 明文攻击节点（原生移植）**：把 bkcrack 对 ZipCrypto 传统加密的已知明文攻击（Biham–Kocher）**原生移植为 Rust 内置节点**，无需外部程序。喂入已知明文——内置 PNG / JPEG(JFIF) / OOXML / ELF 等常见文件头模板，或自定义 Hex/文本——即可求出三个内部密钥，并可直接解密目标条目（Deflate 条目自动 Inflate 还原）。查找表、`Keys` 更新、`Zreduction` 与 `Attack` 均照 bkcrack C++ 源码 1:1 移植，攻击已跨所有 CPU 核心并行；以其官方测试向量（`password` → `ea9b4e4d ba789085 5ff8707d`、"Hello World!" 攻击用例）与自造加密包端到端校验一致。移植自 [bkcrack](https://github.com/kimci86/bkcrack)（zlib 许可，© Kévin “kimci86” Falcoz）。
- **进度条控件 + 单节点进度流**：新增可复用进度条控件（实时百分比 + 当前阶段文字），并让**单节点执行也流式回传进度/日志**（此前仅整图运行有进度）。长耗时节点（如 bkcrack 攻击）在节点体与属性面板都能看到实时进度，并可随时取消。
- **2 个展示节点**：「图片查看」——把上游图片（图片/字节/data:URL）直接在节点上大图展示并原样透传，方便在流程中随处预览；「文本展示」——多行展示上游文本且**有进有出**，可插在数据流中间做「监视 + 继续」（区别于只进不出的「文本输出」）。
- **AI 生成 / Agent 支持「待处理数据」独立输入**：把很长的密文/字节单独粘进「待处理数据」框，程序会直接填入源节点的 `text` 参数，AI 只看开头 200 字预览来判断编码类型、不再照抄整段——避免长数据把输出 token 撑爆导致 JSON 截断/出错。

### 改进

- Agent 上下文预算上调至 ~1M token（原 ~100k），可一次搭更大的图。

## [0.2.6] - 2026-07-04

### 新增

- **macOS / Linux 跨平台支持**：应用现已适配 macOS（Intel + Apple Silicon 通用包）与 Linux（需 WebKitGTK 4.1）。发布流程自动构建三平台绿色版 —— Windows `exe`（UPX 压缩）、Linux 单文件二进制、macOS 通用 `.dmg`；CI 在 Windows / Linux / macOS 三个系统上全量构建、测试并做 Clippy 零告警检查。
- **跨平台自动更新**：自动更新器改为按平台挑选发布资源、校验对应平台可执行文件魔数；Windows / Linux 支持就地替换并自动重启，macOS 因 `.dmg` 内是 `.app` 包无法单文件替换，改为引导到发布页手动下载。
- **画布「整理节点」**：右键菜单一键把节点按当前视口宽高比竖向优先打包成多列并自适应缩放，让整张图在当前分辨率下全部可见（不再随依赖深度横向铺开）。
- **AI 生成进化为 Agent**：从"一次性出图"改为后端 LLM 工具调用循环，逐步在画布上增删节点 / 连线 / 设参、可运行中间结果自适应，用户实时看到搭建过程与旁白；模型不支持工具调用时自动回退一次性生成。
- **端口 AI 预测**：悬停节点输入 / 输出端口弹出 ✨ 建议面板，给出类型兼容的下一个节点（离线可用）并支持手动筛选与 AI 排序，确认即创建并自动连线、避让不重叠。
- **3 个新节点**：LSB 嵌入（图像最低位隐写写入）、ZIP 创建、ZIP 伪加密。

### 改进

- README 增加三平台下载与安装说明（各自的运行时依赖与 macOS Gatekeeper 处理）。
- CI 新增 `cargo clippy --all-targets -- -D warnings` 零告警门禁，并清理了 misclab-core 既有的 Clippy 告警。
- 字节输入端口（`in_bytes`）现可直接接受图片端口：自动解码 `data:` URL，LSB 提取 / 文件雕刻 / 哈希等可直接消费上游图片输出。
- MCP：兼容把对象参数序列化成字符串的客户端；`connect` 按节点描述符校验端口名并自动将 param 提升为可连线的输入句柄，避免 AI 建图时产生「连不上」的死连线。

## [0.2.5] - 2026-07-03

### 修复

- **MCP 工具列表校验失败（tools fetch failed）** —— `set_param` 工具的 `value` 入参是裸 `serde_json::Value`，schemars 会为其生成布尔 `true` 属性 schema；Claude 等 MCP 客户端拒绝布尔属性 schema，进而丢弃整个 `tools/list`，表现为连接正常却「工具全部不可用」。为该参数补上文档注释，将其提升为合法的对象 schema 修复；并在集成测试中新增回归断言，保证每个工具入参的属性 schema 均为对象。

## [0.2.4] - 2026-07-03

### 新增

对照常见 CTF-misc 套路补全 10 个节点：

**压缩包**

- **压缩包伪加密修复** —— 定位中央目录逐条清除 ZIP 伪加密位（bit0，可选强加密位 bit6），修复误报「需要密码」的压缩包。
- **ZIP CRC 爆破** —— 对未压缩长度很小的条目按字符集枚举明文匹配 CRC-32，拼接还原被拆成几字节的 flag。

**密码 / 编码**

- **CryptoJS AES 解密** —— 解 "Salted\_\_" 格式（EVP_BytesToKey/MD5 派生密钥，默认 AES-256-CBC），可给字典爆破口令；与 OpenSSL / CryptoJS 字节兼容。
- **Rabbit 流密码** —— 手写 eSTREAM Rabbit（RFC 4503），128 位密钥 + 可选 64 位 IV，加解密对称，对拍官方全部测试向量。
- **JWT 密钥爆破** —— 字典逐个试 HS256/384/512 的签名密钥，重算 HMAC 比对签名，命中即得 secret。
- **Brainfuck 解释器** —— 运行 Brainfuck / Ook! 源码得到输出（含标准输入）。

**图像 / 取证**

- **BMP 宽高修复** —— 未压缩位图按像素区大小反推被篡改的宽/高（BMP 无 CRC），也支持手动指定。
- **GIF 帧时序解码** —— 读每帧显示时长（厘秒），可当字节转 ASCII 或阈值二值化，flag 常藏在帧间隔里。
- **文件雕刻 (binwalk)** —— 扫描内嵌文件签名，列出偏移并抽取内嵌文件（如图片尾附的 ZIP）。
- **LSB 全组合扫描（zsteg 式）** —— 位平面 × 通道 × 位序 × 遍历方向逐一提取并按可读性 / flag 正则打分，列出最可能的隐写内容。

## [0.2.3] - 2026-07-03

### 新增

- **软件更新**：启动时自动检查 GitHub 最新 Release，一键下载并就地替换、自动重启；「设置 → 软件更新」也可手动检查。
- **图片查看器**：节点 / 检查器 / 运行记录里的图片，点击弹出大图。支持缩放、拖动平移、旋转 90°、水平/垂直翻转、中心对称，以及亮度 / 对比度 / 曝光 / 饱和度 / 锐度等 PS 风格实时调整（仅用于观察，不改原图）。
- **JPG 宽高修复**（新节点）：基线 JPEG 通过统计 MCU 数自动恢复被篡改的真实高度（JPEG 无 CRC 可校验），也支持手动指定尺寸。
- **音频处理**（新分类，5 个节点）：
  - 音频信息 —— 解析 WAV 采样率 / 声道 / 位深 / 时长；
  - 音频频谱图 —— STFT 频谱图，显现藏在频域里的文字或图案；
  - WAV LSB 提取 —— 提取采样最低有效位里的隐藏数据；
  - DTMF 拨号音解码 —— Goertzel 检测电话拨号音还原为数字；
  - **DeepSound 提取** —— 还原 DeepSound 藏在 WAV 里的隐藏文件，支持 AES-256 密码（DSC2/DSCF 格式，逆向自官方 DLL 并与真实文件对拍）。

### 改进

- **文件类型识别**：新增「后缀名」输出，方便拼接文件名等用途。
- **PNG 宽高修复**：新增「自动」模式 —— 解压 IDAT 数据流推断真实尺寸，可修复连 CRC 都被改写的「暴力爆破」情况；保留 CRC 爆破 / 手动模式。

## [0.2.2] - 2026-07-02

### 新增

- 8 个第三方图片隐写工具：cloacked-pixel、ImageMask（文本/文件）、BMP 填补字节、Invoke-PSImage、stegpy、BrainTools、PixelJihad。
- 哈希爆破、通用口令爆破（字典驱动任意节点）、压缩包密码爆破模板。
- HTTP 请求（GET/POST…、HTTP 1.1/2.0、UA/Cookie）、字符串匹配、压缩包文件列表 三个节点。

### 改进

- 运行记录按类型展示节点实际输出（图片 / 字节 / 字符串）。
- 模块库可拖动调宽、分类重排、每个模块加说明；流程模板新增「隐写术」分类与多个常见流程。
- 打包改用 UPX `--lzma --best` 压缩，Release 工作流自动生成更新日志。

## [0.2.1] - 2026-07-02

- 新增 imageIN(图影) 图片藏文件 提取/嵌入，以及配套的 imageIN 提取流程模板。
- 流程模板界面重做：按分类分组、卡片紧凑化。

## [0.2.0] - 2026-07-02

### 新增 · 内嵌 MCP 服务（供 AI 客户端调用）

- 在应用内内嵌 **MCP（Model Context Protocol）服务器**，让 Claude Code / Cursor / Codex / Claude Desktop 等 AI 客户端直接驱动节点图引擎。
  - 基于 **rmcp 2.0 + axum** 的 streamable-HTTP 服务，绑定 `127.0.0.1`，Bearer 令牌鉴权，运行在独立 tokio 线程。
  - **默认关闭**，在「设置 → MCP 服务」中一键启停；配置持久化到 `mcp.json`，令牌首次启用时自动生成。
- **22 个通用元工具**：
  - 发现：`list_categories`、`list_nodes`、`describe_node`、`list_modules`
  - 执行：`run_node`、`run_graph`（不带参数即运行当前画布）
  - 画布协作：`get_canvas`、`set_canvas`、`add_node`、`connect`、`set_param`、`remove_node`、`remove_edge`、`move_node`
  - 持久化与 AI：`save_workflow`、`load_workflow`、`save_composite_module`、`save_script_module`、`generate_workflow`
  - 其他：`get_settings`（API Key 脱敏）、`detect_tool`、`ping`
- **实时画布协作**：前后端画布双向同步，AI 的增删改即时显示在用户画布上（`sync_canvas` ↔ `mcp://canvas-update`，`applyingRemote` 标志 + `rev` 计数双重回声环守卫）。
- **渐进式节点发现（省 token）**：先 `list_categories` 看分类，再按分类精简列出（默认仅 id+名称），用到某节点才取参数详情；典型发现开销从约 78 KB 降到约 2 KB。
- **AI 友好的 IO 适配**：bytes/image 输出落盘并返回文件路径、长文本截断，避免淹没上下文；输入支持 base64 或文件路径。

### 变更

- 设置界面重构为**分类导航**（AI 模型 / 输出目录 / 外部工具 / MCP 服务，四个 tab），不再单页长滚动。
- MCP 面板内置 **Claude Code / Cursor / Codex / 其他客户端**的一键复制配置片段（自动填入端点与令牌）。

### 安全

- MCP 服务默认关闭、仅监听本机、强制 Bearer 令牌；`get_settings` 对 AI 隐藏 API Key。

### 其他

- `mcp` 作为默认 Cargo feature（`--no-default-features` 可移除整套 MCP 依赖）。
- 服务器逻辑经 `AppBridge` trait 抽象，可脱离 Tauri 单测；新增 HTTP 端到端集成测试。
- 版本号 0.1.0 → 0.2.0。

## [0.1.0]

- 初始版本：基于节点图的可视化分析工作台 —— 约 116 个内置节点、可视化画布、自定义复合/脚本节点、AI 生成工作流、流程文件（.lml）等。

[0.2.3]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.3
[0.2.2]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.2
[0.2.1]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.1
[0.2.0]: https://github.com/Tokeii0/lovelymisclab/releases/tag/v0.2.0
