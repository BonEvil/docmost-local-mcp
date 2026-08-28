use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use docmost_local_mcp::{
    auth::webview::run_auth_window, server::DocmostMcpServer,
    startup_config::parse_runtime_startup_config,
};
use rmcp::{ServiceExt, transport::io::stdio};

#[derive(Parser, Debug)]
#[command(name = "docmost-local-mcp")]
#[command(about = "Docmost MCP server for local IDE integrations")]
struct Cli {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    authority_mode: Option<String>,
    #[arg(long)]
    write_tools: Option<String>,
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
            // Parse the original process arguments and environment in one place so CLI and
            // library tests enforce the same fail-closed authority rules.
            let argv = std::env::args().skip(1).collect::<Vec<_>>();
            let startup_config = parse_runtime_startup_config(&argv)?;
            let server = DocmostMcpServer::new(startup_config)?;
            server.serve(stdio()).await?.waiting().await?;
            Ok(())
        }
    }
}
