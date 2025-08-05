use anyhow::Result;
use serde_json;
use std::collections::HashMap;
use whitenoise::{PublicKey, RelayType, Metadata};

use crate::{
    app::App,
    cli::{
        AccountCommands, ContactCommands, GroupCommands, MessageCommands, RelayCommands,
        Commands, CommandResult, OutputFormat, BatchOperation, BatchCommand, KeysCommands
    },
    whitenoise_config::WhitenoiseManager,
    keyring_helper::{KeyringHelper, setup_keyring_environment},
};

pub struct CliHandler {
    app: App,
    output_format: OutputFormat,
    quiet: bool,
    account_pubkey: Option<String>,
}

impl CliHandler {
    pub async fn new(output_format: OutputFormat, quiet: bool, account_pubkey: Option<String>) -> Result<Self> {
        // Initialize WhiteNoise in quiet mode for CLI
        // Completely suppress nostr_relay_pool errors which include purplepag.es timeouts
        std::env::set_var("RUST_LOG", "whitenoise=error,nostr_relay_pool=off");
        
        // Setup keyring environment for keyring-less operation
        setup_keyring_environment()?;
        
        let whitenoise_manager = WhitenoiseManager::new()?;
        let mut manager = whitenoise_manager;
        manager.initialize().await?;
        
        let mut app = App::new(manager).await?;
        
        // Auto-login if account pubkey is provided
        if let Some(pubkey) = &account_pubkey {
            app.auto_login_by_pubkey(pubkey).await?;
        }
        
        Ok(Self {
            app,
            output_format,
            quiet,
            account_pubkey,
        })
    }

    pub async fn handle_command(&mut self, command: Commands) -> Result<()> {
        let result = match command {
            Commands::Account { command } => self.handle_account_command(command).await,
            Commands::Contact { command } => self.handle_contact_command(command).await,
            Commands::Group { command } => self.handle_group_command(command).await,
            Commands::Message { command } => self.handle_message_command(command).await,
            Commands::Relay { command } => self.handle_relay_command(command).await,
            Commands::Batch { file } => self.handle_batch_command(file).await,
            Commands::Status => self.handle_status_command().await,
            Commands::Keys { command } => self.handle_keys_command(command).await,
        };

        match result {
            Ok(output) => {
                if !self.quiet {
                    println!("{}", output);
                }
                Ok(())
            }
            Err(e) => {
                let error_result = CommandResult::<()>::error(e.to_string());
                let output = self.format_output(&error_result)?;
                if !self.quiet {
                    eprintln!("{}", output);
                }
                std::process::exit(1);
            }
        }
    }

