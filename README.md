# WhiteNoise CLI - Interactive Secure Messaging

A command-line interface for secure messaging using the WhiteNoise protocol (Nostr + MLS), fully compatible with the WhiteNoise Flutter client.

## Features

- 🔐 **Secure Identity Management**: Generate and manage cryptographic identities with MLS credentials
- 👥 **Contact Management**: Add, list, and remove contacts with metadata support
- 💬 **MLS-based Messaging**: Direct messages and group chats using MLS encryption
- 🤖 **Automation Support**: Full CLI mode for scripting and automation
- 📱 **Flutter Compatible**: 100% compatible with WhiteNoise Flutter client
- 🔄 **Multi-Relay Support**: Nostr, Inbox, and KeyPackage relay types
- 💾 **Persistent Storage**: WhiteNoise database for account persistence

## Installation & Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```
   Note: Requires Rust 1.82+ for async trait support

3. **Run the CLI**:
   ```bash
   # Interactive mode
   cargo run
   
   # CLI mode (see CLI Commands section)
   cargo run -- --help
   ```

## Usage

### Interactive Mode
Run without arguments to enter interactive mode with menus.

### CLI Mode
Use command-line arguments for automation:

```bash
# Create account with profile
./whitenoise-cli account create --name "Alice" --about "Decentralized messaging fan"

# Send direct message (creates MLS DM group)
./whitenoise-cli message dm --recipient <pubkey> --message "Hello!"

# Create group chat
./whitenoise-cli group create --name "My Group" --members "pubkey1,pubkey2,pubkey3"

# Send group message
./whitenoise-cli message send --group-id <group_id> --message "Hello group!"
```

### Main Menu Options (Interactive Mode)

1. **💬 Start Conversation**
   - Select from your contacts
   - Chat in real-time with a secure interface
   - Type messages and press Enter to send
   - Type 'quit' or press Enter on empty input to exit chat

2. **👥 Manage Contacts**
   - **➕ Add New Contact**: Add contacts by name and public key
   - **📋 List All Contacts**: View all your contacts
   - **🗑️ Remove Contact**: Remove contacts from your list

3. **🔑 Identity Settings**
   - **📝 Change Name**: Update your display name
   - **📋 Copy Public Key**: View and copy your public key to share
   - **🔄 Generate New Identity**: Create a new identity (warning: loses access to existing conversations)

4. **❌ Exit**: Quit the application

### Chat Interface
- View recent message history
- Send messages in real-time
- Messages show timestamps and sender names
- Clean, colorful interface with proper formatting

## Data Storage

The CLI stores data locally in your system's data directory:
- **Identity**: `~/.local/share/whitenoise-cli/identity.json`
- **Contacts**: `~/.local/share/whitenoise-cli/contacts.json`

## Architecture

```
src/
├── main.rs               # Entry point with CLI/interactive routing
├── app.rs                # Main application state and WhiteNoise integration
├── cli.rs                # CLI command definitions (clap)
├── cli_handler.rs        # CLI command execution
├── account.rs            # Account management with WhiteNoise
├── contacts.rs           # Contact management with metadata
├── groups.rs             # MLS group creation and messaging
├── relays.rs             # Multi-type relay management
├── whitenoise_config.rs  # WhiteNoise protocol configuration
└── ui.rs                 # UI helper functions
```

## Technical Details

- **Built with Rust** for performance and safety
- **Nostr SDK** for cryptographic operations
- **Interactive CLI** using dialoguer for menus and input
- **Async/await** support with Tokio runtime
- **JSON serialization** for data persistence
- **Colorful terminal output** with console styling

## Demo Scripts

The repository includes comprehensive demo scripts to showcase CLI-Flutter compatibility:

### Running the Full Demo

```bash
# Run the complete demo (creates profiles, exchanges messages, creates group chat)
./demo_auto_conversation.sh
```

This demo will:
1. Create Alice and Bob accounts with profiles
2. Exchange MLS-based direct messages
3. Create a group chat with Alice, Bob, and a third member
4. Post messages in the group chat
5. Export Flutter-compatible private keys for import

### Individual Setup Scripts

```bash
# Set up Alice
./alice_setup.sh

# Set up Bob (in another terminal/directory)
./bob_setup.sh
```

### Verifying Messages on Relays

Install [nak](https://github.com/fiatjaf/nak) to verify events:

```bash
# Check Alice's messages
nak req -k 4 --author <alice_pubkey> --limit 10 wss://relay.damus.io

# Monitor live messages
nak req -k 4 --author <alice_pubkey> --author <bob_pubkey> --stream wss://relay.damus.io
```

## Current Status

This is a **fully functional implementation** of the WhiteNoise protocol:

- ✅ MLS-based secure messaging (not NIP-04)
- ✅ Direct messages using MLS groups (Flutter compatible)
- ✅ Group chat support with MLS encryption
- ✅ Full relay support (Nostr, Inbox, KeyPackage)
- ✅ Account persistence via WhiteNoise database
- ✅ CLI automation support
- ✅ 100% Flutter client compatibility

## License

This project implements the WhiteNoise CLI as specified in the technical plan.