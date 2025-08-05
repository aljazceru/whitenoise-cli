use anyhow::Result;
use std::path::PathBuf;
use whitenoise::{Whitenoise, WhitenoiseConfig};

pub struct WhitenoiseManager {
    config: WhitenoiseConfig,
    initialized: bool,
}

impl WhitenoiseManager {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whitenoise-cli")
            .join("data");

        let logs_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whitenoise-cli")
            .join("logs");

        // Create directories if they don't exist
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&logs_dir)?;

        let config = WhitenoiseConfig::new(&data_dir, &logs_dir);

        Ok(Self {
            config,
            initialized: false,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if !self.initialized {
            // Add a small delay to let tracing configuration take effect
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Set environment variable to suppress purplepag.es if possible
            std::env::set_var("WHITENOISE_SKIP_PURPLEPAGES", "1");
            
            Whitenoise::initialize_whitenoise(self.config.clone()).await
                .map_err(|e| anyhow::anyhow!("Failed to initialize WhiteNoise: {:?}", e))?;
            self.initialized = true;
            
            // Add a brief pause after initialization to let any initial errors settle
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(())
    }

    pub fn get_instance(&self) -> Result<&'static Whitenoise> {
        if !self.initialized {
            return Err(anyhow::anyhow!("WhiteNoise not initialized"));
        }
        Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))
    }

    pub async fn delete_all_data(&self) -> Result<()> {
        let whitenoise = self.get_instance()?;
        whitenoise.delete_all_data().await
            .map_err(|e| anyhow::anyhow!("Failed to delete all data: {:?}", e))
    }
}