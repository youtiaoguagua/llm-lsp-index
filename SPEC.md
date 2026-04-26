# Spec: LSP-to-MCP Bridge

## Objective

构建一个将 LSP (Language Server Protocol) 能力桥接到 MCP (Model Context Protocol) 的系统，让 AI 模型能够像 IDE 用户一样"漫游"代码库——精准跳转定义、查找引用、获取类型信息，而非依赖模糊的向量检索。

**目标用户：** Claude Code 等 AI 编程助手的用户，需要精准理解跨库代码依赖关系的开发者。

**核心价值：**
- 100% 准确的代码导航（编译器级别静态分析）
- 跨库、跨文件依赖链路追踪
- 零"幻觉匹配"，LSP 返回的坐标绝对精准

## Tech Stack

| 组件 | 技术 | 版本 |
|------|------|------|
| 核心引擎 | Rust + Tokio | 1.75+ / 1.x |
| 桌面框架 | Tauri | 2.x |
| MCP 协议 | rmcp (Rust MCP SDK) | latest |
| LSP 类型 | lsp-types crate | latest |
| 文件监控 | notify | latest |
| JSON-RPC | serde_json | latest |

## Commands

```bash
# 开发
cargo run                           # 启动 MCP Server (stdio mode)
cargo run -- --sse                  # 启动 MCP Server (SSE mode)
cargo tauri dev                     # 启动桌面应用开发模式

# 构建
cargo build --release               # 构建 MCP Server
cargo tauri build                   # 构建桌面应用安装包

# 测试
cargo test                          # 单元测试（后续补充）
cargo test --test integration       # 集成测试（后续补充）

# Lint
cargo clippy -- -D warnings         # Rust lint
cargo fmt --check                   # 格式检查
```

## Project Structure

```
lsp-index/
├── src/
│   ├── main.rs                 # MCP Server 入口
│   ├── mcp/
│   │   ├── mod.rs              # MCP 协议模块
│   │   ├── server.rs           # MCP Server 实现
│   │   ├── tools.rs            # 5 个 MCP Tool 定义
│   │   └── protocol.rs         # MCP 类型映射
│   ├── lsp/
│   │   ├── mod.rs              # LSP 客户端模块
│   │   ├── client.rs           # Headless LSP Client
│   │   ├── process.rs          # LSP 进程管理（启动/握手/通信）
│   │   ├── registry.rs         # 语言检测 + LSP 配置注册表
│   │   ├── watcher.rs          # 文件变化监控（notify crate）
│   │   └── types.rs            # LSP 类型处理
│   ├── bridge/
│   │   ├── mod.rs              # LSP → MCP 桥接层
│   │   ├── handlers.rs         # LSP 请求 → MCP Tool 映射
│   │   └── snippet.rs          # 代码切片提取器
│   └── utils/
│   │   ├── mod.rs
│   │   ├── file.rs             # 文件读取/切片
│   │   └── uri.rs              # URI 处理
│   └── config.rs                # 全局配置
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Tauri 应用入口
│   │   ├── app.rs              # 桌面应用逻辑
│   │   └── lsp_manager.rs      # GUI LSP 进程管理
│   ├── tauri.conf.json         # Tauri 配置
│   ├── Cargo.toml
│   └── build.rs
├── tests/
│   ├── integration/            # 集成测试（后续补充）
│   └── fixtures/                # 测试代码库样本
│       ├── rust-sample/
│       ├── go-sample/
│       └── ts-sample/
├── Cargo.toml                   # Workspace 配置
├── Cargo.lock
├── SPEC.md                      # 本规格文档
└── CLAUDE.md                    # Claude Code 项目指南
```

## MCP Tools

暴露 5 个核心 MCP Tool：

| Tool | LSP 方法 | 输入 | 输出 |
|------|----------|------|------|
| `lsp_goto_definition` | textDocument/definition | file_path, line, character, **all_implementations?: bool** | 目标文件路径 + 代码切片（默认 top-3，可选全部） |
| `lsp_find_references` | textDocument/references | file_path, line, character | 引用列表 (文件:行号) |
| `lsp_hover` | textDocument/hover | file_path, line, character | 类型签名 + Docstring |
| `lsp_workspace_symbols` | workspace/symbol | query | 符号列表 (名称:类型:位置) |
| `lsp_get_diagnostics` | textDocument/diagnostic* | file_path | lint/类型错误列表 |

