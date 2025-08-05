use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select, Input, Confirm};
use whitenoise::{Account, PublicKey, RelayType, Metadata, Whitenoise};

use crate::{
    account::AccountManager, 
    contacts::ContactManager, 
    groups::{GroupManager, GroupData}, 
    relays::RelayManager,
    ui, 
    storage::Storage,
    whitenoise_config::WhitenoiseManager
};

pub struct App {
    pub account_manager: AccountManager,
    pub contacts: ContactManager,
    pub groups: GroupManager,
    pub relays: RelayManager,
    pub storage: Storage,
    pub term: Term,
    pub whitenoise_manager: WhitenoiseManager,
}

impl App {
    pub async fn new(whitenoise_manager: WhitenoiseManager) -> Result<Self> {
        let storage = Storage::new().await?;
        let account_manager = AccountManager::new().await?;
        let contacts = storage.load_contacts().await.unwrap_or_else(|_| ContactManager::new());
        let groups = GroupManager::new();
        let relays = RelayManager::new();
        
        Ok(Self {
            account_manager,
            contacts,
            groups,
            relays,
            storage,
            term: Term::stdout(),
            whitenoise_manager,
        })
    }

    pub async fn run_main_menu(&mut self) -> Result<bool> {
        self.term.clear_screen()?;
        
        if !self.account_manager.is_logged_in() {
            return self.account_setup_menu().await;
        }

        if let Some(account) = self.account_manager.get_current_account() {
            println!("{} {}", style("Logged in as:").bold(), style(&account.pubkey.to_hex()[..16]).green());
            if let Ok(Some(metadata)) = self.account_manager.get_metadata().await {
                if let Some(name) = &metadata.name {
                    println!("{} {}", style("Name:").dim(), style(name).dim());
                }
            }
            println!();
        }

        let options = vec![
            "💬 Group Conversations",
            "📩 Direct Messages",
            "👥 Manage Contacts", 
            "📡 Relay Settings",
            "🔑 Account Settings",
            "❌ Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.group_conversations_menu().await,
            1 => self.direct_messages_menu().await,
            2 => self.manage_contacts_menu().await,
            3 => self.relay_settings_menu().await,
            4 => self.account_settings_menu().await,
            5 => Ok(false),
            _ => Ok(true),
        }
    }

    async fn account_setup_menu(&mut self) -> Result<bool> {
        self.term.clear_screen()?;
        println!("{}", style("🆕 Account Setup").bold().cyan());
        println!();

        let options = vec![
            "🔑 Create New Identity",
            "🔓 Login with Existing Key",
            "📋 View All Accounts",
            "❌ Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Account Setup:")
            .items(&options)
            .interact()?;

        match selection {
            0 => self.create_new_identity().await,
            1 => self.login_existing_account().await,
            2 => self.view_all_accounts().await,
            3 => Ok(false),
            _ => Ok(true),
        }
    }

    async fn create_new_identity(&mut self) -> Result<bool> {
        println!("{}", style("🆕 Creating New Identity").bold().yellow());
        println!();
        
        let _account = self.account_manager.create_identity().await?;
        
        // Set up default relay configuration
        let current_account = self.account_manager.get_current_account().unwrap();
        for relay_type in RelayManager::all_relay_types() {
            let default_relays = match relay_type {
                RelayType::Nostr => vec![
                    "ws://localhost:10547".to_string(),
                    "wss://relay.damus.io".to_string(),
                    "wss://relay.primal.net".to_string(),
                    "wss://nos.lol".to_string(),
                ],
                RelayType::Inbox => vec![
                    "ws://localhost:10547".to_string(),
                    "wss://relay.damus.io".to_string(),
                    "wss://relay.primal.net".to_string(),
                ],
                RelayType::KeyPackage => vec![
                    "ws://localhost:10547".to_string(),
                    "wss://relay.damus.io".to_string(),
                    "wss://nos.lol".to_string(),
                ],
            };
            
            if let Err(e) = self.relays.update_relays(current_account, relay_type, default_relays).await {
                println!("{} Warning: Failed to set up {} relays: {}", 
                    style("⚠️").yellow(), 
                    self.relays.relay_type_name(&relay_type),
                    e
                );
            }
        }

        // Clean up unwanted relays (like purplepag.es that cause connection errors)
        if let Err(e) = self.relays.cleanup_unwanted_relays(current_account).await {
            println!("{} Warning: Failed to clean up unwanted relays: {}", style("⚠️").yellow(), e);
        }

        // Publish key package
        if let Err(e) = self.relays.publish_key_package(current_account).await {
            println!("{} Warning: Failed to publish key package: {}", style("⚠️").yellow(), e);
        }

        // Set up basic metadata
        self.setup_profile_metadata().await?;

        println!();
        println!("{}", style("🎉 Account setup complete! You can now start messaging.").bold().green());
        ui::wait_for_enter("Press Enter to continue...");
        Ok(true)
    }

