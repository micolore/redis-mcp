use redis::cluster::ClusterClient;
use redis::cluster_async::ClusterConnection;
use std::time::Duration;
use tokio::time::timeout;

pub struct RedisClusterManager {
    client: ClusterClient,
    conn_timeout: Duration,
}

impl RedisClusterManager {
    pub fn new(nodes: Vec<&str>, timeout_secs: u64) -> anyhow::Result<Self> {
        let client = ClusterClient::new(nodes)?;
        Ok(Self {
            client,
            conn_timeout: Duration::from_secs(timeout_secs),
        })
    }

    pub async fn get_conn(&self) -> anyhow::Result<ClusterConnection> {
        match timeout(self.conn_timeout, self.client.get_async_connection()).await {
            Ok(Ok(conn)) => Ok(conn),
            Ok(Err(e)) => Err(anyhow::anyhow!("Redis 连接失败: {}", e)),
            Err(_) => Err(anyhow::anyhow!("连接数据库超时")),
        }
    }
}
