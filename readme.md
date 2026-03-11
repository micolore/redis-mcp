## 项目简介

`redis-mcp` 是一个使用 Rust 实现的 **Redis 集群 MCP Server**，通过 MCP 协议向 Cursor 等客户端暴露操作 Redis 的工具。目前支持：

- `redis_get`：读取指定 key 的字符串值  
- `redis_set`：写入指定 key 的字符串值  
- `redis_del`：删除指定 key  

同时支持 **Redis Cluster** 和 **多环境配置**（通过环境变量 `CONFIG_PATH` 切换不同的 `config.toml`）。

---

## 技术栈

- **语言**：Rust 2021
- **异步运行时**：Tokio
- **Redis 客户端**：`redis`（cluster-async）
- **协议**：手写 JSON-RPC + STDIO（兼容 Cursor MCP）

---

## 配置说明

### 1. 默认配置（单环境）

项目根目录下的 `config.toml`：

```toml
redis_nodes = [
    "redis://192.168.1.4:6379",
    "redis://192.168.1.5:6379"
]
server_name = "my-redis-mcp"
timeout_secs = 3
```

- **redis_nodes**：Redis 集群各节点地址  
- **server_name**：服务名，仅用于日志输出  
- **timeout_secs**：获取 Redis 连接的超时时间（秒）

### 2. 多环境配置（通过 CONFIG_PATH）

通过环境变量 `CONFIG_PATH` 指定配置文件路径，例如（PowerShell）：

```powershell
$env:CONFIG_PATH="D:\workspace\ai\mcp\scrm-redis-dev.toml"
```

`config.rs` 行为：

- 若设置 `CONFIG_PATH`：使用该路径加载配置  
- 否则：回退到当前工作目录下的 `config.toml`  
- 额外校验：
  - `redis_nodes` 不能为空  
  - `timeout_secs` 必须大于 0

示例多环境配置文件 `scrm-redis-dev.toml`：

```toml
redis_nodes = [
    "redis://10.0.0.1:6379",
    "redis://10.0.0.2:6379"
]
server_name = "scrm-redis-dev"
timeout_secs = 3
```

---

## 构建与运行

### 1. 构建

在项目根目录执行：

```bash
cargo build --release
```

生成可执行文件（Windows）：

```text
target\release\redis-mcp.exe
```

### 2. 手工测试（命令行）

以 `redis_set` 为例，在 PowerShell 中：

```powershell
$env:CONFIG_PATH="D:\workspace\ai\mcp\scrm-redis-dev.toml"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"redis_set","arguments":{"key":"mcp:test","value":"hello"}}}' |
    target\release\redis-mcp.exe
```

成功时会返回类似：

```json
{"id":1,"jsonrpc":"2.0","result":{"content":[{"text":"OK: 已写入 key=mcp:test","type":"text"}]}}
```

---

## 在 Cursor 中作为 MCP 使用

### 1. MCP 配置示例

在 `mcp.json` 中添加：

```jsonc
{
  "mcpServers": {
    "scrm-redis-dev": {
      "type": "command",
      "command": "D:\\workspace\\rust\\redis-mcp\\target\\release\\redis-mcp.exe",
      "args": [],
      "env": {
        "CONFIG_PATH": "D:\\workspace\\ai\\mcp\\scrm-redis-dev.toml"
      }
    }
  }
}
```

说明：

- **scrm-redis-dev**：在 Cursor 里显示的 MCP 服务名  
- **command**：指向构建出的 `redis-mcp.exe`  
- **CONFIG_PATH**：指定要使用的配置文件（支持多环境 / 多集群）

### 2. 在 Cursor 中测试

1. 打开 Cursor → 右侧 MCP / Tools 面板  
2. 选择服务 `scrm-redis-dev`  
3. 工具列表中可以看到：
   - `redis_get`
   - `redis_set`
   - `redis_del`

#### 示例：写入 / 读取 / 删除

- **写入（redis_set）**

```json
{
  "key": "scrm:mcp:test",
  "value": "hello-scrm-mcp"
}
```

返回：`OK: 已写入 key=scrm:mcp:test`

- **读取（redis_get）**

```json
{
  "key": "scrm:mcp:test"
}
```

返回：`hello-scrm-mcp`

- **删除（redis_del）**

```json
{
  "key": "scrm:mcp:test"
}
```

第一次返回：`OK: 已删除 key=scrm:mcp:test`  
第二次返回：`Key 不存在: scrm:mcp:test`

---

## MCP 协议行为概览

- 传输方式：STDIO（stdin / stdout）  
- 消息格式：一行一个 JSON-RPC 2.0 消息  

### 支持的方法

- **initialize**
  - 请求：Cursor 发送 `method: "initialize"`，带 `protocolVersion`、`capabilities`、`clientInfo` 等
  - 响应：
    - `protocolVersion`：回传客户端版本或默认值
    - `capabilities.tools.listChanged: false`
    - `serverInfo`：`{ "name": "redis-mcp", "version": "0.1.0" }`

- **tools/list**
  - 返回当前可用工具列表及其 `inputSchema`

- **tools/call**
  - 根据 `params.name` 分发到具体工具，并使用 `params.arguments` 作为参数

- **notifications/*（如 notifications/cancelled）**
  - 按 JSON-RPC 规范视为通知，不返回响应

---

## 工具说明

### 1. redis_get

- **描述**：从 Redis 集群获取指定 key 的字符串值  
- **参数**：

```json
{
  "key": "string"
}
```

- **行为**：
  - `GET key`
  - key 存在：返回对应值  
  - key 不存在：返回 `"Key 不存在"`

### 2. redis_set

- **描述**：向 Redis 集群写入一个字符串 key  
- **参数**：

```json
{
  "key": "string",
  "value": "string"
}
```

- **行为**：
  - `SET key value`
  - 成功返回：`"OK: 已写入 key=<key>"`

### 3. redis_del

- **描述**：从 Redis 集群删除一个 key  
- **参数**：

```json
{
  "key": "string"
}
```

- **行为**：
  - `DEL key`
  - 删除数 > 0：`"OK: 已删除 key=<key>"`  
  - 删除数 = 0：`"Key 不存在: <key>"`

---

## 后续扩展建议

- 增加按前缀 / pattern 查询的工具（如 `keys`、`scan`，注意集群环境的开销）  
- 增加 JSON / Hash 类型的读写支持  
- 增加简单的命名空间约定（如统一使用 `env:tenant:biz:key` 结构），避免 key 冲突