    async fn setup_profile_metadata(&mut self) -> Result<()> {
        println!("{}", style("📝 Set up your profile (optional)").bold().cyan());
        println!();

        let name: String = Input::new()
            .with_prompt("Display name (leave empty to skip)")
            .allow_empty(true)
            .interact()?;

        let about: String = Input::new()
            .with_prompt("About (leave empty to skip)")
            .allow_empty(true)
            .interact()?;

        if !name.is_empty() || !about.is_empty() {
            let mut metadata = Metadata::new();
            
            if !name.is_empty() {
                metadata = metadata.name(&name);
            }
            if !about.is_empty() {
                metadata = metadata.about(&about);
            }

            match self.account_manager.update_metadata(&metadata).await {
                Ok(_) => println!("{} Profile updated successfully!", style("✅").green()),
                Err(e) => println!("{} Failed to update profile: {}", style("⚠️").yellow(), e),
            }
        }

        Ok(())
    }

    async fn login_existing_account(&mut self) -> Result<bool> {
        println!("{}", style("🔓 Login with Existing Key").bold().yellow());
        println!();

        let key: String = Input::new()
            .with_prompt("Enter your private key (nsec... or hex)")
            .interact()?;

        match self.account_manager.login(key).await {
            Ok(_) => {
                // Clean up unwanted relays after login
                if let Some(current_account) = self.account_manager.get_current_account() {
                    if let Err(_) = self.relays.cleanup_unwanted_relays(current_account).await {
                        // Silently ignore errors - cleanup is optional
                    }
                }
                
                println!();
                println!("{}", style("🎉 Login successful!").bold().green());
                ui::wait_for_enter("Press Enter to continue...");
                Ok(true)
            }
            Err(e) => {
                println!("{} Login failed: {}", style("❌").red(), e);
                ui::wait_for_enter("Press Enter to continue...");
                Ok(true)
            }
        }
    }

