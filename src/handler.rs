use crate::redis_cli::RedisClusterManager;
use redis::AsyncCommands;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_request(
    method: &str,
    params: Value,
    redis_mgr: &Arc<RedisClusterManager>,
) -> anyhow::Result<Value> {
    match method {
        // 列出可用工具
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "redis_get",
                    "description": "从 Redis 集群获取特定 Key 的值",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "key": { "type": "string" }
                        },
                        "required": ["key"]
                    }
                },
                {
                    "name": "redis_set",
                    "description": "向 Redis 集群写入一个字符串 Key",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "key": { "type": "string" },
                            "value": { "type": "string" }
                        },
                        "required": ["key", "value"]
                    }
                },
                {
                    "name": "redis_del",
                    "description": "从 Redis 集群删除一个 Key",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "key": { "type": "string" }
                        },
                        "required": ["key"]
                    }
                }
            ]
        })),

        // 执行具体的工具调用
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("tools/call 参数错误: 缺少字段 name 或类型不是 string"))?;

            let args = params
                .get("arguments")
                .ok_or_else(|| anyhow::anyhow!("tools/call 参数错误: 缺少字段 arguments"))?;

            match name {
                "redis_get" => {
                    let key = args
                        .get("key")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "redis_get 参数错误: 缺少字段 key 或类型不是 string"
                            )
                        })?;

                    let mut conn = redis_mgr.get_conn().await?;
                    let val: Option<String> = conn.get(key).await?;

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": val.unwrap_or_else(|| "Key 不存在".to_string())
                        }]
                    }))
                }

                "redis_set" => {
                    let key = args
                        .get("key")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "redis_set 参数错误: 缺少字段 key 或类型不是 string"
                            )
                        })?;

                    let value = args
                        .get("value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "redis_set 参数错误: 缺少字段 value 或类型不是 string"
                            )
                        })?;

                    let mut conn = redis_mgr.get_conn().await?;
                    let _: () = conn.set(key, value).await?;

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("OK: 已写入 key={}", key)
                        }]
                    }))
                }

                "redis_del" => {
                    let key = args
                        .get("key")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "redis_del 参数错误: 缺少字段 key 或类型不是 string"
                            )
                        })?;

                    let mut conn = redis_mgr.get_conn().await?;
                    let deleted: i64 = conn.del(key).await?;

                    let msg = if deleted > 0 {
                        format!("OK: 已删除 key={}", key)
                    } else {
                        format!("Key 不存在: {}", key)
                    };

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": msg
                        }]
                    }))
                }

                _ => Err(anyhow::anyhow!(format!("未知工具: {name}"))),
            }
        }

        _ => Err(anyhow::anyhow!(format!("不支持的方法: {}", method))),
    }
}
