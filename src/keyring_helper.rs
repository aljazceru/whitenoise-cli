use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct FileKeyStore {
    version: u32,
    keys: HashMap<String, String>,
}

pub struct KeyringHelper {
    store_path: PathBuf,
}

impl KeyringHelper {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory found"))?;
        let store_path = home.join(".whitenoise_keys.json");
        
        Ok(Self { store_path })
    }
    
    pub fn store_key(&self, pubkey: &str, privkey: &str) -> Result<()> {
        let mut store = self.load_store()?;
        
        // Simple obfuscation - not secure but matches WhiteNoise approach
        let obfuscated = self.obfuscate(privkey);
        store.keys.insert(pubkey.to_string(), obfuscated);
        
        self.save_store(&store)?;
        Ok(())
    }
    
    pub fn get_key(&self, pubkey: &str) -> Result<Option<String>> {
        let store = self.load_store()?;
        
        if let Some(obfuscated) = store.keys.get(pubkey) {
            let privkey = self.deobfuscate(obfuscated)?;
            Ok(Some(privkey))
        } else {
            Ok(None)
        }
    }
    
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let store = self.load_store()?;
        Ok(store.keys.keys().cloned().collect())
    }
    
    pub fn remove_key(&self, pubkey: &str) -> Result<()> {
        let mut store = self.load_store()?;
        store.keys.remove(pubkey);
        self.save_store(&store)?;
        Ok(())
    }
    
    fn load_store(&self) -> Result<FileKeyStore> {
        if self.store_path.exists() {
            let content = fs::read_to_string(&self.store_path)?;
            let store: FileKeyStore = serde_json::from_str(&content)?;
            Ok(store)
        } else {
            Ok(FileKeyStore {
                version: 1,
                keys: HashMap::new(),
            })
        }
    }
    
    fn save_store(&self, store: &FileKeyStore) -> Result<()> {
        let content = serde_json::to_string_pretty(store)?;
        fs::write(&self.store_path, content)?;
        
        // Set file permissions to 0600 (read/write for owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&self.store_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.store_path, perms)?;
        }
        
        Ok(())
    }
    
    fn obfuscate(&self, data: &str) -> String {
        // Simple XOR obfuscation with a fixed key
        let key = b"WhiteNoiseCLI2024";
        let data_bytes = data.as_bytes();
        let mut obfuscated = Vec::with_capacity(data_bytes.len());
        
        for (i, &byte) in data_bytes.iter().enumerate() {
            obfuscated.push(byte ^ key[i % key.len()]);
        }
        
        general_purpose::STANDARD.encode(&obfuscated)
    }
    
    fn deobfuscate(&self, obfuscated: &str) -> Result<String> {
        let key = b"WhiteNoiseCLI2024";
        let data = general_purpose::STANDARD.decode(obfuscated)?;
        let mut deobfuscated = Vec::with_capacity(data.len());
        
        for (i, &byte) in data.iter().enumerate() {
            deobfuscated.push(byte ^ key[i % key.len()]);
        }
        
        Ok(String::from_utf8(deobfuscated)?)
    }
}

// Environment setup for keyring-less operation
pub fn setup_keyring_environment() -> Result<()> {
    // Set environment variables to use file storage instead of keyring
    std::env::set_var("WHITENOISE_FILE_STORAGE", "1");
    std::env::set_var("WHITENOISE_NO_KEYRING", "1");
    
    // Create dummy D-Bus session for environments without it
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "disabled:");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_obfuscation() {
        let helper = KeyringHelper::new().unwrap();
        let original = "test_private_key_12345";
        let obfuscated = helper.obfuscate(original);
        let deobfuscated = helper.deobfuscate(&obfuscated).unwrap();
        assert_eq!(original, deobfuscated);
    }
}