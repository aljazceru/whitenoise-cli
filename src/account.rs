use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};
use whitenoise::{Account, AccountSettings, Metadata, Whitenoise};

use crate::storage::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub pubkey: String,
    pub settings: AccountSettings,
    pub last_synced: u64,
}

impl AccountData {
    pub fn from_account(account: &Account) -> Self {
        Self {
            pubkey: account.pubkey.to_hex(),
            settings: account.settings.clone(),
            last_synced: account.last_synced.as_u64(),
        }
    }
}

pub struct AccountManager {
    current_account: Option<Account>,
    storage: Storage,
}

impl AccountManager {
    pub async fn new() -> Result<Self> {
        let storage = Storage::new().await?;
        let mut manager = Self {
            current_account: None,
            storage,
        };
        
        // Try to auto-login with stored pubkey
        if let Some(pubkey) = manager.storage.load_current_account_pubkey().await? {
            if let Ok(_) = manager.auto_login_by_pubkey(&pubkey).await {
                // Successfully logged in
            }
        }
        
        Ok(manager)
    }
    
    pub async fn auto_login_by_pubkey(&mut self, pubkey: &str) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
        
        // Try to parse the pubkey and get the account from WhiteNoise
        if let Ok(public_key) = whitenoise::PublicKey::from_hex(pubkey) {
            if let Ok(mut account) = whitenoise.get_account(&public_key).await {
                // Fix empty relay arrays if present (needed for accounts affected by DB migration)
                println!("{}", style("🔍 Auto-login: Checking relay configuration...").blue());
                println!("Account nip65_relays count: {}", account.nip65_relays.len());
                if let Ok(updated) = whitenoise.fix_account_empty_relays(&mut account).await {
                    if updated {
                        println!("{}", style("🔧 Auto-login: Fixed empty relay configuration").yellow());
                        println!("Account nip65_relays count after fix: {}", account.nip65_relays.len());
                    } else {
                        println!("{}", style("🔗 Auto-login: Connected existing relays to NostrManager").blue());
                    }
                } else {
                    println!("{}", style("⚠️ Auto-login: Failed to fix/connect relays").red());
                }
                self.current_account = Some(account);
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("Account not found for pubkey: {}", pubkey))
    }

    pub async fn fetch_accounts(&self) -> Result<Vec<AccountData>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
        
        let accounts = whitenoise.fetch_accounts().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch accounts: {:?}", e))?;
        
        Ok(accounts.values().map(AccountData::from_account).collect())
    }

    pub async fn create_identity(&mut self) -> Result<Account> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
        
        println!("{}", style("🔐 Generating cryptographic keys and MLS credentials...").yellow());
        
        let account = whitenoise.create_identity().await
            .map_err(|e| anyhow::anyhow!("Failed to create identity: {:?}", e))?;
        
        println!("{}", style("✅ Identity created successfully!").green());
        println!("{} {}", style("Public Key (hex):").bold(), style(&account.pubkey.to_hex()).dim());
        
        self.current_account = Some(account.clone());
        Ok(account)
    }

    pub async fn login(&mut self, nsec_or_hex_privkey: String) -> Result<Account> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
        
        println!("{}", style("🔑 Logging in...").yellow());
        
        let mut account = whitenoise.login(nsec_or_hex_privkey).await
            .map_err(|e| anyhow::anyhow!("Failed to login: {:?}", e))?;
        
        // Fix empty relay arrays if present (needed for accounts affected by DB migration)
        println!("{}", style("🔍 Checking relay configuration...").blue());
        println!("Account nip65_relays count: {}", account.nip65_relays.len());
        if let Ok(updated) = whitenoise.fix_account_empty_relays(&mut account).await {
            if updated {
                println!("{}", style("🔧 Fixed empty relay configuration").yellow());
                println!("Account nip65_relays count after fix: {}", account.nip65_relays.len());
            } else {
                println!("{}", style("ℹ️ Relay configuration already valid").blue());
            }
        } else {
            println!("{}", style("⚠️ Failed to fix relay configuration").red());
        }
        
        println!("{}", style("✅ Login successful!").green());
        println!("{} {}", style("Public Key:").bold(), style(&account.pubkey.to_hex()).dim());
        
        self.current_account = Some(account.clone());
        
        // Save the current account pubkey to storage for persistence
        self.storage.save_current_account_pubkey(&account.pubkey.to_hex()).await?;
        
        Ok(account)
    }

    pub async fn logout(&mut self) -> Result<()> {
        if let Some(account) = &self.current_account {
            let whitenoise = Whitenoise::get_instance()
                .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
            
            whitenoise.logout(&account.pubkey).await
                .map_err(|e| anyhow::anyhow!("Failed to logout: {:?}", e))?;
            
            self.current_account = None;
            
            // Clear the saved account from storage
            self.storage.clear_current_account().await?;
            
            println!("{}", style("✅ Logged out successfully!").green());
        }
        Ok(())
    }

    pub async fn export_nsec(&self) -> Result<String> {
        if let Some(account) = &self.current_account {
            let whitenoise = Whitenoise::get_instance()
                .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
            
            whitenoise.export_account_nsec(account).await
                .map_err(|e| anyhow::anyhow!("Failed to export nsec: {:?}", e))
        } else {
            Err(anyhow::anyhow!("No account logged in"))
        }
    }

    pub async fn export_npub(&self) -> Result<String> {
        if let Some(account) = &self.current_account {
            let whitenoise = Whitenoise::get_instance()
                .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
            
            whitenoise.export_account_npub(account).await
                .map_err(|e| anyhow::anyhow!("Failed to export npub: {:?}", e))
        } else {
            Err(anyhow::anyhow!("No account logged in"))
        }
    }

    pub async fn get_metadata(&self) -> Result<Option<Metadata>> {
        if let Some(account) = &self.current_account {
            let whitenoise = Whitenoise::get_instance()
                .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
            
            // Use the account's relays to fetch metadata
            whitenoise.fetch_metadata_from(account.nip65_relays.clone(), account.pubkey).await
                .map_err(|e| anyhow::anyhow!("Failed to fetch metadata: {:?}", e))
        } else {
            Err(anyhow::anyhow!("No account logged in"))
        }
    }

    pub async fn update_metadata(&self, metadata: &Metadata) -> Result<()> {
        if let Some(account) = &self.current_account {
            let whitenoise = Whitenoise::get_instance()
                .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;
            
            whitenoise.update_metadata(metadata, account).await
                .map_err(|e| anyhow::anyhow!("Failed to update metadata: {:?}", e))
        } else {
            Err(anyhow::anyhow!("No account logged in"))
        }
    }


    pub fn get_current_account(&self) -> Option<&Account> {
        self.current_account.as_ref()
    }

    pub fn is_logged_in(&self) -> bool {
        self.current_account.is_some()
    }

    pub fn set_current_account(&mut self, account: Account) {
        self.current_account = Some(account);
    }
}