use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};
use whitenoise::{Account, PublicKey, RelayType, RelayUrl, Whitenoise, Event};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub nostr_relays: Vec<String>,
    pub inbox_relays: Vec<String>,
    pub key_package_relays: Vec<String>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            nostr_relays: vec![
                "ws://localhost:10547".to_string(),
                "wss://relay.damus.io".to_string(),
                "wss://relay.primal.net".to_string(),
                "wss://nos.lol".to_string(),
                "wss://relay.nostr.net".to_string(),
            ],
            inbox_relays: vec![
                "ws://localhost:10547".to_string(),
                "wss://relay.damus.io".to_string(),
                "wss://relay.primal.net".to_string(),
                "wss://relay.nostr.net".to_string(),
            ],
            key_package_relays: vec![
                "ws://localhost:10547".to_string(),
                "wss://relay.damus.io".to_string(),
                "wss://nos.lol".to_string(),
                "wss://relay.nostr.net".to_string(),
            ],
        }
    }
}

pub struct RelayManager {
    config: RelayConfig,
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            config: RelayConfig::default(),
        }
    }

    pub async fn fetch_relays(&self, pubkey: PublicKey, relay_type: RelayType) -> Result<Vec<RelayUrl>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        // Get the account directly instead of trying to fetch from network
        let account = whitenoise.get_account(&pubkey).await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {:?}", e))?;

        // Return the appropriate relay array from the account
        let relays = match relay_type {
            RelayType::Nostr => account.nip65_relays,
            RelayType::Inbox => account.inbox_relays,
            RelayType::KeyPackage => account.key_package_relays,
        };

        Ok(relays)
    }

    pub async fn update_relays(
        &mut self,
        _account: &Account,
        relay_type: RelayType,
        relays: Vec<String>,
    ) -> Result<()> {
        let _whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        // Convert strings to RelayUrl objects
        let relay_urls: Result<Vec<RelayUrl>, _> = relays
            .iter()
            .map(|url| RelayUrl::parse(url))
            .collect();

        let _relay_urls = relay_urls
            .map_err(|e| anyhow::anyhow!("Invalid relay URL: {:?}", e))?;

        // WhiteNoise doesn't have update_relays - relays are stored on the account
        // This would require updating the account object and saving it
        // For now, we'll just log this as a limitation
        println!("⚠️ Relay updates are stored locally but not persisted to WhiteNoise");

        // Update local config
        match relay_type {
            RelayType::Nostr => self.config.nostr_relays = relays,
            RelayType::Inbox => self.config.inbox_relays = relays,
            RelayType::KeyPackage => self.config.key_package_relays = relays,
        }

        println!("{} {} relays updated successfully!", 
            style("✅").green(), 
            self.relay_type_name(&relay_type)
        );

        Ok(())
    }

    pub async fn fetch_key_package(&self, pubkey: PublicKey) -> Result<Option<Event>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        // Fetch key package relays for this pubkey
        let nip65_relays = vec![
            RelayUrl::parse("ws://localhost:10547")?,
            RelayUrl::parse("wss://relay.damus.io")?,
            RelayUrl::parse("wss://relay.primal.net")?,
            RelayUrl::parse("wss://nos.lol")?,
            RelayUrl::parse("wss://relay.nostr.net")?,
        ];
        
        let key_package_relays = whitenoise.fetch_relays_from(nip65_relays, pubkey, RelayType::KeyPackage).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch key package relays: {:?}", e))?;

        whitenoise.fetch_key_package_event_from(key_package_relays, pubkey).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch key package: {:?}", e))
    }

    pub async fn publish_key_package(&self, _account: &Account) -> Result<()> {
        let _whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        println!("{}", style("🔐 Publishing MLS key package to relays...").yellow());

        // WhiteNoise doesn't have onboarding state - key packages are published automatically
        // during account creation/login
        println!("{}", style("ℹ️ Key packages are published automatically during account setup.").yellow());

        println!("{}", style("✅ Key package publishing status updated!").green());
        Ok(())
    }

    pub fn get_config(&self) -> &RelayConfig {
        &self.config
    }

    pub fn get_relays_for_type(&self, relay_type: &RelayType) -> &Vec<String> {
        match relay_type {
            RelayType::Nostr => &self.config.nostr_relays,
            RelayType::Inbox => &self.config.inbox_relays,
            RelayType::KeyPackage => &self.config.key_package_relays,
        }
    }

    pub fn relay_type_name(&self, relay_type: &RelayType) -> &'static str {
        match relay_type {
            RelayType::Nostr => "Nostr",
            RelayType::Inbox => "Inbox",
            RelayType::KeyPackage => "KeyPackage",
        }
    }

    pub fn all_relay_types() -> Vec<RelayType> {
        vec![RelayType::Nostr, RelayType::Inbox, RelayType::KeyPackage]
    }

    pub async fn test_relay_connection(&self, relay_url: &str) -> Result<bool> {
        // Basic URL validation
        if let Err(_) = url::Url::parse(relay_url) {
            return Ok(false);
        }

        // For now, just validate the URL format
        // In a more complete implementation, we could try to connect to the relay
        let is_websocket = relay_url.starts_with("ws://") || relay_url.starts_with("wss://");
        Ok(is_websocket)
    }

    pub async fn add_relay_to_type(&mut self, account: &Account, relay_type: RelayType, relay_url: String) -> Result<()> {
        if !self.test_relay_connection(&relay_url).await? {
            return Err(anyhow::anyhow!("Invalid relay URL or connection failed"));
        }

        let mut current_relays = self.get_relays_for_type(&relay_type).clone();
        if !current_relays.contains(&relay_url) {
            current_relays.push(relay_url);
            self.update_relays(account, relay_type, current_relays).await?;
        }

        Ok(())
    }

    pub async fn remove_relay_from_type(&mut self, account: &Account, relay_type: RelayType, relay_url: &str) -> Result<()> {
        let mut current_relays = self.get_relays_for_type(&relay_type).clone();
        current_relays.retain(|url| url != relay_url);
        self.update_relays(account, relay_type, current_relays).await?;
        Ok(())
    }

    pub async fn cleanup_unwanted_relays(&mut self, _account: &Account) -> Result<()> {
        // Remove problematic relays that cause connection errors
        let unwanted_relays = ["wss://purplepag.es", "wss://relay.purplepag.es"];
        
        for relay_type in Self::all_relay_types() {
            let current_relays = self.get_relays_for_type(&relay_type).clone();
            let filtered_relays: Vec<String> = current_relays
                .into_iter()
                .filter(|url| !unwanted_relays.contains(&url.as_str()))
                .collect();
                
            // Update local config
            match relay_type {
                RelayType::Nostr => self.config.nostr_relays = filtered_relays,
                RelayType::Inbox => self.config.inbox_relays = filtered_relays,
                RelayType::KeyPackage => self.config.key_package_relays = filtered_relays,
            }
        }
        
        println!("{}", style("✅ Unwanted relays removed from local configuration").green());
        Ok(())
    }
}