use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};
use whitenoise::{
    Account, Group, GroupId, GroupState, GroupType, NostrGroupConfigData, PublicKey, Whitenoise,
    MessageWithTokens, ChatMessage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupData {
    pub mls_group_id: String,
    pub nostr_group_id: String,
    pub name: String,
    pub description: String,
    pub admin_pubkeys: Vec<String>,
    pub last_message_id: Option<String>,
    pub last_message_at: Option<u64>,
    pub group_type: GroupType,
    pub epoch: u64,
    pub state: GroupState,
}

impl GroupData {
    pub fn from_group(group: &Group) -> Self {
        Self {
            mls_group_id: hex::encode(group.mls_group_id.as_slice()),
            nostr_group_id: hex::encode(group.nostr_group_id),
            name: group.name.clone(),
            description: group.description.clone(),
            admin_pubkeys: group.admin_pubkeys.iter().map(|pk| pk.to_hex()).collect(),
            last_message_id: group.last_message_id.map(|id| id.to_hex()),
            last_message_at: group.last_message_at.map(|at| at.as_u64()),
            group_type: group.group_type,
            epoch: group.epoch,
            state: group.state,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageData {
    pub id: String,
    pub pubkey: String,
    pub content: String,
    pub created_at: u64,
    pub is_reply: bool,
    pub reply_to_id: Option<String>,
    pub is_deleted: bool,
    pub kind: u16,
}

impl MessageData {
    pub fn from_chat_message(message: &ChatMessage) -> Self {
        Self {
            id: message.id.clone(),
            pubkey: message.author.to_hex(),
            content: message.content.clone(),
            created_at: message.created_at.as_u64(),
            is_reply: message.is_reply,
            reply_to_id: message.reply_to_id.clone(),
            is_deleted: message.is_deleted,
            kind: message.kind,
        }
    }
}

pub struct GroupManager {
    current_groups: Vec<GroupData>,
}

impl GroupManager {
    pub fn new() -> Self {
        Self {
            current_groups: Vec::new(),
        }
    }

    pub async fn fetch_groups(&mut self, account: &Account) -> Result<Vec<GroupData>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let groups = whitenoise.fetch_groups(account, true).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch groups: {:?}", e))?;

        let group_data: Vec<GroupData> = groups.iter().map(GroupData::from_group).collect();
        self.current_groups = group_data.clone();
        Ok(group_data)
    }

    pub async fn create_group(
        &mut self,
        creator_account: &Account,
        member_pubkeys: Vec<PublicKey>,
        admin_pubkeys: Vec<PublicKey>,
        group_name: String,
        group_description: String,
    ) -> Result<GroupData> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        println!("{}", style("🔧 Creating MLS group...").yellow());

        // Use the creator's account relays directly for the group configuration
        // If the account has been fixed by fix_account_empty_relays, nip65_relays will be populated
        let nostr_relays = if creator_account.nip65_relays.is_empty() {
            // Fallback to trying to fetch from network if account relays are empty
            whitenoise
                .fetch_relays_from(creator_account.nip65_relays.clone(), creator_account.pubkey, whitenoise::RelayType::Nostr)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch relays: {:?}", e))?
        } else {
            // Use account's existing relays
            creator_account.nip65_relays.clone()
        };

        let nostr_group_config = NostrGroupConfigData {
            name: group_name,
            description: group_description,
            image_key: None,
            image_url: None,
            relays: nostr_relays,
        };

        let creator_account_clone = creator_account.clone();
        let group = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(whitenoise.create_group(
                &creator_account_clone,
                member_pubkeys,
                admin_pubkeys,
                nostr_group_config,
            ))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Failed to create group: {:?}", e))?;

        println!("{}", style("✅ Group created successfully!").green());
        let group_data = GroupData::from_group(&group);
        self.current_groups.push(group_data.clone());
        Ok(group_data)
    }

    pub async fn fetch_group_members(&self, account: &Account, group_id: &GroupId) -> Result<Vec<PublicKey>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        whitenoise.fetch_group_members(account, group_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch group members: {:?}", e))
    }

    pub async fn fetch_group_admins(&self, account: &Account, group_id: &GroupId) -> Result<Vec<PublicKey>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        whitenoise.fetch_group_admins(account, group_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch group admins: {:?}", e))
    }

    pub async fn add_members_to_group(
        &self,
        account: &Account,
        group_id: &GroupId,
        member_pubkeys: Vec<PublicKey>,
    ) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        println!("{}", style("👥 Adding members to group...").yellow());

        let account_clone = account.clone();
        let group_id_clone = group_id.clone();

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(whitenoise.add_members_to_group(
                &account_clone,
                &group_id_clone,
                member_pubkeys,
            ))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Failed to add members: {:?}", e))?;

        println!("{}", style("✅ Members added successfully!").green());
        Ok(())
    }

    pub async fn remove_members_from_group(
        &self,
        account: &Account,
        group_id: &GroupId,
        member_pubkeys: Vec<PublicKey>,
    ) -> Result<()> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        println!("{}", style("👥 Removing members from group...").yellow());

        let account_clone = account.clone();
        let group_id_clone = group_id.clone();

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(whitenoise.remove_members_from_group(
                &account_clone,
                &group_id_clone,
                member_pubkeys,
            ))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Failed to remove members: {:?}", e))?;

        println!("{}", style("✅ Members removed successfully!").green());
        Ok(())
    }

    pub async fn send_message_to_group(
        &self,
        account: &Account,
        group_id: &GroupId,
        message: String,
        kind: u16,
    ) -> Result<MessageWithTokens> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let account_clone = account.clone();
        let group_id_clone = group_id.clone();

        let message_with_tokens = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(whitenoise.send_message_to_group(
                &account_clone,
                &group_id_clone,
                message,
                kind,
                None, // tags
            ))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Failed to send message: {:?}", e))?;

        Ok(message_with_tokens)
    }

    pub async fn fetch_messages_for_group(
        &self,
        account: &Account,
        group_id: &GroupId,
    ) -> Result<Vec<MessageWithTokens>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        whitenoise.fetch_messages_for_group(account, group_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch messages: {:?}", e))
    }

    pub async fn fetch_aggregated_messages_for_group(
        &self,
        account: &Account,
        group_id: &GroupId,
    ) -> Result<Vec<ChatMessage>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        whitenoise.fetch_aggregated_messages_for_group(&account.pubkey, group_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch aggregated messages: {:?}", e))
    }

    pub fn group_id_from_string(group_id_str: &str) -> Result<GroupId> {
        let bytes = hex::decode(group_id_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode group ID: {}", e))?;
        Ok(GroupId::from_slice(&bytes))
    }

    pub fn group_id_to_string(group_id: &GroupId) -> String {
        hex::encode(group_id.as_slice())
    }

    pub fn get_groups(&self) -> &[GroupData] {
        &self.current_groups
    }

    pub async fn get_or_create_dm_group(
        &self,
        account: &Account,
        recipient: &PublicKey,
    ) -> Result<GroupId> {
        // First check if a DM group already exists
        if let Some(group_id) = self.find_dm_group(account, recipient).await? {
            return Ok(group_id);
        }

        // Create a new DM group
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let creator_pubkey = account.pubkey;
        let recipient_pubkey = *recipient;
        
        // Use the account's relays directly for the DM group configuration
        // If the account has been fixed by fix_account_empty_relays, nip65_relays will be populated
        let nostr_relays = if account.nip65_relays.is_empty() {
            // Fallback to trying to fetch from network if account relays are empty
            whitenoise
                .fetch_relays_from(account.nip65_relays.clone(), creator_pubkey, whitenoise::RelayType::Nostr)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch relays: {:?}", e))?
        } else {
            // Use account's existing relays
            account.nip65_relays.clone()
        };
        
        // Create a 2-person group for DM
        let group_config = NostrGroupConfigData {
            name: format!("DM with {}", &recipient.to_hex()[..8]),
            description: "Direct message conversation".to_string(),
            image_key: None,
            image_url: None,
            relays: nostr_relays,
        };

        // MLS protocol: creator should not be included in member list
        // The creator is automatically added by the MLS group creation process
        let member_pubkeys = vec![recipient_pubkey];
        let admin_pubkeys = vec![creator_pubkey, recipient_pubkey];
        
        let account_clone = account.clone();
        let group = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(whitenoise.create_group(
                &account_clone,
                member_pubkeys,
                admin_pubkeys,
                group_config,
            ))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
        .map_err(|e| anyhow::anyhow!("Failed to create DM group: {:?}", e))?;

        Ok(group.mls_group_id)
    }

    pub async fn find_dm_group(
        &self,
        account: &Account,
        recipient: &PublicKey,
    ) -> Result<Option<GroupId>> {
        let whitenoise = Whitenoise::get_instance()
            .map_err(|e| anyhow::anyhow!("Failed to get WhiteNoise instance: {:?}", e))?;

        let groups = whitenoise.fetch_groups(account, true).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch groups: {:?}", e))?;

        // Find a DM group that contains exactly the account and recipient
        for group in groups {
            if group.group_type == GroupType::DirectMessage {
                // Get group members
                let members = whitenoise.fetch_group_members(account, &group.mls_group_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch group members: {:?}", e))?;
                
                // Check if it's a DM between these two users
                if members.len() == 2 {
                    let member_pubkeys: Vec<PublicKey> = members.into_iter().collect();
                    if member_pubkeys.contains(&account.pubkey) && member_pubkeys.contains(recipient) {
                        return Ok(Some(group.mls_group_id));
                    }
                }
            }
        }

        Ok(None)
    }
}