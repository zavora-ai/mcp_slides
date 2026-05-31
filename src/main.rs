#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::{ServiceExt, transport::stdio};
    slides_mcp_server::SlidesServer::new().serve(stdio()).await?.waiting().await?;
    Ok(())
}
