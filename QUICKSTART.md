# 🚀 K0maru-Harness 快速上手与日常研发标准 SOP (`QUICKSTART.md`)

`k0maru-harness` 是 K0maru 智能体工程底座的无守护进程（Daemonless）元脚手架与编排中枢，统一协调五大 Rust 微内核与三套 MCP Server。本文档提供从**一键环境装配**、**健康度自检**到**日常研发全流程**的极简人类操作指南。

---

## 1. 一键安装与跨机同步 (`install.sh` & `setup`)

无论是首次在新机器（Mac 驾驶舱或远端 Linux 算力节点）部署，还是升级微内核版本后刷新 MCP 注册表，均可通过一条命令幂等完成。

### 1.1 本地一键装配（仅刷新 MCP 配置、目录总线与 Schema）
当二进制已安装，仅需自动创建 `~/.k0maru/{tuples,receipts,logs}`、幂等写入 `~/.gemini/config/mcp_config.json` 并生成 Antigravity CLI 所需的 5 个懒加载工具 Schema 时：
```bash
k0maru-harness setup
```
预期返回单行 K-DSL 状态电报：
```text
!Σ:SETUP|mcp_config:UPDATED|schemas:5|dirs:OK|probe:READY
```

### 1.2 全量编译安装与跨机一键同步 (`./scripts/install.sh`)
- **本机全量编译 + 签名 + MCP 装配 + 知识库同步**：
  ```bash
  ./scripts/install.sh
  ```
- **跨机（Tailscale / SSH）全栈同步至远端节点（如 `company-pc`）**：
  ```bash
  ./scripts/install.sh --remote company-pc
  ```
  该脚本会自动通过 `rsync` 同步五大微内核源码、`SecondBrain` 知识库、全局 `GEMINI.md` 铁律与 `skills/`，并在远端编译二进制、执行 `k0maru-harness setup` 与 `k0maru sync`。

---

## 2. 五大微内核与 3 个 MCP Server 清单自检

### 2.1 五大 Rust 微内核二进制清单
所有二进制统一安装在 `~/.local/bin/` 与 `~/.cargo/bin/`：

| 组件名称 | 二进制名 | 架构层级 | 核心职责 |
| :--- | :--- | :--- | :--- |
| **k0maru-harness** | `k0maru-harness` | Meta-Orchestrator | 全生命周期流水线、JIT 经验激活、Pre-Stdout 日志下沉、Seen-3× 错题自动演化 |
| **k0maru-agent-router** | `k0maru-router` | L3/L4 Bus & Router | K-DSL 0.1 编解码、Linda 元组空间 (`~/.k0maru/tuples`)、Herdr/tmux 跨机探针 |
| **k0maru-agent-verifier** | `k0maru-verifier` | L2 Physical Oracle | 编译/Clippy/测试硬门禁、波普尔反向证伪 (`--red-check`)、SHA-256 签名收据 |
| **k0maru-agent-workspace** | `k0maru-workspace` | L1 Sandbox | 基于 Git Worktree 的瞬态隔离沙箱生命周期管理 |
| **k0maru-agent-memory** | `k0maru-memory` (`k0maru`) | L0 Cognitive Hub | SQLite BM25 + ONNX 向量 + 双链图谱 RRF 检索、活体架构守卫 (`k0maru guard`) |

运行以下命令一键检查五大微内核及本地总线状态：
```bash
k0maru-harness probe
```

### 2.2 三个核心 MCP Server 清单
执行 `k0maru-harness setup` 后，`~/.gemini/config/mcp_config.json` 中会自动维护以下 3 个核心 MCP Server（并完整保留 `diana-voice`、`zhihu` 等其他已有服务）：

