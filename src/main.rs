#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};
    slides_mcp_server::SlidesServer::new()
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
