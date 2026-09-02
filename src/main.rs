use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use docmost_local_mcp::{
    auth::webview::run_auth_window, server::DocmostMcpServer,
    startup_config::parse_runtime_startup_config, stdio_compat,
};
use rmcp::ServiceExt;

#[derive(Parser, Debug)]
#[command(name = "docmost-local-mcp")]
#[command(about = "Docmost MCP server for local IDE integrations")]
struct Cli {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    allow_insecure_loopback_http: bool,
    /// I acknowledge that encrypted credential files and their key share one directory
    #[arg(long)]
    allow_insecure_credential_file: bool,
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
    /// Remove all local authentication state for one canonical Docmost origin
    Forget(ForgetArgs),
}

#[derive(Args, Debug)]
struct ForgetArgs {
    #[arg(long)]
    base_url: String,
    #[arg(long)]
    allow_insecure_loopback_http: bool,
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
        Some(Command::Forget(args)) => {
            use docmost_local_mcp::{
                startup_config::CanonicalDocmostOrigin, storage::state_store::StateStore,
            };
            let origin =
                CanonicalDocmostOrigin::parse(&args.base_url, args.allow_insecure_loopback_http)?;
            StateStore::new(None, false)?
                .forget_origin(origin.as_str())
                .await?;
            eprintln!("Forgot local authentication state for {}.", origin.as_str());
            Ok(())
        }
        None => {
            // Parse the original process arguments and environment in one place so CLI and
            // library tests enforce the same fail-closed origin and authority rules.
            let argv = std::env::args().skip(1).collect::<Vec<_>>();
            let startup_config = parse_runtime_startup_config(&argv)?;

            // Run the bounded, side-effect-free Atlas compatibility preflight before
            // constructing the server. Server construction opens the origin-scoped state
            // store, so this ordering is the credential-access boundary.
            let mut stdout = tokio::io::stdout();
            let stdin = stdio_compat::negotiate(tokio::io::stdin(), &mut stdout).await?;
            let server = DocmostMcpServer::new(startup_config)?;
            server.serve((stdin, stdout)).await?.waiting().await?;
            Ok(())
        }
    }
}
