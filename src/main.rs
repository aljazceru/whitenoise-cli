use anyhow::Result;
use clap::Parser;
use console::{style, Term};

mod app;
mod account;
mod contacts;
mod groups;
mod relays;
mod ui;
mod storage;
mod whitenoise_config;
mod cli;
mod cli_handler;
mod keyring_helper;

use app::App;
use whitenoise_config::WhitenoiseManager;
use cli::Cli;
use cli_handler::CliHandler;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Check if we should run in CLI mode (non-interactive)
    if cli.command.is_some() {
        // CLI mode - handle commands and exit
        run_cli_mode(cli).await
    } else if cli.interactive {
        // Explicitly requested interactive mode
        run_interactive_mode().await
    } else {
        // Default to interactive mode when no command specified
        run_interactive_mode().await
    }
}

async fn run_cli_mode(cli: Cli) -> Result<()> {
    let mut handler = CliHandler::new(cli.output, cli.quiet, cli.account).await?;
    
    if let Some(command) = cli.command {
        handler.handle_command(command).await?;
    }
    
    Ok(())
}

async fn run_interactive_mode() -> Result<()> {
    // Configure selective logging to filter out known library issues
    // These are internal library issues that don't affect CLI functionality
    std::env::set_var("RUST_LOG", 
        "error,whitenoise::event_processor=off,nostr_relay_pool::relay::inner=off,whitenoise::delete_key_package_from_relays_for_account=off,nostr_relay_pool::pool=off,nostr_relay_pool=off"
    );
    
    let term = Term::stdout();
    term.clear_screen()?;
    
    println!("{}", style("🔐 WhiteNoise CLI - Secure Messaging").bold().cyan());
    println!("{}", style("Built on WhiteNoise + MLS Protocol").dim());
    println!();

    // Initialize WhiteNoise
    let mut whitenoise_manager = WhitenoiseManager::new()?;
    println!("{}", style("🔧 Initializing WhiteNoise...").yellow());
    whitenoise_manager.initialize().await?;
    println!("{}", style("✅ WhiteNoise initialized successfully!").green());
    println!();

    let mut app = App::new(whitenoise_manager).await?;
    
    loop {
        match app.run_main_menu().await {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(e) => {
                eprintln!("{} {}", style("Error:").red().bold(), e);
                ui::wait_for_enter("Press Enter to continue...");
            }
        }
    }

    println!("{}", style("👋 Goodbye!").green());
    Ok(())
}