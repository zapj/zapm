# ZAPM - Rust Process Manager

ZAPM (Zest Advanced Process Manager) 是一个用 Rust 编写的简单进程管理器，提供 Web 界面来管理和监控进程。

## 功能

- 启动、停止、重启和删除进程
- 通过 Web 界面管理进程
- 配置工作目录和环境变量
- 自动重启选项
- 实时进程状态监控

## 使用方法

### 命令行工具

ZAPM 提供了完整的命令行工具来管理进程，无需通过 Web 界面。

#### 1. 启动服务器

```bash
zapm server
```

服务器将在 http://localhost:9527 上运行。

#### 2. 列出所有进程

```bash
zapm list
```

#### 3. 查看进程详情

```bash
zapm show <process-name>
```

#### 4. 添加或更新进程

```bash
zapm add <process-name> --command "<command>" [--working-dir <path>] [--env "KEY1=VAL1,KEY2=VAL2"] [--auto-restart]
```

示例：
```bash
zapm add my-process --command "node server.js" --working-dir "/path/to/app" --env "NODE_ENV=production,PORT=3000" --auto-restart
```

#### 5. 启动进程

```bash
zapm start <process-name>
```

#### 6. 停止进程

```bash
zapm stop <process-name>
```

#### 7. 重启进程

```bash
zapm restart <process-name>
```

#### 8. 删除进程

```bash
zapm remove <process-name>
```

#### 9. 强制移除进程（不停止直接删除配置）

```bash
zapm remove --force <process-name>
```

### Web 界面

启动服务器后，可以通过 Web 界面管理进程：

```bash
zapm server
```

服务器将在 http://localhost:9527 上运行。

## API 使用方法

### 1. 获取所有进程列表

获取所有已配置的进程及其状态。

```bash
GET /api/processes

# 示例
curl http://localhost:9527/api/processes
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

获取指定进程的详细信息。

```bash
GET /api/processes/:name

# 示例
curl http://localhost:9527/api/processes/my-process
```

响应示例：
```json
{
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
```

### 3. 创建或更新进程

创建新进程或更新现有进程的配置。

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

# 示例
curl -X POST http://localhost:9527/api/processes/my-process \
  -H "Content-Type: application/json" \
  -d '{
    "command": "node server.js",
    "working_dir": "/path/to/app",
    "env": {
      "NODE_ENV": "production",
      "PORT": "3000"
    },
    "auto_restart": true
  }'
```

### 4. 启动进程

启动指定的进程。如果进程已经在运行，则返回成功。

```bash
POST /api/processes/:name/start
Content-Type: application/json

# 请求体（可选，如果进程已配置）
{
  "command": "node server.js",    // 可选，如果进程已配置
  "working_dir": "/path/to/app",  // 可选
  "env": {                        // 可选
    "NODE_ENV": "production",
    "PORT": "3000"
  },
  "auto_restart": true            // 可选，是否自动重启
}

# 示例（进程已配置）
curl -X POST http://localhost:9527/api/processes/my-process/start

# 示例（提供额外参数）
curl -X POST http://localhost:9527/api/processes/my-process/start \
  -H "Content-Type: application/json" \
  -d '{
    "command": "node server.js",
    "working_dir": "/path/to/app",
    "env": {
      "NODE_ENV": "production"
    },
    "auto_restart": true
  }'
```

### 5. 停止进程

停止指定的进程。

```bash
POST /api/processes/:name/stop

# 示例
curl -X POST http://localhost:9527/api/processes/my-process/stop
```

### 6. 重启进程

重启指定的进程。

```bash
POST /api/processes/:name/restart

# 示例
curl -X POST http://localhost:9527/api/processes/my-process/restart
```

### 7. 删除进程

删除指定的进程配置。如果进程正在运行，会先停止进程。

```bash
DELETE /api/processes/:name

# 示例
curl -X DELETE http://localhost:9527/api/processes/my-process
```

## 构建

```bash
cargo build --release
```

## 许可证

MIT