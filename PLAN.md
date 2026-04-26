# Implementation Plan: LSP-to-MCP Bridge

## Overview

构建一个将 LSP 能力桥接到 MCP 的系统，让 AI 能像 IDE 用户一样精准导航代码库。采用垂直切片策略：先用 Rust (rust-analyzer) 验证完整链路，再扩展多语言支持。

## Architecture Decisions

1. **先用 stdio MCP Server 跑通核心链路**，Tauri 桌面端后续集成
2. **Rust 语言先行**，验证架构后再添加 Go/TS
3. **文件 watch 使用 notify crate**，被动同步而非主动 didChange 管理
4. **代码切片默认 50 行上限**，goto_definition 默认 top-3 实现类

## Dependency Graph

```
Workspace 初始化
    │
    ├── 语言检测 (registry)
    │       │
    │       └── LSP 进程启动 (process)
    │               │
    │               ├── LSP 握手 (handshake)
    │               │       │
    │               │       └── LSP Client (client)
    │               │               │
    │               │               ├── 文件 Watch (watcher)
    │               │               │
    │               │               └── LSP 请求处理
    │               │                       │
    │               │                       └── MCP Tool 映射 (bridge)
    │               │                               │
    │               │                               └── MCP Server (server)
    │               │                                       │
    │               │                                       └── 可被 Claude Code 连接
    │               │
    │               └── Tauri GUI 管理（后续阶段）
    │
    └── 测试 fixtures (后续)
```

## Task List

### Phase 1: 项目骨架

- [ ] **Task 1: 初始化 Rust 项目结构**
  - 创建 Cargo workspace，配置基础依赖（tokio, serde_json, lsp-types, notify）
  - 创建 src 目录结构
  - **Acceptance:** `cargo build` 成功，目录结构符合 SPEC
  - **Verify:** `cargo check` 无错误
  - **Dependencies:** None
  - **Files:** `Cargo.toml`, `src/main.rs`, `src/lib.rs`
  - **Scope:** S

- [ ] **Task 2: 实现 LSP Registry（语言检测 + 配置）**
  - 根据根目录特征文件检测语言（Cargo.toml → rust-analyzer）
  - 存储语言 → LSP 二进制路径映射
  - **Acceptance:** 能正确识别 Rust 项目并返回 rust-analyzer 配置
  - **Verify:** 单元测试：传入 "Cargo.toml" 路径，返回 rust-analyzer
  - **Dependencies:** Task 1
  - **Files:** `src/lsp/mod.rs`, `src/lsp/registry.rs`
  - **Scope:** S

### Phase 2: LSP 进程管理

- [ ] **Task 3: 启动 LSP 进程**
  - 使用 tokio::process::Command 启动 rust-analyzer
  - 接管 stdin/stdout 管道
  - 实现进程生命周期管理（启动、退出、崩溃恢复）
  - **Acceptance:** 能成功启动 rust-analyzer 进程，stdin/stdout 管道可用
  - **Verify:** 手动运行，检查 rust-analyzer 进程存在
  - **Dependencies:** Task 2
  - **Files:** `src/lsp/process.rs`
  - **Scope:** M

- [ ] **Task 4: 实现 LSP 握手**
  - 发送 initialize 请求（包含 rootUri, capabilities）
  - 等待响应并解析 LSP 能力清单
  - 发送 initialized 通知
  - 实现 textDocument/didOpen 通知
  - **Acceptance:** 与 rust-analyzer 完成完整握手，LSP 进入就绪状态
  - **Verify:** 日志显示 handshake 完成，LSP 返回 capabilities
  - **Dependencies:** Task 3
  - **Files:** `src/lsp/client.rs`, `src/lsp/types.rs`
  - **Scope:** M

### Checkpoint: LSP 进程可用
- [ ] rust-analyzer 能启动并完成握手
- [ ] stdin/stdout 通信正常
- [ ] 代码编译无错误

### Phase 3: MCP 桥接

- [ ] **Task 5: 实现 MCP Server 骨架**
  - 使用 rmcp 或手动实现 MCP 协议基础
  - stdio 通信模式
  - 实现 MCP 初始化握手
  - **Acceptance:** MCP Server 能响应 Claude Code 的初始化请求
  - **Verify:** 用 Claude Code 连接，日志显示 MCP handshake 成功
  - **Dependencies:** Task 4
  - **Files:** `src/mcp/mod.rs`, `src/mcp/server.rs`, `src/mcp/protocol.rs`
  - **Scope:** M

- [ ] **Task 6: 实现 goto_definition MCP Tool**
  - 定义 Tool schema（file_path, line, character, all_implementations）
  - 调用 LSP textDocument/definition
  - 解析响应，提取目标位置
  - 实现代码切片读取（上下各扩展 20 行）
  - **Acceptance:** 调用 Tool 返回正确的定义位置 + 代码片段
  - **Verify:** 在 fixtures/rust-sample 上测试，返回正确的函数定义位置
  - **Dependencies:** Task 5, 需要先有测试 fixtures
  - **Files:** `src/mcp/tools.rs`, `src/bridge/handlers.rs`, `src/bridge/snippet.rs`, `src/utils/file.rs`
  - **Scope:** M

- [ ] **Task 7: 实现 find_references MCP Tool**
  - 调用 LSP textDocument/references
  - 返回所有引用位置列表
  - **Acceptance:** 返回正确的引用位置列表
  - **Verify:** 在 fixtures 上测试，找到所有调用点
  - **Dependencies:** Task 5, Task 6（复用 snippet 逻辑）
  - **Files:** `src/mcp/tools.rs`, `src/bridge/handlers.rs`
  - **Scope:** S

