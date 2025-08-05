use anyhow::Result;
use std::path::PathBuf;

use crate::contacts::ContactManager;

pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    pub async fn new() -> Result<Self> {
        // Use current working directory for folder-based persistence
        let data_dir = std::env::current_dir()?
            .join(".whitenoise-cli");

        std::fs::create_dir_all(&data_dir)?;

        Ok(Self { data_dir })
    }

    pub async fn new_global() -> Result<Self> {
        // Use global data directory for global persistence
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
            .join("whitenoise-cli");

        std::fs::create_dir_all(&data_dir)?;

        Ok(Self { data_dir })
    }

    pub async fn save_contacts(&self, contacts: &ContactManager) -> Result<()> {
        let path = self.data_dir.join("contacts.json");
        let json = serde_json::to_string_pretty(contacts)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub async fn load_contacts(&self) -> Result<ContactManager> {
        let path = self.data_dir.join("contacts.json");
        if !path.exists() {
            return Ok(ContactManager::new());
        }

        let json = std::fs::read_to_string(path)?;
        let contacts = serde_json::from_str(&json)?;
        Ok(contacts)
    }

    pub async fn save_current_account_pubkey(&self, pubkey: &str) -> Result<()> {
        let path = self.data_dir.join("current_account_pubkey.txt");
        std::fs::write(path, pubkey)?;
        Ok(())
    }

    pub async fn load_current_account_pubkey(&self) -> Result<Option<String>> {
        let path = self.data_dir.join("current_account_pubkey.txt");
        if !path.exists() {
            return Ok(None);
        }

        let pubkey = std::fs::read_to_string(path)?;
        Ok(Some(pubkey.trim().to_string()))
    }

    pub async fn clear_current_account(&self) -> Result<()> {
        let path = self.data_dir.join("current_account_pubkey.txt");
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}