    async fn view_all_accounts(&mut self) -> Result<bool> {
        println!("{}", style("📋 All Accounts").bold().cyan());
        println!();

        match self.account_manager.fetch_accounts().await {
            Ok(accounts) => {
                if accounts.is_empty() {
                    println!("{}", style("No accounts found.").dim());
                } else {
                    for (i, account) in accounts.iter().enumerate() {
                        println!("{}. {}", 
                            style(format!("{}", i + 1)).bold(),
                            style(&account.pubkey[..16]).green()
                        );
                    }
                }
            }
            Err(e) => {
                println!("{} Failed to fetch accounts: {}", style("❌").red(), e);
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(true)
    }

    async fn group_conversations_menu(&mut self) -> Result<bool> {
        loop {
            self.term.clear_screen()?;
            println!("{}", style("💬 Group Conversations").bold().cyan());
            println!();

            // Fetch groups for current account
            if let Some(account) = self.account_manager.get_current_account() {
                match self.groups.fetch_groups(account).await {
                    Ok(groups) => {
                        if groups.is_empty() {
                            println!("{}", style("No groups yet. Create one to get started!").dim().italic());
                        } else {
                            println!("{}", style("Your Groups:").bold());
                            for (i, group) in groups.iter().enumerate() {
                                let last_message = group.last_message_at
                                    .map(|t| format!(" ({})", chrono::DateTime::from_timestamp(t as i64, 0)
                                        .unwrap_or_default()
                                        .format("%m/%d %H:%M")))
                                    .unwrap_or_default();
                                println!("{}. {} {} members{}", 
                                    style(format!("{}", i + 1)).bold(),
                                    style(&group.name).green(),
                                    style("📊").dim(),
                                    style(last_message).dim()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Failed to fetch groups: {}", style("❌").red(), e);
                    }
                }
            }

            println!();
            let options = vec![
                "💬 Join Group Chat",
                "➕ Create New Group",
                "👥 Manage Group Members",
                "🔙 Back to Main Menu",
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Group Options:")
                .items(&options)
                .interact()?;

            match selection {
                0 => self.join_group_chat().await?,
                1 => self.create_new_group().await?,
                2 => self.manage_group_members().await?,
                3 => return Ok(true),
                _ => {}
            }
        }
    }

    async fn join_group_chat(&mut self) -> Result<()> {
        let account_clone = if let Some(account) = self.account_manager.get_current_account() {
            account.clone()
        } else {
            return Ok(());
        };

        let groups = self.groups.fetch_groups(&account_clone).await?;
        
        if groups.is_empty() {
            println!("{}", style("No groups available to join.").yellow());
            ui::wait_for_enter("Press Enter to continue...");
            return Ok(());
        }

        let group_options: Vec<String> = groups
            .iter()
            .map(|g| format!("{} ({} members)", g.name, g.admin_pubkeys.len()))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select group to join:")
            .items(&group_options)
            .interact()?;

        let selected_group = groups[selection].clone();
        self.start_group_chat(&account_clone, &selected_group).await?;
        Ok(())
    }

    async fn start_group_chat(&mut self, account: &Account, group: &GroupData) -> Result<()> {
        let group_id = GroupManager::group_id_from_string(&group.mls_group_id)?;
        
        println!("{} Joining group '{}'...", style("🔄").yellow(), style(&group.name).bold());

        loop {
            self.term.clear_screen()?;
            
            println!("{}", style(format!("💬 Group Chat: {}", group.name)).bold().cyan());
            println!("{}", style("─".repeat(50)).dim());
            
            // Fetch and display recent messages
            match self.groups.fetch_aggregated_messages_for_group(account, &group_id).await {
                Ok(messages) => {
                    if messages.is_empty() {
                        println!("{}", style("No messages yet. Start the conversation!").dim().italic());
                    } else {
                        let recent_messages = messages.iter().rev().take(10).rev();
                        for msg in recent_messages {
                            let timestamp = chrono::DateTime::from_timestamp(msg.created_at.as_u64() as i64, 0)
                                .unwrap_or_default()
                                .format("%H:%M");
                            let author_short = &msg.author.to_hex()[..8];
                            println!("{} {} {}", 
                                style(format!("[{}]", timestamp)).dim(),
                                style(format!("{}:", author_short)).bold().blue(), 
                                msg.content
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("{} Failed to fetch messages: {}", style("❌").red(), e);
                }
            }
            
            println!("{}", style("─".repeat(50)).dim());
            println!();
            
            let input: String = Input::new()
                .with_prompt(&format!("💭 Message to {} (or 'quit' to exit)", group.name))
                .allow_empty(true)
                .interact()?;

            if input.trim().to_lowercase() == "quit" || input.trim().is_empty() {
                break;
            }

            match self.groups.send_message_to_group(account, &group_id, input.trim().to_string(), 9).await {
                Ok(_) => {
                    println!("{} Message sent!", style("✅").green());
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    println!("{} Failed to send message: {}", style("❌").red(), e);
                    ui::wait_for_enter("Press Enter to continue...");
                }
            }
        }

        Ok(())
    }

    async fn create_new_group(&mut self) -> Result<()> {
        if let Some(account) = self.account_manager.get_current_account() {
            println!("{}", style("➕ Create New Group").bold().green());
            println!();

            let group_name: String = Input::new()
                .with_prompt("Group name")
                .interact()?;

            let group_description: String = Input::new()
                .with_prompt("Group description")
                .allow_empty(true)
                .interact()?;

            // For now, create with just the creator
            let member_pubkeys = vec![account.pubkey];
            let admin_pubkeys = vec![account.pubkey];

            match self.groups.create_group(
                account,
                member_pubkeys,
                admin_pubkeys,
                group_name.clone(),
                group_description,
            ).await {
                Ok(_) => {
                    println!("{} Group '{}' created successfully!", style("✅").green(), group_name);
                }
                Err(e) => {
                    println!("{} Failed to create group: {}", style("❌").red(), e);
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn manage_group_members(&mut self) -> Result<()> {
        println!("{}", style("👥 Group member management not yet implemented").yellow());
        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn direct_messages_menu(&mut self) -> Result<bool> {
        loop {
            self.term.clear_screen()?;
            println!("{}", style("📩 Direct Messages").bold().cyan());
            println!();

            let options = vec![
                "💬 Send Direct Message",
                "📋 Fetch Contacts",
                "🔙 Back to Main Menu",
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Direct Message Options:")
                .items(&options)
                .interact()?;

            match selection {
                0 => self.send_direct_message().await?,
                1 => self.fetch_contacts().await?,
                2 => return Ok(true),
                _ => {}
            }
        }
    }

    async fn send_direct_message(&mut self) -> Result<()> {
        if let Some(account) = self.account_manager.get_current_account() {
            if self.contacts.is_empty() {
                println!("{}", style("No contacts found. Fetch contacts first!").yellow());
                ui::wait_for_enter("Press Enter to continue...");
                return Ok(());
            }

            let contacts = self.contacts.list();
            let contact_options: Vec<String> = contacts
                .iter()
                .map(|c| format!("{} ({})", c.name, &c.public_key[..16]))
                .collect();

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select contact to message:")
                .items(&contact_options)
                .interact()?;

            let selected_contact = &contacts[selection];
            let receiver_pubkey = PublicKey::from_hex(&selected_contact.public_key)?;

            let message: String = Input::new()
                .with_prompt("Message")
                .interact()?;

            match self.contacts.send_direct_message(account, &receiver_pubkey, message).await {
                Ok(_) => {
                    println!("{} Direct message sent!", style("✅").green());
                }
                Err(e) => {
                    println!("{} Failed to send direct message: {}", style("❌").red(), e);
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn fetch_contacts(&mut self) -> Result<()> {
        if let Some(account) = self.account_manager.get_current_account() {
            println!("{}", style("📡 Fetching contacts from relays...").yellow());
            
            match self.contacts.fetch_contacts(account.pubkey).await {
                Ok(_) => {
                    println!("{} Contacts fetched successfully!", style("✅").green());
                    println!("{} Found {} contacts", style("📊").dim(), self.contacts.list().len());
                }
                Err(e) => {
                    println!("{} Failed to fetch contacts: {}", style("❌").red(), e);
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn manage_contacts_menu(&mut self) -> Result<bool> {
        loop {
            self.term.clear_screen()?;
            println!("{}", style("👥 Manage Contacts").bold().cyan());
            println!();

            let mut options = vec![
                "📡 Fetch Contacts from Relays",
                "➕ Add Manual Contact",
                "📋 List All Contacts",
            ];

            if !self.contacts.is_empty() {
                options.push("🗑️  Remove Contact");
            }
            
            options.push("🔙 Back to Main Menu");

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Contact Management:")
                .items(&options)
                .interact()?;

            match selection {
                0 => self.fetch_contacts().await?,
                1 => self.add_manual_contact().await?,
                2 => self.list_contacts().await?,
                3 if !self.contacts.is_empty() => self.remove_contact().await?,
                _ => return Ok(true),
            }
        }
    }

    async fn add_manual_contact(&mut self) -> Result<()> {
        println!("{}", style("➕ Add Manual Contact").bold().green());
        println!();

        let name: String = Input::new()
            .with_prompt("Contact name")
            .interact()?;

        let public_key: String = Input::new()
            .with_prompt("Contact's public key (npub... or hex)")
            .interact()?;

        match self.contacts.add(name.clone(), public_key).await {
            Ok(_) => {
                println!("{} Contact '{}' added successfully!", style("✅").green(), name);
            }
            Err(e) => {
                println!("{} Failed to add contact: {}", style("❌").red(), e);
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn list_contacts(&self) -> Result<()> {
        self.term.clear_screen()?;
        println!("{}", style("📋 Your Contacts").bold().cyan());
        println!();

        if self.contacts.is_empty() {
            println!("{}", style("No contacts yet. Fetch contacts or add them manually!").dim().italic());
        } else {
            for (i, contact) in self.contacts.list().iter().enumerate() {
                let display_name = contact.metadata.as_ref()
                    .and_then(|m| m.display_name.as_ref())
                    .unwrap_or(&contact.name);
                
                println!("{}. {} {}", 
                    style(format!("{}", i + 1)).bold(),
                    style(display_name).green(),
                    style(format!("({})", &contact.public_key[..16])).dim()
                );
                
                if let Some(metadata) = &contact.metadata {
                    if let Some(about) = &metadata.about {
                        println!("   {}", style(about).dim().italic());
                    }
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn remove_contact(&mut self) -> Result<()> {
        println!("{}", style("🗑️  Remove Contact").bold().red());
        println!();

        let contacts = self.contacts.list();
        let contact_options: Vec<String> = contacts
            .iter()
            .map(|c| format!("{} ({})", c.name, &c.public_key[..16]))
            .collect();

        if contact_options.is_empty() {
            println!("No contacts to remove.");
            ui::wait_for_enter("Press Enter to continue...");
            return Ok(());
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select contact to remove:")
            .items(&contact_options)
            .interact()?;

        let contact_to_remove = contacts[selection].clone();
        
        let confirm = Confirm::new()
            .with_prompt(&format!("Are you sure you want to remove '{}'?", contact_to_remove.name))
            .default(false)
            .interact()?;

        if confirm {
            self.contacts.remove(&contact_to_remove.public_key).await?;
            println!("{} Contact removed successfully!", style("✅").green());
        } else {
            println!("Cancelled.");
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn relay_settings_menu(&mut self) -> Result<bool> {
        loop {
            self.term.clear_screen()?;
            println!("{}", style("📡 Relay Settings").bold().cyan());
            println!();

            let options = vec![
                "📋 View Current Relays",
                "➕ Add Relay",
                "🗑️  Remove Relay",
                "🔙 Back to Main Menu",
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Relay Management:")
                .items(&options)
                .interact()?;

            match selection {
                0 => self.view_current_relays().await?,
                1 => self.add_relay().await?,
                2 => self.remove_relay().await?,
                3 => return Ok(true),
                _ => {}
            }
        }
    }

    async fn view_current_relays(&mut self) -> Result<()> {
        if let Some(account) = self.account_manager.get_current_account() {
            println!("{}", style("📋 Current Relay Configuration").bold().cyan());
            println!();

            for relay_type in RelayManager::all_relay_types() {
                println!("{} {}:", style("📡").bold(), self.relays.relay_type_name(&relay_type));
                
                match self.relays.fetch_relays(account.pubkey, relay_type).await {
                    Ok(relay_urls) => {
                        if relay_urls.is_empty() {
                            println!("  {}", style("None configured").dim());
                        } else {
                            for relay_url in relay_urls {
                                println!("  • {}", style(relay_url.to_string()).green());
                            }
                        }
                    }
                    Err(e) => {
                        println!("  {}", style(format!("Error: {}", e)).red());
                    }
                }
                println!();
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn add_relay(&mut self) -> Result<()> {
        if let Some(account) = self.account_manager.get_current_account() {
            println!("{}", style("➕ Add Relay").bold().green());
            println!();

            let relay_type_options = vec!["Nostr", "Inbox", "KeyPackage"];
            let type_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select relay type:")
                .items(&relay_type_options)
                .interact()?;

            let relay_type = RelayManager::all_relay_types()[type_selection];

            let relay_url: String = Input::new()
                .with_prompt("Relay URL (wss://...)")
                .interact()?;

            match self.relays.add_relay_to_type(account, relay_type, relay_url).await {
                Ok(_) => {
                    println!("{} Relay added successfully!", style("✅").green());
                }
                Err(e) => {
                    println!("{} Failed to add relay: {}", style("❌").red(), e);
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn remove_relay(&mut self) -> Result<()> {
        println!("{}", style("🗑️  Remove relay functionality not yet implemented").yellow());
        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn account_settings_menu(&mut self) -> Result<bool> {
        loop {
            self.term.clear_screen()?;
            println!("{}", style("🔑 Account Settings").bold().cyan());
            println!();

            if let Some(account) = self.account_manager.get_current_account() {
                println!("{} {}", style("Public Key:").bold(), style(&account.pubkey.to_hex()).dim());
                
                if let Ok(Some(metadata)) = self.account_manager.get_metadata().await {
                    if let Some(name) = &metadata.name {
                        println!("{} {}", style("Name:").bold(), name);
                    }
                    if let Some(about) = &metadata.about {
                        println!("{} {}", style("About:").bold(), about);
                    }
                }
                println!();
            }

            let options = vec![
                "📝 Update Profile",
                "📋 Export Public Key (npub)",
                "🔐 Export Private Key (nsec)",
                "🚪 Logout",
                "🔙 Back to Main Menu",
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Account Options:")
                .items(&options)
                .interact()?;

            match selection {
                0 => self.update_profile().await?,
                1 => self.export_public_key().await?,
                2 => self.export_private_key().await?,
                3 => {
                    self.account_manager.logout().await?;
                    return Ok(true);
                }
                4 => return Ok(true),
                _ => {}
            }
        }
    }

    async fn update_profile(&mut self) -> Result<()> {
        println!("{}", style("📝 Update Profile").bold().cyan());
        println!();

        let current_metadata = self.account_manager.get_metadata().await?;

        let name: String = Input::new()
            .with_prompt("Display name")
            .with_initial_text(
                current_metadata.as_ref()
                    .and_then(|m| m.name.as_ref())
                    .unwrap_or(&String::new())
            )
            .allow_empty(true)
            .interact()?;

        let about: String = Input::new()
            .with_prompt("About")
            .with_initial_text(
                current_metadata.as_ref()
                    .and_then(|m| m.about.as_ref())
                    .unwrap_or(&String::new())
            )
            .allow_empty(true)
            .interact()?;

        let mut metadata = Metadata::new();
        if !name.is_empty() {
            metadata = metadata.name(&name);
        }
        if !about.is_empty() {
            metadata = metadata.about(&about);
        }

        match self.account_manager.update_metadata(&metadata).await {
            Ok(_) => {
                println!("{} Profile updated successfully!", style("✅").green());
            }
            Err(e) => {
                println!("{} Failed to update profile: {}", style("❌").red(), e);
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn export_public_key(&self) -> Result<()> {
        match self.account_manager.export_npub().await {
            Ok(npub) => {
                println!("{}", style("📋 Your Public Key (npub):").bold());
                println!("{}", style(&npub).green());
                println!();
                println!("💡 Share this with people who want to message you securely.");
            }
            Err(e) => {
                println!("{} Failed to export public key: {}", style("❌").red(), e);
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    async fn export_private_key(&self) -> Result<()> {
        let confirm = Confirm::new()
            .with_prompt("⚠️  This will show your private key. Make sure nobody else can see your screen. Continue?")
            .default(false)
            .interact()?;

        if confirm {
            match self.account_manager.export_nsec().await {
                Ok(nsec) => {
                    println!("{}", style("🔐 Your Private Key (nsec):").bold().red());
                    println!("{}", style(&nsec).red());
                    println!();
                    println!("{}", style("⚠️  NEVER share this with anyone! Save it securely.").bold().red());
                }
                Err(e) => {
                    println!("{} Failed to export private key: {}", style("❌").red(), e);
                }
            }
        }

        ui::wait_for_enter("Press Enter to continue...");
        Ok(())
    }

    /// Auto-login with a specific account by public key
    pub async fn auto_login_by_pubkey(&mut self, pubkey_hex: &str) -> Result<()> {
        // Parse the pubkey
        let pubkey = PublicKey::from_hex(pubkey_hex)
            .map_err(|e| anyhow::anyhow!("Invalid public key hex: {}", e))?;
        
        // Fetch all accounts to find the matching one
        let accounts = self.account_manager.fetch_accounts().await?;
        
        // Find the account with matching pubkey
        let matching_account = accounts.iter()
            .find(|acc| acc.pubkey == pubkey.to_hex());
        
        if matching_account.is_none() {
            return Err(anyhow::anyhow!("No account found with pubkey: {}", pubkey_hex));
        }
        
        // Get the account from WhiteNoise database
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
        
        // Fetch all accounts and find the one with matching pubkey
        let accounts = whitenoise.fetch_accounts().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch accounts: {:?}", e))?;
        
        let mut account = accounts.get(&pubkey)
            .ok_or_else(|| anyhow::anyhow!("No account found with pubkey: {}", pubkey_hex))?
            .clone();
        
        // Fix empty relay arrays if present (needed for accounts affected by DB migration)
        if let Ok(_) = whitenoise.fix_account_empty_relays(&mut account).await {
            // Silent fix for CLI operations
        }
        
        // Set as current account in account manager
        self.account_manager.set_current_account(account);
        
        // Note: Private keys are stored in system keyring by WhiteNoise
        // In environments without keyring, you'll need to use interactive mode
        // or modify WhiteNoise to use file-based storage
        
        Ok(())
    }
}