    async fn handle_account_command(&mut self, command: AccountCommands) -> Result<String> {
        match command {
            AccountCommands::Create { name, about } => {
                let account = self.app.account_manager.create_identity().await?;
                
                // Set up default relays
                self.app.setup_default_relays(&account).await?;
                
                // Update metadata if provided
                if name.is_some() || about.is_some() {
                    let mut metadata = Metadata::new();
                    if let Some(n) = name {
                        metadata = metadata.name(&n);
                    }
                    if let Some(a) = about {
                        metadata = metadata.about(&a);
                    }
                    self.app.account_manager.update_metadata(&metadata).await?;
                }

                let result = CommandResult::success(serde_json::json!({
                    "pubkey": account.pubkey.to_hex(),
                    "message": "Account created successfully"
                }));
                self.format_output(&result)
            }
            AccountCommands::Login { key } => {
                let account = self.app.account_manager.login(key).await?;
                
                // Clean up unwanted relays
                if let Err(_) = self.app.relays.cleanup_unwanted_relays(&account).await {
                    // Ignore cleanup errors in CLI mode
                }

                let result = CommandResult::success(serde_json::json!({
                    "pubkey": account.pubkey.to_hex(),
                    "message": "Login successful"
                }));
                self.format_output(&result)
            }
            AccountCommands::List => {
                let accounts = self.app.account_manager.fetch_accounts().await?;
                let result = CommandResult::success(accounts);
                self.format_output(&result)
            }
            AccountCommands::Info => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let metadata = self.app.account_manager.get_metadata().await.ok().flatten();
                    let result = CommandResult::success(serde_json::json!({
                        "pubkey": account.pubkey.to_hex(),
                        "metadata": metadata
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            AccountCommands::Export { private } => {
                if private {
                    let nsec = self.app.account_manager.export_nsec().await?;
                    let result = CommandResult::success(serde_json::json!({
                        "private_key": nsec
                    }));
                    self.format_output(&result)
                } else {
                    let npub = self.app.account_manager.export_npub().await?;
                    let result = CommandResult::success(serde_json::json!({
                        "public_key": npub
                    }));
                    self.format_output(&result)
                }
            }
            AccountCommands::Update { name, about } => {
                let mut metadata = Metadata::new();
                if let Some(n) = name {
                    metadata = metadata.name(&n);
                }
                if let Some(a) = about {
                    metadata = metadata.about(&a);
                }
                
                self.app.account_manager.update_metadata(&metadata).await?;
                let result = CommandResult::success(serde_json::json!({
                    "message": "Profile updated successfully"
                }));
                self.format_output(&result)
            }
            AccountCommands::Logout => {
                self.app.account_manager.logout().await?;
                let result = CommandResult::success(serde_json::json!({
                    "message": "Logged out successfully"
                }));
                self.format_output(&result)
            }
        }
    }

    async fn handle_contact_command(&mut self, command: ContactCommands) -> Result<String> {
        match command {
            ContactCommands::Add { pubkey, name } => {
                // First add to CLI's ContactManager for local use
                self.app.contacts.add(name.clone(), pubkey.clone()).await?;
                // Save contacts to storage after adding
                self.app.storage.save_contacts(&self.app.contacts).await?;
                
                // Also add to WhiteNoise's contact system for group/DM functionality
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let contact_pubkey = if pubkey.starts_with("npub") {
                        whitenoise::PublicKey::parse(&pubkey)
                            .map_err(|e| anyhow::anyhow!("Invalid npub format: {:?}", e))?
                    } else {
                        whitenoise::PublicKey::from_hex(&pubkey)
                            .map_err(|e| anyhow::anyhow!("Invalid hex format: {:?}", e))?
                    };
                    
                    let whitenoise = whitenoise::Whitenoise::get_instance()
                        .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
                    
                    // Add contact to WhiteNoise's system (ignore duplicate errors)
                    let _ = whitenoise.add_contact(&account, contact_pubkey).await;
                }
                
                let result = CommandResult::success(serde_json::json!({
                    "pubkey": pubkey,
                    "name": name,
                    "message": "Contact added successfully"
                }));
                self.format_output(&result)
            }
            ContactCommands::Remove { pubkey } => {
                self.app.contacts.remove(&pubkey).await?;
                // Save contacts to storage after removing
                self.app.storage.save_contacts(&self.app.contacts).await?;
                let result = CommandResult::success(serde_json::json!({
                    "pubkey": pubkey,
                    "message": "Contact removed successfully"
                }));
                self.format_output(&result)
            }
            ContactCommands::List => {
                let contacts = self.app.contacts.list();
                let result = CommandResult::success(contacts);
                self.format_output(&result)
            }
            ContactCommands::Fetch => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    self.app.contacts.fetch_contacts(account.pubkey).await?;
                    let count = self.app.contacts.list().len();
                    let result = CommandResult::success(serde_json::json!({
                        "message": format!("Fetched {} contacts", count),
                        "count": count
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            ContactCommands::Show { pubkey } => {
                if let Some(contact) = self.app.contacts.list().iter().find(|c| c.public_key == pubkey) {
                    let result = CommandResult::success(contact);
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("Contact not found".to_string());
                    self.format_output(&result)
                }
            }
        }
    }

    async fn handle_group_command(&mut self, command: GroupCommands) -> Result<String> {
        match command {
            GroupCommands::Create { name, description, members } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let member_pubkeys = if let Some(members_str) = members {
                        let keys: Result<Vec<PublicKey>, _> = members_str
                            .split(',')
                            .map(|s| PublicKey::from_hex(s.trim()).or_else(|_| PublicKey::parse(s.trim())))
                            .collect();
                        keys?
                    } else {
                        // Empty member list - creator is automatically added by MLS protocol
                        vec![]
                    };

                    let admin_pubkeys = vec![account.pubkey];
                    let desc = description.unwrap_or_default();

                    let group = self.app.groups.create_group(
                        account,
                        member_pubkeys,
                        admin_pubkeys,
                        name.clone(),
                        desc,
                    ).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "group_id": group.mls_group_id,
                        "name": name,
                        "message": "Group created successfully"
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            GroupCommands::List => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let groups = self.app.groups.fetch_groups(account).await?;
                    let result = CommandResult::success(groups);
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            GroupCommands::Show { group_id } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let groups = self.app.groups.fetch_groups(account).await?;
                    if let Some(group) = groups.iter().find(|g| g.mls_group_id == group_id) {
                        let result = CommandResult::success(group);
                        self.format_output(&result)
                    } else {
                        let result = CommandResult::<()>::error("Group not found".to_string());
                        self.format_output(&result)
                    }
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            GroupCommands::Join { group_id: _ } => {
                let result = CommandResult::<()>::error("Join command requires interactive mode".to_string());
                self.format_output(&result)
            }
        }
    }

    async fn handle_message_command(&mut self, command: MessageCommands) -> Result<String> {
        match command {
            MessageCommands::Send { group_id, message, kind } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let group_id_obj = crate::groups::GroupManager::group_id_from_string(&group_id)?;
                    let sent_message = self.app.groups.send_message_to_group(
                        account,
                        &group_id_obj,
                        message.clone(),
                        kind,
                    ).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "group_id": group_id,
                        "message": message,
                        "message_id": sent_message.message.id.to_hex(),
                        "status": "sent"
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            MessageCommands::Dm { recipient, message } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let recipient_key = PublicKey::from_hex(&recipient)
                        .or_else(|_| PublicKey::parse(&recipient))?;

                    // Get or create DM group with recipient
                    let dm_group_id = self.app.groups.get_or_create_dm_group(
                        account,
                        &recipient_key,
                    ).await?;

                    // Send message to the DM group
                    let sent_message = self.app.groups.send_message_to_group(
                        account,
                        &dm_group_id,
                        message.clone(),
                        1, // Text message kind
                    ).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "recipient": recipient,
                        "message": message,
                        "dm_group_id": format!("{:?}", dm_group_id),
                        "message_id": sent_message.message.id.to_hex(),
                        "status": "sent"
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            MessageCommands::List { group_id, limit } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let group_id_obj = crate::groups::GroupManager::group_id_from_string(&group_id)?;
                    let messages = self.app.groups.fetch_aggregated_messages_for_group(
                        account,
                        &group_id_obj,
                    ).await?;

                    let limited_messages: Vec<_> = messages.iter().rev().take(limit).rev().collect();
                    let result = CommandResult::success(serde_json::json!({
                        "group_id": group_id,
                        "messages": limited_messages,
                        "count": limited_messages.len()
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            MessageCommands::ListDm { contact, limit } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let contact_key = PublicKey::from_hex(&contact)
                        .or_else(|_| PublicKey::parse(&contact))?;

                    // Get DM group with contact
                    if let Some(dm_group_id) = self.app.groups.find_dm_group(
                        account,
                        &contact_key,
                    ).await? {
                        // Fetch messages from the DM group
                        let messages = self.app.groups.fetch_aggregated_messages_for_group(
                            account,
                            &dm_group_id,
                        ).await?;

                        let limited_messages: Vec<_> = messages.iter().rev().take(limit).rev().collect();
                        let result = CommandResult::success(serde_json::json!({
                            "contact": contact,
                            "dm_group_id": format!("{:?}", dm_group_id),
                            "messages": limited_messages,
                            "count": limited_messages.len()
                        }));
                        self.format_output(&result)
                    } else {
                        let result = CommandResult::success(serde_json::json!({
                            "contact": contact,
                            "messages": [],
                            "count": 0,
                            "note": "No DM group found with this contact"
                        }));
                        self.format_output(&result)
                    }
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            MessageCommands::GetDmGroup { contact } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let contact_key = PublicKey::from_hex(&contact)
                        .or_else(|_| PublicKey::parse(&contact))?;

                    // Get or create DM group with contact
                    let dm_group_id = self.app.groups.get_or_create_dm_group(
                        account,
                        &contact_key,
                    ).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "contact": contact,
                        "dm_group_id": format!("{:?}", dm_group_id),
                        "created": true
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
        }
    }

    async fn handle_relay_command(&mut self, command: RelayCommands) -> Result<String> {
        match command {
            RelayCommands::List { relay_type } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let relay_types = if let Some(rt) = relay_type {
                        vec![self.parse_relay_type(&rt)?]
                    } else {
                        crate::relays::RelayManager::all_relay_types()
                    };

                    let mut relay_info = HashMap::new();
                    for rt in relay_types {
                        let relays = self.app.relays.fetch_relays(account.pubkey, rt).await?;
                        relay_info.insert(self.app.relays.relay_type_name(&rt), relays);
                    }

                    let result = CommandResult::success(relay_info);
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            RelayCommands::Add { url, relay_type } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let rt = self.parse_relay_type(&relay_type)?;
                    self.app.relays.add_relay_to_type(account, rt, url.clone()).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "url": url,
                        "relay_type": relay_type,
                        "message": "Relay added successfully"
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            RelayCommands::Remove { url, relay_type } => {
                if let Some(account) = self.app.account_manager.get_current_account() {
                    let rt = self.parse_relay_type(&relay_type)?;
                    self.app.relays.remove_relay_from_type(account, rt, &url).await?;

                    let result = CommandResult::success(serde_json::json!({
                        "url": url,
                        "relay_type": relay_type,
                        "message": "Relay removed successfully"
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error("No account logged in".to_string());
                    self.format_output(&result)
                }
            }
            RelayCommands::Test { url } => {
                let is_valid = self.app.relays.test_relay_connection(&url).await?;
                let result = CommandResult::success(serde_json::json!({
                    "url": url,
                    "valid": is_valid,
                    "status": if is_valid { "reachable" } else { "unreachable" }
                }));
                self.format_output(&result)
            }
        }
    }

    async fn handle_batch_command(&mut self, file_path: String) -> Result<String> {
        let content = std::fs::read_to_string(&file_path)?;
        let batch: BatchOperation = if file_path.ends_with(".json") {
            serde_json::from_str(&content)?
        } else {
            return Err(anyhow::anyhow!("Only JSON batch files are supported currently"));
        };

        let mut results = Vec::new();
        for operation in batch.operations {
            let result = self.execute_batch_operation(operation).await;
            results.push(result);
        }

        let batch_result = CommandResult::success(serde_json::json!({
            "batch_file": file_path,
            "operations": results.len(),
            "results": results
        }));
        self.format_output(&batch_result)
    }

    async fn handle_status_command(&mut self) -> Result<String> {
        let is_logged_in = self.app.account_manager.is_logged_in();
        let current_account = if is_logged_in {
            self.app.account_manager.get_current_account().map(|a| a.pubkey.to_hex())
        } else {
            None
        };

        let result = CommandResult::success(serde_json::json!({
            "logged_in": is_logged_in,
            "current_account": current_account,
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now()
        }));
        self.format_output(&result)
    }

    async fn handle_keys_command(&mut self, command: KeysCommands) -> Result<String> {
        let helper = KeyringHelper::new()?;
        
        match command {
            KeysCommands::Store { pubkey, privkey } => {
                // Validate pubkey
                let _ = PublicKey::from_hex(&pubkey)
                    .map_err(|e| anyhow::anyhow!("Invalid public key hex: {}", e))?;
                
                // Store the key
                helper.store_key(&pubkey, &privkey)?;
                
                let result = CommandResult::success(serde_json::json!({
                    "pubkey": pubkey,
                    "message": "Private key stored successfully"
                }));
                self.format_output(&result)
            }
            KeysCommands::Get { pubkey } => {
                if let Some(privkey) = helper.get_key(&pubkey)? {
                    let result = CommandResult::success(serde_json::json!({
                        "pubkey": pubkey,
                        "privkey": privkey
                    }));
                    self.format_output(&result)
                } else {
                    let result = CommandResult::<()>::error(format!("No key found for pubkey: {}", pubkey));
                    self.format_output(&result)
                }
            }
            KeysCommands::List => {
                let keys = helper.list_keys()?;
                let result = CommandResult::success(serde_json::json!({
                    "keys": keys,
                    "count": keys.len()
                }));
                self.format_output(&result)
            }
            KeysCommands::Remove { pubkey } => {
                helper.remove_key(&pubkey)?;
                let result = CommandResult::success(serde_json::json!({
                    "pubkey": pubkey,
                    "message": "Key removed successfully"
                }));
                self.format_output(&result)
            }
        }
    }

    async fn execute_batch_operation(&mut self, operation: BatchCommand) -> serde_json::Value {
        let result = match operation {
            BatchCommand::AccountCreate { name, about } => {
                self.handle_account_command(AccountCommands::Create { name, about }).await
            }
            BatchCommand::ContactAdd { pubkey, name } => {
                self.handle_contact_command(ContactCommands::Add { pubkey, name }).await
            }
            BatchCommand::GroupCreate { name, description, members } => {
                let members_str = members.map(|m| m.join(","));
                self.handle_group_command(GroupCommands::Create { name, description, members: members_str }).await
            }
            BatchCommand::MessageSend { group_id, message, kind } => {
                self.handle_message_command(MessageCommands::Send { 
                    group_id, 
                    message, 
                    kind: kind.unwrap_or(1) 
                }).await
            }
            BatchCommand::MessageDm { recipient, message } => {
                self.handle_message_command(MessageCommands::Dm { recipient, message }).await
            }
            BatchCommand::RelayAdd { url, relay_type } => {
                self.handle_relay_command(RelayCommands::Add { url, relay_type }).await
            }
        };

        match result {
            Ok(output) => serde_json::json!({"success": true, "output": output}),
            Err(e) => serde_json::json!({"success": false, "error": e.to_string()}),
        }
    }

    fn parse_relay_type(&self, relay_type: &str) -> Result<RelayType> {
        match relay_type.to_lowercase().as_str() {
            "nostr" => Ok(RelayType::Nostr),
            "inbox" => Ok(RelayType::Inbox),
            "keypackage" | "key_package" => Ok(RelayType::KeyPackage),
            _ => Err(anyhow::anyhow!("Invalid relay type: {}. Use 'nostr', 'inbox', or 'keypackage'", relay_type)),
        }
    }

    fn format_output<T: serde::Serialize>(&self, result: &CommandResult<T>) -> Result<String> {
        match self.output_format {
            OutputFormat::Json => Ok(serde_json::to_string_pretty(result)?),
            OutputFormat::Yaml => {
                // For now, output as JSON since YAML support requires additional dependency
                Ok(serde_json::to_string_pretty(result)?)
            }
            OutputFormat::Human => {
                if result.success {
                    if let Some(ref data) = result.data {
                        Ok(serde_json::to_string_pretty(data)?)
                    } else {
                        Ok("Operation completed successfully".to_string())
                    }
                } else {
                    Ok(format!("Error: {}", result.error.as_ref().unwrap_or(&"Unknown error".to_string())))
                }
            }
        }
    }
}

// Extension trait to add setup_default_relays method
trait AppExtensions {
    async fn setup_default_relays(&mut self, account: &whitenoise::Account) -> Result<()>;
}

impl AppExtensions for App {
    async fn setup_default_relays(&mut self, account: &whitenoise::Account) -> Result<()> {
        use crate::relays::RelayManager;
        
        for relay_type in RelayManager::all_relay_types() {
            let default_relays = match relay_type {
                RelayType::Nostr => vec![
                    "wss://relay.damus.io".to_string(),
                    "wss://relay.primal.net".to_string(),
                    "wss://nos.lol".to_string(),
                ],
                RelayType::Inbox => vec![
                    "wss://relay.damus.io".to_string(),
                    "wss://relay.primal.net".to_string(),
                ],
                RelayType::KeyPackage => vec![
                    "wss://relay.damus.io".to_string(),
                    "wss://nos.lol".to_string(),
                ],
            };
            
            if let Err(_) = self.relays.update_relays(account, relay_type, default_relays).await {
                // Ignore relay setup errors in CLI mode
            }
        }

        // Clean up unwanted relays
        if let Err(_) = self.relays.cleanup_unwanted_relays(account).await {
            // Ignore cleanup errors
        }

        // Publish key package
        if let Err(_) = self.relays.publish_key_package(account).await {
            // Ignore key package publishing errors
        }

        Ok(())
    }
}