*注：diagnostics 通过 LSP push 通知获取，需订阅 + 文件 watch 同步。

## Code Style

```rust
// 模块命名：小写，简短，语义清晰
mod lsp;
mod mcp;
mod bridge;

// 结构体命名：大驼峰
pub struct LspClient {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

// 函数命名：小写+下划线，动词开头
pub async fn start_lsp_process(config: &LspConfig) -> Result<LspClient> {
    // ...
}

// 错误处理：使用 thiserror 定义自定义错误
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP process failed to start: {0}")]
    ProcessStart(String),
    #[error("LSP handshake failed")]
    HandshakeFailed,
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),
}

// MCP Tool 实现示例
pub async fn lsp_goto_definition(
    client: &LspClient,
    file_path: &str,
    line: u32,
    character: u32,
) -> Result<DefinitionResult> {
    let params = GotoDefinitionParams {
        text_document: TextDocumentIdentifier {
            uri: file_path_to_uri(file_path),
        },
        position: Position { line, character },
    };
    let response = client.request("textDocument/definition", params)?;
    extract_definition_snippet(response, file_path)
}
```

## Testing Strategy

**阶段 1（当前）：手动验证**
- 用真实代码库启动 LSP 进程
- 通过 Claude Code 调用 MCP Tools 验证返回结果
- 确保跨语言场景正确工作

**阶段 2（后续补充）：自动化测试**
- 单元测试：mock LSP 响应，测试协议转换逻辑
- 集成测试：启动真实 LSP，用 fixtures 代码库验证
- 覆盖目标：核心 MCP Tool 处理逻辑

**测试代码库样本：**
- `tests/fixtures/rust-sample/` — 简单 Rust 项目，含跨模块调用
- `tests/fixtures/go-sample/` — 简单 Go 项目，含 interface 实现
- `tests/fixtures/ts-sample/` — 简单 TS 项目，含类型定义

## Boundaries

**Always:**
- 每个新语言支持必须实现完整的 LSP 握手流程
- MCP Tool 返回的代码切片不超过 50 行（避免 Token 爆炸）
- 文件 URI 忄须使用 `file://` 协议格式
- 使用 `thiserror` 定义错误类型
- commit 前运行 `cargo clippy` 和 `cargo fmt`

**Ask first:**
- 添加新的 MCP Tool（超出 5 个基础 Tool）
- 支持新语言（超出 Rust/Go/TypeScript）
- 改变 LSP 进程生命周期管理策略
- 添加外部依赖（超出核心 tech stack）
- 修改 MCP 协议版本兼容性

**Never:**
- commit `.env` 或任何敏感配置
- 在 vendor 目录写代码
- 跳过 LSP handshake 直接发送请求
- 修改 fixtures 测试代码库的结构（测试稳定性）

## Success Criteria

**MVP 成功标准：**

| 检查项 | 验证方式 |
|--------|----------|
| Rust 代码库导航正确 | 在 rust-sample 上调用 5 个 Tool，返回正确坐标 |
| Go 代码库导航正确 | 在 go-sample 上调用 5 个 Tool，返回正确坐标 |
| TS 代码库导航正确 | 在 ts-sample 上调用 5 个 Tool，返回正确坐标 |
| MCP Server 可被 Claude Code 连接 | 在 Claude Code 配置 MCP Server，成功调用 Tool |
| Tauri 桌面端可启动并显示 LSP 状态 | 运行 `cargo tauri dev`，GUI 显示活跃 LSP 进程 |

**终极目标：**
- AI 能通过 MCP Tools 精准追踪跨库调用链路（如 Controller → Service → DB layer）
- 零向量检索"幻觉匹配"

## Open Questions (已决定)

1. **多实现分支处理：** ✅ 已决定
   - 默认返回 top-3，添加可选参数 `all_implementations: bool` 让调用者获取全部

2. **文件同步策略：** ✅ 已决定
   - 使用文件 watch 机制（如 `notify` crate）监控磁盘变化，自动同步给 LSP
   - 避免主动 `didChange` 管理，简化实现

3. **SSE vs stdio：** ✅ 已决定
   - 使用 stdio 模式，Claude Code 直接启动进程