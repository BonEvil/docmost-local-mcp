use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use docmost_local_mcp::{
    auth::webview::run_auth_window, server::DocmostMcpServer, types::StartupConfig,
};
use rmcp::{ServiceExt, transport::io::stdio};

#[derive(Parser, Debug)]
#[command(name = "docmost-local-mcp")]
#[command(about = "Docmost MCP server for local IDE integrations")]
struct Cli {
    #[arg(long, env = "DOCMOST_BASE_URL")]
    base_url: Option<String>,
    #[arg(
        long,
        env = "DOCMOST_ALLOW_INSECURE_LOOPBACK_HTTP",
        default_value_t = false
    )]
    allow_insecure_loopback_http: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(name = "auth-window", hide = true)]
    AuthWindow(AuthWindowArgs),
}

#[derive(Args, Debug)]
struct AuthWindowArgs {
    #[arg(long)]
    url: String,
    #[arg(long = "success-url")]
    success_url: String,
    #[arg(long, default_value = "Docmost Sign In")]
    title: String,
    #[arg(long, default_value_t = 500)]
    width: u32,
    #[arg(long, default_value_t = 680)]
    height: u32,
}

#[tokio::main]
async fn main() {
    if let Err(error) = try_main().await {
        eprintln!("{:#}", error);
        std::process::exit(1);
    }
}

async fn try_main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::AuthWindow(args)) => {
            run_auth_window(
                args.url,
                args.success_url,
                args.title,
                args.width,
                args.height,
            )
            .await?;
            Ok(())
        }
        None => {
            let startup_config = StartupConfig {
                base_url: cli.base_url,
                allow_insecure_loopback_http: cli.allow_insecure_loopback_http,
            };
            let server = DocmostMcpServer::new(startup_config)?;
            server.serve(stdio()).await?.waiting().await?;
            Ok(())
        }
    }
}