1. **`k0maru-harness`** (`k0maru-harness mcp`)：
   - `harness_probe`：探活四大支柱微内核与 Linda/Receipts 总线状态（返回 `!Σ:PROBE|...`）。
   - `harness_jit`：状态决策点毫秒级提取 `<180 Tokens` 架构不变量与历史错题。
   - `harness_record_lesson`：向 `LESSONS.md` 写入或累加故障教训（满 `Seen 3×` 自动升格为 Living ADR）。
   - `harness_run`：端到端执行任务并将冗长日志下沉至 `~/.k0maru/logs/`，仅向 Agent 返回单行 `!Σ:PIPELINE|...` 电报，彻底杜绝上下文膨胀。
   - `harness_receipt`：按 `task_id` 校验物理收据并返回单行 `!Σ:RECEIPT|...`。
2. **`k0maru-memory`** (`k0maru mcp --vault ~/Documents/SecondBrain`)：
   - 提供 `recall_memory`、`get_project_loadout`、`guard_check`、`flush_session` 等认知中枢工具。
3. **`k0maru-router`** (`k0maru-router mcp`)：
   - 提供 `route_task`、`probe_workspace`、`run_e2e_pipeline` 等跨仓/跨机路由工具。

---

## 3. 日常开发标准 SOP (Standard Operating Procedure)

### 步骤一：仓库准入与 Git 门禁挂载（每个仓库仅需一次）
在任意 Git 仓库根目录执行：
```bash
k0maru-harness init-hook
```
此命令会在 `.git/hooks/pre-commit` 安装守护钩子，自动拦截违反 ADR 的代码变更（`k0maru guard check --diff`）以及针对 `main`/`master` 的直接提交。

### 步骤二：架构决策点 JIT 经验激活
在进行分支操作、并发锁设计、数据库选型或接口重构前，按需拉取高密度约束：
```bash
k0maru-harness memory jit --decision "git branching"
```

### 步骤三：执行端到端验证与波普尔反向证伪 (`--red-check`)
在功能分支完成开发与单元测试编写后，调用单行电报流水线或验证器生成不可篡改的收据：
```bash
# 方式 A：通过 Harness 统一流水线运行（自动下沉日志至 ~/.k0maru/logs/）
k0maru-harness run "implement feature X" --domain HARNESS --red-check --wire

# 方式 B：直接针对基准分支 dev 执行波普尔反向证伪并签署指定 Task ID 收据
k0maru-verifier verify --red-check --base-branch dev --task-id TICK-007
```

### 步骤四：核验加密收据并合入 `dev`
```bash
k0maru-harness receipt TICK-007
# 输出示例：!Σ:RECEIPT|TICK-007|status:PASS|red_check:PASS|commit:c6a89d0|sig:70a705158c37
```

---

## 4. 常见排障速查 (Troubleshooting)

| 症状 / 报错 | 根因定位 | 快速修复命令 |
| :--- | :--- | :--- |
| `k0maru-harness probe` 显示某组件 `MISSING` | 二进制未软链或未安装至 `~/.local/bin` | 运行 `./scripts/install.sh` 重新编译安装 |
| macOS 终端执行二进制被 `SIGKILL (Exit 137)` 终止 | Apple Silicon 未对覆盖安装的二进制重签 Ad-hoc 签名 | 运行 `codesign -s - -f ~/.local/bin/k0maru*` |
| AI Agent 找不到 `harness_run` / `harness_jit` MCP 工具 | Lazy MCP Schema 尚未同步至 `~/.gemini/antigravity-cli/mcp/` | 运行 `k0maru-harness setup` 自动补齐 Schema 与配置 |
| `k0maru-verifier verify --red-check` 报错失败 | 新增测试在回退实现代码后依然通过（空洞断言或伪测试） | 检查新增 `#[test]`，确保断言真实验证了新逻辑并在旧基线上失败 |
| 流水线执行失败需要查看完整编译/测试报错堆栈 | Pre-Stdout Sink 默认将详细输出重定向至日志文件 | 查看 `!Σ:PIPELINE` 电报中的 `log:~/.k0maru/logs/<TASK_ID>.log` |
