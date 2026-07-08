# Codex Message Manage

本地 Codex 数据管理器。后端使用 Rust，前端使用 Vue3 + Element Plus。

## 功能特性

- 本地 Codex 会话数据管理
- 会话备份与恢复
- 会话标题编辑
- 会话删除与回收站
- 数据库优化

## 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+
- npm 或 yarn

### 启动项目

**开发：**
```bash
# 启动后端
cargo run

# 启动前端（新终端）
cd frontend && npm run dev
```

**使用构建的程序：**

在[releases](https://github.com/VacantHusky/codex-message-manage/releases)下载最新的程序，解压运行。

### 访问应用

- 前端开发服务器：http://localhost:5173
- 后端 API：http://127.0.0.1:5178

## 配置

后端默认读取 `codex-manager.toml`：

```toml
data_dir = "codex-save"
bind = "127.0.0.1:5178"
trash_dir = ".codex-manager-trash"
metadata_path = ".codex-manager.json"
```

### 环境变量

- `CODEX_DATA_DIR`：指定 Codex 数据目录路径

### 更换数据目录

#### 方法1：修改配置文件

编辑 `codex-manager.toml` 文件，修改 `data_dir` 字段：

```toml
data_dir = "/path/to/your/codex/data"
```

#### 方法2：使用环境变量

启动时设置环境变量：

```bash
# Windows
set CODEX_DATA_DIR=D:\path\to\your\codex\data
start.bat

# Linux/macOS
CODEX_DATA_DIR=/path/to/your/codex/data ./start.sh
```

#### 方法3：通过 API 动态更新

使用 API 端点更新数据目录配置：

```bash
curl -X POST http://127.0.0.1:5178/api/config/data-dir \
  -H "Content-Type: application/json" \
  -d '{"data_dir": "/path/to/your/codex/data"}'
```

**注意**：通过 API 更新后需要重启应用才能生效。

## 生产构建

```bash
cd frontend && npm install && npm run build
cd .. && cargo run
```

构建后访问 `http://127.0.0.1:5178`。

## 发布包

```bash
scripts/build-release.sh
```

脚本会构建前端，并按 Rust target 生成 `dist/codex-message-manage-<version>-<target>.tar.gz`。

## 项目结构

```
codex-message-manage/
├── src/                    # Rust 后端源代码
│   ├── main.rs            # 主入口
│   ├── models.rs          # 数据模型
│   └── store.rs           # 数据存储
├── frontend/              # Vue3 前端
│   ├── src/               # 前端源代码
│   ├── package.json       # 前端依赖
│   └── vite.config.ts     # Vite 配置
├── codex-save/            # Codex 数据目录
├── scripts/               # 构建脚本
├── Cargo.toml             # Rust 依赖配置
├── codex-manager.toml     # 应用配置
└── README.md              # 项目说明
```

## 开发说明

### 后端

- 使用 Axum 作为 Web 框架
- 使用 SQLx + SQLite 作为数据库
- 支持 CORS 跨域请求

### 前端

- 使用 Vue3 + TypeScript
- 使用 Element Plus UI 组件库
- 使用 Vite 作为构建工具

## 许可证

MIT
