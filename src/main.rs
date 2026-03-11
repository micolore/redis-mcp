mod config;
mod redis_cli;
mod handler;

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::AppConfig::load()?;

    let redis_mgr = Arc::new(redis_cli::RedisClusterManager::new(
        cfg.get_node_refs(),
        cfg.timeout_secs,
    )?);

    eprintln!("Redis MCP Server v2 [{}] is running...", cfg.server_name);

    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut reader = BufReader::new(stdin).lines();
    let mut writer = BufWriter::new(stdout);

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                let s = serde_json::to_string(&resp)?;
                writer.write_all(s.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        // notifications/* 是通知，不需要返回任何响应
        let resp_opt = match method.as_str() {
            "initialize" => Some(handle_initialize(id.clone(), params)),
            "tools/list" | "tools/call" => {
                Some(handle_tools_method(&method, id.clone(), params, &redis_mgr).await)
            }
            m if m.starts_with("notifications/") => None,
            _ => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Unknown method: {}", method)
                }
            })),
        };

        if let Some(resp) = resp_opt {
            let s = serde_json::to_string(&resp)?;
            writer.write_all(s.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

fn handle_initialize(id: Value, params: Value) -> Value {
    // 如果客户端传了 protocolVersion，则优先使用；否则回退默认值
    let client_protocol = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("2025-11-25");

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": client_protocol,
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "redis-mcp",
                "version": "0.1.0"
            }
        }
    })
}

async fn handle_tools_method(
    method: &str,
    id: Value,
    params: Value,
    redis_mgr: &Arc<redis_cli::RedisClusterManager>,
) -> Value {
    let result = handler::handle_request(method, params, redis_mgr).await;

    match result {
        Ok(value) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": value
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": e.to_string()
            }
        }),
    }
}
