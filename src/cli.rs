use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "whitenoise-cli")]
#[command(about = "WhiteNoise CLI - Secure MLS messaging client")]
#[command(version)]
pub struct Cli {
    /// Run in interactive mode (default if no command specified)
    #[arg(short, long)]
    pub interactive: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Suppress all output except results
    #[arg(short, long)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<String>,

    /// Account public key to use (hex format)
    #[arg(short = 'a', long)]
    pub account: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Account management commands
    Account {
        #[command(subcommand)]
        command: AccountCommands,
    },
    /// Contact management commands  
    Contact {
        #[command(subcommand)]
        command: ContactCommands,
    },
    /// Group management commands
    Group {
        #[command(subcommand)]
        command: GroupCommands,
    },
    /// Message commands
    Message {
        #[command(subcommand)]
        command: MessageCommands,
    },
    /// Relay management commands
    Relay {
        #[command(subcommand)]
        command: RelayCommands,
    },
    /// Batch operations from file
    Batch {
        /// Path to batch file (JSON or YAML)
        #[arg(short, long)]
        file: String,
    },
    /// Get status information
    Status,
    /// Manage keys locally (for keyring-less environments)
    Keys {
        #[command(subcommand)]
        command: KeysCommands,
    },
}

#[derive(Subcommand)]
pub enum KeysCommands {
    /// Store a private key locally
    Store {
        /// Public key (hex)
        #[arg(short, long)]
        pubkey: String,
        /// Private key (nsec or hex)
        #[arg(short = 'k', long)]
        privkey: String,
    },
    /// Retrieve a stored private key
    Get {
        /// Public key (hex)
        #[arg(short, long)]
        pubkey: String,
    },
    /// List all stored public keys
    List,
    /// Remove a stored key
    Remove {
        /// Public key (hex)
        #[arg(short, long)]
        pubkey: String,
    },
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Create a new identity
    Create {
        /// Display name
        #[arg(short, long)]
        name: Option<String>,
        /// About/bio
        #[arg(short, long)]
        about: Option<String>,
    },
    /// Login with existing key
    Login {
        /// Private key (nsec or hex)
        #[arg(short, long)]
        key: String,
    },
    /// List all accounts
    List,
    /// Show current account info
    Info,
    /// Export public key
    Export {
        /// Export private key instead of public
        #[arg(short, long)]
        private: bool,
    },
    /// Update profile
    Update {
        /// Display name
        #[arg(short, long)]
        name: Option<String>,
        /// About/bio  
        #[arg(short, long)]
        about: Option<String>,
    },
    /// Logout current account
    Logout,
}

#[derive(Subcommand)]
pub enum ContactCommands {
    /// Add a contact
    Add {
        /// Contact's public key (npub or hex)
        #[arg(short, long)]
        pubkey: String,
        /// Display name
        #[arg(short, long)]
        name: String,
    },
    /// Remove a contact
    Remove {
        /// Contact's public key (npub or hex)
        #[arg(short, long)]
        pubkey: String,
    },
    /// List all contacts
    List,
    /// Fetch contacts from relays
    Fetch,
    /// Show contact details
    Show {
        /// Contact's public key (npub or hex)
        pubkey: String,
    },
}

#[derive(Subcommand)]
pub enum GroupCommands {
    /// Create a new group
    Create {
        /// Group name
        #[arg(short, long)]
        name: String,
        /// Group description
        #[arg(short, long)]
        description: Option<String>,
        /// Member public keys (comma-separated)
        #[arg(short, long)]
        members: Option<String>,
    },
    /// List all groups
    List,
    /// Show group details
    Show {
        /// Group ID
        group_id: String,
    },
    /// Join a group chat (interactive)
    Join {
        /// Group ID
        group_id: String,
    },
}

#[derive(Subcommand)]
pub enum MessageCommands {
    /// Send a message to a group
    Send {
        /// Group ID
        #[arg(short, long)]
        group_id: String,
        /// Message content
        #[arg(short, long)]
        message: String,
        /// Message kind (default: 1)
        #[arg(short, long, default_value = "1")]
        kind: u16,
    },
    /// Send a direct message (creates/uses MLS DM group)
    Dm {
        /// Recipient's public key (npub or hex)
        #[arg(short, long)]
        recipient: String,
        /// Message content
        #[arg(short, long)]
        message: String,
    },
    /// List messages from a group
    List {
        /// Group ID
        #[arg(short, long)]
        group_id: String,
        /// Number of messages to fetch (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// List direct messages with a contact
    ListDm {
        /// Contact's public key (npub or hex)
        #[arg(short, long)]
        contact: String,
        /// Number of messages to fetch (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Get or create DM group with a contact
    GetDmGroup {
        /// Contact's public key (npub or hex)
        #[arg(short, long)]
        contact: String,
    },
}

#[derive(Subcommand)]
pub enum RelayCommands {
    /// List configured relays
    List {
        /// Relay type filter (nostr, inbox, keypackage)
        #[arg(short, long)]
        relay_type: Option<String>,
    },
    /// Add a relay
    Add {
        /// Relay URL
        #[arg(short, long)]
        url: String,
        /// Relay type (nostr, inbox, keypackage)
        #[arg(short, long)]
        relay_type: String,
    },
    /// Remove a relay
    Remove {
        /// Relay URL
        #[arg(short, long)]
        url: String,
        /// Relay type (nostr, inbox, keypackage)
        #[arg(short, long)]
        relay_type: String,
    },
    /// Test relay connection
    Test {
        /// Relay URL
        url: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct BatchOperation {
    pub operations: Vec<BatchCommand>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum BatchCommand {
    AccountCreate { name: Option<String>, about: Option<String> },
    ContactAdd { pubkey: String, name: String },
    GroupCreate { name: String, description: Option<String>, members: Option<Vec<String>> },
    MessageSend { group_id: String, message: String, kind: Option<u16> },
    MessageDm { recipient: String, message: String },
    RelayAdd { url: String, relay_type: String },
}

#[derive(Serialize, Deserialize)]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> CommandResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}