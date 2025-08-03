# ZAPM - Rust 进程管理器

ZAPM (Zap Process Manager) 是一个用 Rust 编写的跨平台进程管理器，提供命令行工具和 Web 界面来管理和监控进程。

## 功能特性

- **进程管理**：启动、停止、重启和删除进程
- **Web 界面**：通过浏览器管理所有进程
- **跨平台支持**：同时支持 Windows 和 Linux 系统
- **守护进程模式**：作为后台服务运行
- **配置灵活**：支持工作目录和环境变量配置
- **自动重启**：进程崩溃时自动重启
- **实时监控**：监控进程状态和资源使用

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/zapj/zapm.git
cd zapm

# 构建项目
cargo build --release

# 安装到系统路径（可选）
cargo install --path .
```

## 使用方法

### 命令行工具

ZAPM 提供了完整的命令行工具来管理进程，无需通过 Web 界面。

#### 1. 启动服务器

```bash
zapm server [--host <host>] [--port <port>]
```

服务器默认在 http://localhost:2400 上运行。

#### 2. 以守护进程模式运行

```bash
zapm daemon
```

这将在后台启动 ZAPM，适用于 Windows 和 Linux 系统。

#### 3. 列出所有进程

```bash
zapm list
```

#### 4. 查看进程详情

```bash
zapm show <process-name>
zapm status <process-name>
```

#### 5. 添加或更新进程

```bash
zapm add <process-name> --cmd "<command>" [--dir <path>] [--env "KEY1=VAL1" --env "KEY2=VAL2"] [--auto-restart]
```

示例：
```bash
zapm add my-process --cmd "node server.js" --dir "/path/to/app" --env "NODE_ENV=production" --env "PORT=3000" --auto-restart
```

#### 6. 启动进程

```bash
zapm start <process-name>
```

#### 7. 停止进程

```bash
zapm stop <process-name>
```

#### 8. 重启进程

```bash
zapm restart <process-name>
```

#### 9. 删除进程

```bash
zapm remove <process-name>
```

#### 10. 强制移除进程（不停止直接删除配置）

```bash
zapm remove <process-name> --force
```

### Web 界面

启动服务器后，可以通过 Web 界面管理进程：

```bash
zapm server
```

然后在浏览器中访问 http://localhost:2400

## API 参考

### 1. 获取所有进程列表

```bash
GET /api/processes

# 示例
curl http://localhost:2400/api/processes
```

响应示例：
```json
{
  "my-process": {
    "name": "my-process",
    "command": "node server.js",
    "working_dir": "/path/to/app",
    "env": {
      "NODE_ENV": "production",
      "PORT": "3000"
    },
    "auto_restart": true,
    "status": "Running",
    "pid": 1234,
    "created_at": "2025-08-02T12:00:00+00:00",
    "updated_at": "2025-08-02T12:00:00+00:00"
  }
}
```

### 2. 获取单个进程信息

```bash
GET /api/processes/:name

# 示例
curl http://localhost:2400/api/processes/my-process
```

### 3. 创建或更新进程

```bash
POST /api/processes/:name
Content-Type: application/json

# 请求体
{
  "command": "node server.js",
  "working_dir": "/path/to/app",  // 可选
  "env": {                        // 可选
    "NODE_ENV": "production",
    "PORT": "3000"
  },
  "auto_restart": true            // 是否自动重启
}
```

### 4. 启动进程

```bash
POST /api/processes/:name/start
```

### 5. 停止进程

```bash
POST /api/processes/:name/stop
```

### 6. 重启进程

```bash
POST /api/processes/:name/restart
```

### 7. 删除进程

```bash
DELETE /api/processes/:name
```

## 配置文件

ZAPM 的配置文件位于：

- Windows: `%APPDATA%\zapm\config.json`
- Linux: `~/.config/zapm/config.json`

## 系统要求

- Windows 7+ 或 Linux (内核 2.6.23+)
- 至少 50MB 可用内存

## 贡献

欢迎提交 Pull Request 和 Issue！

## 许可证

MIT