- [ ] **Task 8: 实现 hover MCP Tool**
  - 调用 LSP textDocument/hover
  - 返回类型签名 + Docstring
  - **Acceptance:** 返回正确的类型信息和文档
  - **Verify:** 在 fixtures 上测试，显示变量类型
  - **Dependencies:** Task 5
  - **Files:** `src/mcp/tools.rs`, `src/bridge/handlers.rs`
  - **Scope:** S

- [ ] **Task 9: 实现 workspace_symbols MCP Tool**
  - 调用 LSP workspace/symbol
  - 返回符号列表（名称:类型:位置）
  - **Acceptance:** 能全局搜索类名/函数名
  - **Verify:** 搜索 fixtures 中的函数名，返回正确位置
  - **Dependencies:** Task 5
  - **Files:** `src/mcp/tools.rs`, `src/bridge/handlers.rs`
  - **Scope:** S

- [ ] **Task 10: 实现 get_diagnostics MCP Tool**
  - 订阅 LSP diagnostics 推送通知
  - 缓存 diagnostics 结果
  - 提供 Tool 查询接口
  - **Acceptance:** 能获取文件的 lint/类型错误
  - **Verify:** 在有错误的 fixtures 上测试，返回错误列表
  - **Dependencies:** Task 5
  - **Files:** `src/mcp/tools.rs`, `src/bridge/handlers.rs`, `src/lsp/client.rs`
  - **Scope:** M

### Checkpoint: MCP Tools 可用
- [ ] 5 个 MCP Tool 都能响应
- [ ] 在 Claude Code 中能调用 Tools 并获得正确结果
- [ ] 代码切片不超过 50 行

### Phase 4: 文件 Watch

- [ ] **Task 11: 实现文件 Watcher**
  - 使用 notify crate 监控 workspace 目录
  - 文件变化时发送 textDocument/didChange 给 LSP
  - **Acceptance:** 编辑文件后，后续 LSP 查询返回更新后的结果
  - **Verify:** 修改 fixtures 文件，hover 返回更新后的类型
  - **Dependencies:** Task 4
  - **Files:** `src/lsp/watcher.rs`, `src/lsp/client.rs`
  - **Scope:** M

### Phase 5: 多语言扩展

- [ ] **Task 12: 添加 Go 语言支持 (gopls)**
  - 扩展 registry 添加 go.mod → gopls 映射
  - 测试 gopls 握手和 Tools
  - **Acceptance:** 在 go-sample 上所有 Tools 正常工作
  - **Verify:** 在 fixtures/go-sample 上测试 5 个 Tools
  - **Dependencies:** Task 11
  - **Files:** `src/lsp/registry.rs`
  - **Scope:** M

- [ ] **Task 13: 添加 TypeScript 支持 (typescript-language-server)**
  - 扩展 registry 添加 package.json → ts-ls 映射
  - 测试 TS 握手和 Tools
  - **Acceptance:** 在 ts-sample 上所有 Tools 正常工作
  - **Verify:** 在 fixtures/ts-sample 上测试 5 个 Tools
  - **Dependencies:** Task 11
  - **Files:** `src/lsp/registry.rs`
  - **Scope:** M

### Checkpoint: 多语言支持
- [ ] Rust/Go/TS 三种语言都能正常工作
- [ ] 自动语言检测正确

### Phase 6: Tauri 桌面端（后续）

- [ ] **Task 14: 创建 Tauri 项目骨架**
  - 初始化 src-tauri 目录
  - 配置 tauri.conf.json
  - **Acceptance:** `cargo tauri dev` 能启动空白窗口
  - **Dependencies:** Phase 5 完成
  - **Files:** `src-tauri/` 目录
  - **Scope:** M

- [ ] **Task 15: 实现 LSP 进程管理 GUI**
  - 显示活跃的 LSP 进程列表
  - 显示进程状态（运行中/已停止）
  - 手动启动/停止 LSP 进程
  - **Acceptance:** GUI 能显示和管理 LSP 进程
  - **Dependencies:** Task 14
  - **Files:** `src-tauri/src/lsp_manager.rs`, 前端组件
  - **Scope:** L（需拆分）

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| rmcp SDK 不成熟或不支持 stdio | High | 准备备选方案：手动实现 MCP JSON-RPC |
| LSP 握手细节差异（不同语言 server 行为不同） | Med | 每种语言单独测试，记录差异 |
| notify 在 Windows 上行为异常 | Med | 先在 Linux/macOS 验证，Windows 单独测试 |
| gopls/ts-ls 需要 Node.js 环境 | Med | 文档说明依赖环境，检测环境是否满足 |
| 多实现类排序逻辑复杂 | Low | 第一版简单返回前 3 个（按响应顺序） |

## Test Fixtures（需提前准备）

```
tests/fixtures/
├── rust-sample/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs        # 入口函数，调用 lib.rs 中的函数
│       └── lib.rs         # 定义函数，有跨模块调用
│
├── go-sample/
│   ├── go.mod
│   └── main.go            # 定义 interface + 多实现类
│
└── ts-sample/
│   ├── package.json
│   ├── tsconfig.json
│   └── src/
│       ├── index.ts       # 类型定义 + 函数调用
│       └── types.ts       # 导出类型
```

## Open Questions

1. rmcp SDK 是否已支持 MCP 2024-11-05 版本？需要验证
2. gopls 和 ts-ls 在 Windows 上的安装路径如何检测？

## Verification

- [ ] 每个任务有 acceptance criteria
- [ ] 每个任务有 verification step
- [ ] 任务依赖正确排序
- [ ] 检查点设置合理
- [ ] 无 XL 任务