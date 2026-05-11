use clap::{Args, Parser, Subcommand};

use agentic_api::config::{RuntimeConfig, normalize_base_url};
use agentic_api::server;

#[derive(Args, Clone)]
struct GatewayOpts {
    #[arg(long, env = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,

    #[arg(long, default_value = "0.0.0.0")]
    gateway_host: String,

    #[arg(long, default_value_t = 9000)]
    gateway_port: u16,

    #[arg(long, default_value_t = 600.0)]
    upstream_ready_timeout: f64,

    #[arg(long, default_value_t = 2.0)]
    upstream_ready_interval: f64,
}

#[derive(Parser)]
#[command(name = "agentic-api", about = "Stateful API gateway for vLLM Responses API")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    llm_api_base: Option<String>,

    #[command(flatten)]
    gateway: GatewayOpts,
}

#[derive(Subcommand)]
enum Commands {
    /// Spawn vLLM and run the gateway in the foreground
    Serve {
        /// Model name or path
        model: String,

        /// vLLM server port
        #[arg(long, default_value_t = 8000)]
        port: u16,

        #[command(flatten)]
        gateway: GatewayOpts,

        /// Additional arguments passed through to vLLM
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        vllm_args: Vec<String>,
    },
}

fn build_config(llm_api_base: &str, opts: &GatewayOpts) -> RuntimeConfig {
    RuntimeConfig {
        llm_api_base: normalize_base_url(llm_api_base),
        openai_api_key: opts.openai_api_key.clone(),
        gateway_host: opts.gateway_host.clone(),
        gateway_port: opts.gateway_port,
        upstream_ready_timeout_s: opts.upstream_ready_timeout,
        upstream_ready_interval_s: opts.upstream_ready_interval,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agentic_api=info".parse().expect("valid filter")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            let base = cli.llm_api_base.ok_or(
                "standalone mode requires --llm-api-base; use `agentic-api serve <model>` for integrated mode",
            )?;
            let config = build_config(&base, &cli.gateway);
            server::run(config).await
        }
        Some(Commands::Serve {
            model,
            port,
            gateway,
            vllm_args,
        }) => {
            let base = format!("http://127.0.0.1:{port}");
            let config = build_config(&base, &gateway);

            let mut args = vec!["--model".to_owned(), model];
            args.push("--port".to_owned());
            args.push(port.to_string());
            args.extend(vllm_args);

            server::run_with_vllm(config, args).await
        }
    }
}
