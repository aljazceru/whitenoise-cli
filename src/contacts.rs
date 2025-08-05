use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use whitenoise::{PublicKey, Metadata, Whitenoise, Tag, RelayUrl, Account};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub public_key: String,
    pub metadata: Option<ContactMetadata>,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactMetadata {
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
}

impl ContactMetadata {
    pub fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            display_name: metadata.name.clone(),
            about: metadata.about.clone(),
            picture: metadata.picture.clone(),
            banner: metadata.banner.clone(),
            nip05: metadata.nip05.clone(),
            lud16: metadata.lud16.clone(),
        }
    }

    pub fn to_metadata(&self) -> Metadata {
        let mut metadata = Metadata::new();
        
        if let Some(name) = &self.display_name {
            metadata = metadata.name(name);
        }
        if let Some(about) = &self.about {
            metadata = metadata.about(about);
        }
        if let Some(picture) = &self.picture {
            if let Ok(url) = url::Url::parse(picture) {
                metadata = metadata.picture(url);
            }
        }
        if let Some(banner) = &self.banner {
            if let Ok(url) = url::Url::parse(banner) {
                metadata = metadata.banner(url);
            }
        }
        if let Some(nip05) = &self.nip05 {
            metadata = metadata.nip05(nip05);
        }
        if let Some(lud16) = &self.lud16 {
            metadata = metadata.lud16(lud16);
        }
        
        metadata
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ContactManager {
    contacts: HashMap<String, Contact>,
}

impl ContactManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch_contacts(&mut self, account_pubkey: PublicKey) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let contacts = whitenoise.fetch_contacts(&account_pubkey).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch contacts: {:?}", e))?;

        self.contacts.clear();
        for (pubkey, metadata_opt) in contacts {
            let contact = Contact {
                name: metadata_opt.as_ref()
                    .and_then(|m| m.name.clone())
                    .unwrap_or_else(|| pubkey.to_hex()[..16].to_string()),
                public_key: pubkey.to_hex(),
                metadata: metadata_opt.map(|m| ContactMetadata::from_metadata(&m)),
                added_at: chrono::Utc::now(),
            };
            self.contacts.insert(pubkey.to_hex(), contact);
        }

        Ok(())
    }

    pub async fn query_contacts(&mut self, account_pubkey: PublicKey) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let contacts = whitenoise.query_contacts(account_pubkey).await
            .map_err(|e| anyhow::anyhow!("Failed to query contacts: {:?}", e))?;

        self.contacts.clear();
        for (pubkey, metadata_opt) in contacts {
            let contact = Contact {
                name: metadata_opt.as_ref()
                    .and_then(|m| m.name.clone())
                    .unwrap_or_else(|| pubkey.to_hex()[..16].to_string()),
                public_key: pubkey.to_hex(),
                metadata: metadata_opt.map(|m| ContactMetadata::from_metadata(&m)),
                added_at: chrono::Utc::now(),
            };
            self.contacts.insert(pubkey.to_hex(), contact);
        }

        Ok(())
    }

    pub async fn send_direct_message(
        &self,
        sender_account: &Account,
        receiver: &PublicKey,
        content: String,
    ) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let tags: Vec<Tag> = Vec::new(); // Empty tags for now
        
        whitenoise
            .send_direct_message_nip04(sender_account, receiver, content, tags)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send direct message: {:?}", e))
    }

    pub async fn add(&mut self, name: String, public_key: String) -> Result<()> {
        // Parse the public key to validate it
        let pubkey = if public_key.starts_with("npub") {
            // Use parse method for npub format
            PublicKey::parse(&public_key)
                .map_err(|e| anyhow::anyhow!("Invalid npub format: {:?}", e))?
        } else {
            PublicKey::from_hex(&public_key)
                .map_err(|e| anyhow::anyhow!("Invalid hex format: {:?}", e))?
        };

        // Try to fetch metadata for this contact
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        // Include local relay for testing plus public relays
        let nip65_relays = vec![
            RelayUrl::parse("ws://localhost:10547")?,
            RelayUrl::parse("wss://relay.damus.io")?,
            RelayUrl::parse("wss://relay.primal.net")?,
            RelayUrl::parse("wss://nos.lol")?,
        ];
        
        let metadata = whitenoise.fetch_metadata_from(nip65_relays, pubkey).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch metadata: {:?}", e))?;

        let contact = Contact {
            name,
            public_key: pubkey.to_hex(),
            metadata: metadata.map(|m| ContactMetadata::from_metadata(&m)),
            added_at: chrono::Utc::now(),
        };

        self.contacts.insert(pubkey.to_hex(), contact);
        Ok(())
    }

    pub async fn remove(&mut self, public_key: &str) -> Result<()> {
        self.contacts.remove(public_key);
        Ok(())
    }

    pub fn get(&self, public_key: &str) -> Option<&Contact> {
        self.contacts.get(public_key)
    }

    pub fn list(&self) -> Vec<&Contact> {
        self.contacts.values().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }
}