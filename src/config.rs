use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub redis_nodes: Vec<String>,
    pub server_name: String,
    pub timeout_secs: u64,
}

impl AppConfig {
    /// 加载应用配置。
    /// 优先读取环境变量 CONFIG_PATH 指定的文件路径；
    /// 如果未配置，则回退为当前工作目录下的 config.toml。
    pub fn load() -> anyhow::Result<Self> {
        // 优先使用环境变量 CONFIG_PATH
        let path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

        let content = fs::read_to_string(&path)
            .map_err(|_| anyhow::anyhow!(format!("找不到配置文件: {}", path)))?;
        let config: AppConfig = toml::from_str(&content)?;

        if config.redis_nodes.is_empty() {
            return Err(anyhow::anyhow!("配置错误: redis_nodes 不能为空"));
        }
        if config.timeout_secs == 0 {
            return Err(anyhow::anyhow!("配置错误: timeout_secs 必须大于 0"));
        }

        Ok(config)
    }

    pub fn get_node_refs(&self) -> Vec<&str> {
        self.redis_nodes.iter().map(|s| s.as_str()).collect()
    }
}
