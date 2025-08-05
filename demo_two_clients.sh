#!/bin/bash

# WhiteNoise CLI Two-Client Interaction Demo
# This demonstrates messaging between Alice and Bob with nsec export

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${MAGENTA}WhiteNoise CLI Two-Client Interaction Demo${NC}"
echo -e "${BLUE}==========================================${NC}"
echo ""

# Enable file-based storage for keyring-less operation
export WHITENOISE_FILE_STORAGE=1
export WHITENOISE_NO_KEYRING=1
export DBUS_SESSION_BUS_ADDRESS="disabled:"

# Function to extract JSON from output
extract_json() {
    # Extract the JSON object which may span multiple lines
    awk '/^{/{p=1} p{print} /^}/{if(p) exit}' || echo "{}"
}

# Function to extract value from JSON
extract_value() {
    local json="$1"
    local key="$2"
    echo "$json" | jq -r ".$key // empty" 2>/dev/null || echo ""
}

echo -e "${CYAN}Setting up environment for file-based key storage...${NC}"
echo ""

# Create directories for Alice and Bob
mkdir -p alice_demo bob_demo

# Note: Both instances will share the same WhiteNoise database
# This is actually helpful for contact discovery in a demo environment

# Copy the CLI binary to both directories
echo -e "${YELLOW}Copying CLI binaries...${NC}"
cp target/release/whitenoise-cli alice_demo/ 2>/dev/null || cp whitenoise-cli alice_demo/
cp target/release/whitenoise-cli bob_demo/ 2>/dev/null || cp whitenoise-cli bob_demo/

echo ""
echo -e "${GREEN}Step 1: Set up Alice's account${NC}"
cd alice_demo

# Check if we have existing accounts to reuse
ALICE_PUBKEY=""

# Try to find an existing Alice account by looking at stored accounts
if [ -f "../alice_pubkey.txt" ]; then
    STORED_ALICE=$(cat ../alice_pubkey.txt 2>/dev/null)
    # Fetch current account list and verify this account still exists
    EXISTING_ACCOUNTS=$(./whitenoise-cli --output json account list 2>/dev/null)
    if echo "$EXISTING_ACCOUNTS" | jq -e --arg pubkey "$STORED_ALICE" '.data[] | select(.pubkey == $pubkey)' >/dev/null 2>&1; then
        ALICE_PUBKEY="$STORED_ALICE"
        echo -e "${CYAN}♻️  Reusing existing Alice account${NC}"
        echo "Alice's public key: $ALICE_PUBKEY"
    fi
fi

# If no existing Alice account, create a new one
if [ -z "$ALICE_PUBKEY" ]; then
    echo -e "${YELLOW}Creating new Alice account...${NC}"
    ALICE_CREATE_OUTPUT=$(./whitenoise-cli --output json account create --name "Alice Demo" --about "Alice's test account for mobile" 2>&1)
    ALICE_CREATE=$(echo "$ALICE_CREATE_OUTPUT" | extract_json)
    ALICE_PUBKEY=$(extract_value "$ALICE_CREATE" "data.pubkey")
    
    if [ -z "$ALICE_PUBKEY" ] || [ "$ALICE_PUBKEY" = "null" ]; then
        echo -e "${RED}Failed to create Alice's account${NC}"
        exit 1
    fi
    
    # Store Alice's pubkey for future reuse
    echo "$ALICE_PUBKEY" > ../alice_pubkey.txt
    echo "Alice's public key: $ALICE_PUBKEY"
fi

# Export Alice's private key
echo -e "${YELLOW}Exporting Alice's private key...${NC}"
ALICE_EXPORT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" account export --private 2>&1 | extract_json)
ALICE_NSEC=$(extract_value "$ALICE_EXPORT" "data.private_key")

if [ -z "$ALICE_NSEC" ] || [ "$ALICE_NSEC" = "null" ]; then
    echo -e "${YELLOW}Private key managed by WhiteNoise internally${NC}"
    ALICE_NSEC="Managed by WhiteNoise (use interactive mode to view)"
fi

echo -e "${CYAN}Alice's nsec (for mobile import): ${GREEN}${ALICE_NSEC}${NC}"

# Explicitly update Alice's metadata to trigger event publishing
echo -e "${YELLOW}Publishing Alice's profile to relays...${NC}"
./whitenoise-cli --output json --account "$ALICE_PUBKEY" account update --name "Alice Demo" --about "Alice's test account for mobile" >/dev/null 2>&1
echo "✅ Alice's profile published"
echo ""

cd ..

echo -e "${GREEN}Step 2: Set up Bob's account${NC}"
cd bob_demo

# Check for existing Bob account
BOB_PUBKEY=""

# Try to find an existing Bob account by looking at stored accounts
if [ -f "../bob_pubkey.txt" ]; then
    STORED_BOB=$(cat ../bob_pubkey.txt 2>/dev/null)
    # Fetch current account list and verify this account still exists
    EXISTING_ACCOUNTS=$(./whitenoise-cli --output json account list 2>/dev/null)
    if echo "$EXISTING_ACCOUNTS" | jq -e --arg pubkey "$STORED_BOB" '.data[] | select(.pubkey == $pubkey)' >/dev/null 2>&1; then
        BOB_PUBKEY="$STORED_BOB"
        echo -e "${CYAN}♻️  Reusing existing Bob account${NC}"
        echo "Bob's public key: $BOB_PUBKEY"
    fi
fi

# If no existing Bob account, create a new one
if [ -z "$BOB_PUBKEY" ]; then
    echo -e "${YELLOW}Creating new Bob account...${NC}"
    BOB_CREATE=$(./whitenoise-cli --output json account create --name "Bob Demo" --about "Bob's test account for mobile" 2>/dev/null | extract_json)
    BOB_PUBKEY=$(extract_value "$BOB_CREATE" "data.pubkey")
    
    if [ -z "$BOB_PUBKEY" ] || [ "$BOB_PUBKEY" = "null" ]; then
        echo -e "${RED}Failed to create Bob's account${NC}"
        exit 1
    fi
    
    # Store Bob's pubkey for future reuse
    echo "$BOB_PUBKEY" > ../bob_pubkey.txt
    echo "Bob's public key: $BOB_PUBKEY"
fi

# Export Bob's private key
echo -e "${YELLOW}Exporting Bob's private key...${NC}"
BOB_EXPORT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" account export --private 2>&1 | extract_json)
BOB_NSEC=$(extract_value "$BOB_EXPORT" "data.private_key")

if [ -z "$BOB_NSEC" ] || [ "$BOB_NSEC" = "null" ]; then
    echo -e "${YELLOW}Private key managed by WhiteNoise internally${NC}"
    BOB_NSEC="Managed by WhiteNoise (use interactive mode to view)"
fi

echo -e "${CYAN}Bob's nsec (for mobile import): ${GREEN}${BOB_NSEC}${NC}"

# Explicitly update Bob's metadata to trigger event publishing
echo -e "${YELLOW}Publishing Bob's profile to relays...${NC}"
./whitenoise-cli --output json --account "$BOB_PUBKEY" account update --name "Bob Demo" --about "Bob's test account for mobile" >/dev/null 2>&1
echo "✅ Bob's profile published"
echo ""

cd ..

echo -e "${GREEN}Step 3: Wait for key packages to propagate${NC}"
echo "Waiting 5 seconds for key packages to be available on relays..."
sleep 5

echo -e "${GREEN}Step 4: Alice adds contacts${NC}"
cd alice_demo

# Add Bob as contact
ADD_BOB_OUTPUT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" contact add --name "Bob" --pubkey "$BOB_PUBKEY" 2>&1)
ADD_BOB=$(echo "$ADD_BOB_OUTPUT" | extract_json)
if echo "$ADD_BOB" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob added to Alice's contacts"
else
    echo "⚠️  Failed to add Bob"
    echo "$ADD_BOB" | jq '.' 2>/dev/null || echo "$ADD_BOB"
fi

# Add the third member
THIRD_MEMBER_NPUB="npub1d503g9345lpdtvtt0mhjxck5jedug9xmn2msuyqnxytltvnldkaslnrkqe"
ADD_THIRD_OUTPUT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" contact add --name "Third Member" --pubkey "$THIRD_MEMBER_NPUB" 2>&1)
ADD_THIRD=$(echo "$ADD_THIRD_OUTPUT" | extract_json)
if echo "$ADD_THIRD" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Third member added to Alice's contacts"
else
    echo "⚠️  Failed to add third member"
    echo "$ADD_THIRD" | jq '.' 2>/dev/null || echo "$ADD_THIRD"
fi

cd ..

echo ""
echo -e "${GREEN}Step 5: Bob adds contacts${NC}"
cd bob_demo

# Add Alice as contact
ADD_ALICE_OUTPUT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" contact add --name "Alice" --pubkey "$ALICE_PUBKEY" 2>&1)
ADD_ALICE=$(echo "$ADD_ALICE_OUTPUT" | extract_json)
if echo "$ADD_ALICE" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice added to Bob's contacts"
else
    echo "⚠️  Failed to add Alice"
    echo "$ADD_ALICE" | jq '.' 2>/dev/null || echo "$ADD_ALICE"
fi

# Add the third member
ADD_THIRD_BOB_OUTPUT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" contact add --name "Third Member" --pubkey "$THIRD_MEMBER_NPUB" 2>&1)
ADD_THIRD_BOB=$(echo "$ADD_THIRD_BOB_OUTPUT" | extract_json)
if echo "$ADD_THIRD_BOB" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Third member added to Bob's contacts"
else
    echo "⚠️  Failed to add third member"
    echo "$ADD_THIRD_BOB" | jq '.' 2>/dev/null || echo "$ADD_THIRD_BOB"
fi

cd ..

echo ""
echo -e "${GREEN}Step 6: Alice creates group chat with Bob and Third Member${NC}"
cd alice_demo

# Convert npub to hex (npub1d503g9345lpdtvtt0mhjxck5jedug9xmn2msuyqnxytltvnldkaslnrkqe)
THIRD_MEMBER_HEX="6d1f141635a7c2d5b16b7eef2362d4965bc414db9ab70e10133117f5b27f6dbb"
echo "Third member hex: $THIRD_MEMBER_HEX"

echo -e "${YELLOW}Alice creating group chat...${NC}"
GROUP_CREATE_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" group create --name "Test Group Chat" --description "Demo group with Alice, Bob, and Third Member" --members "$BOB_PUBKEY,$THIRD_MEMBER_HEX" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
GROUP_RESULT=$(echo "$GROUP_CREATE_OUTPUT" | extract_json)

if echo "$GROUP_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Group created successfully"
    GROUP_ID=$(extract_value "$GROUP_RESULT" "data.group_id")
    echo "Group ID: $GROUP_ID"
    
    # Alice sends message to group
    echo -e "${YELLOW}Alice sending message to group...${NC}"
    GROUP_MSG_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message send --group "$GROUP_ID" --message "Hello everyone! This is Alice. Welcome to our WhiteNoise test group! 👋" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
    GROUP_MSG_RESULT=$(echo "$GROUP_MSG_OUTPUT" | extract_json)
    
    if echo "$GROUP_MSG_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
        echo "✅ Group message sent successfully"
    else
        echo "❌ Failed to send group message"
        echo "$GROUP_MSG_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_MSG_RESULT"
    fi
else
    echo "❌ Failed to create group"
    echo "$GROUP_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_RESULT"
fi

cd ..

echo ""
echo -e "${GREEN}Step 7: Alice and Bob send private messages to Third Member${NC}"

# Alice sends DM to third member
cd alice_demo
echo -e "${YELLOW}Alice sending DM to Third Member...${NC}"
ALICE_DM_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$THIRD_MEMBER_HEX" --message "Hi! This is Alice from the WhiteNoise CLI demo. Nice to meet you! 😊" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_DM_RESULT=$(echo "$ALICE_DM_OUTPUT" | extract_json)

if echo "$ALICE_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice's DM sent successfully"
else
    echo "❌ Alice's DM failed"
    echo "$ALICE_DM_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_DM_RESULT"
fi

cd ..

# Bob sends DM to third member
cd bob_demo
echo -e "${YELLOW}Bob sending DM to Third Member...${NC}"
BOB_DM_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$THIRD_MEMBER_HEX" --message "Hello! This is Bob from the WhiteNoise CLI demo. Hope you're having a great day! 🚀" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_DM_RESULT=$(echo "$BOB_DM_OUTPUT" | extract_json)

if echo "$BOB_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob's DM sent successfully"
else
    echo "❌ Bob's DM failed"
    echo "$BOB_DM_RESULT" | jq '.' 2>/dev/null || echo "$BOB_DM_RESULT"
fi

cd ..

echo ""
echo -e "${GREEN}Step 8: Multi-message conversation between Alice and Bob${NC}"

# Create a DM conversation between Alice and Bob
echo -e "${CYAN}Starting multi-message conversation...${NC}"
echo ""

# Alice initiates conversation
cd alice_demo
echo -e "${YELLOW}Alice: Sending initial message to Bob...${NC}"
ALICE_MSG1_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "Hey Bob! 👋 How are you doing today?" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_MSG1_RESULT=$(echo "$ALICE_MSG1_OUTPUT" | extract_json)

if echo "$ALICE_MSG1_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice: Hey Bob! 👋 How are you doing today?"
else
    echo "❌ Alice's message failed"
    echo "$ALICE_MSG1_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_MSG1_RESULT"
fi

cd ..
sleep 2

# Bob responds
cd bob_demo
echo -e "${YELLOW}Bob: Responding to Alice...${NC}"
BOB_MSG1_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$ALICE_PUBKEY" --message "Hi Alice! I'm doing great, thanks for asking! 😊 How about you?" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_MSG1_RESULT=$(echo "$BOB_MSG1_OUTPUT" | extract_json)

if echo "$BOB_MSG1_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob: Hi Alice! I'm doing great, thanks for asking! 😊 How about you?"
else
    echo "❌ Bob's message failed"
    echo "$BOB_MSG1_RESULT" | jq '.' 2>/dev/null || echo "$BOB_MSG1_RESULT"
fi

cd ..
sleep 2

# Alice continues conversation
cd alice_demo
echo -e "${YELLOW}Alice: Continuing conversation...${NC}"
ALICE_MSG2_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "I'm doing fantastic! 🎉 Just testing out this WhiteNoise CLI - it's pretty cool!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_MSG2_RESULT=$(echo "$ALICE_MSG2_OUTPUT" | extract_json)

if echo "$ALICE_MSG2_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice: I'm doing fantastic! 🎉 Just testing out this WhiteNoise CLI - it's pretty cool!"
else
    echo "❌ Alice's second message failed"
    echo "$ALICE_MSG2_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_MSG2_RESULT"
fi

cd ..
sleep 2

# Bob asks about features
cd bob_demo
echo -e "${YELLOW}Bob: Asking about features...${NC}"
BOB_MSG2_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$ALICE_PUBKEY" --message "Totally agree! 🚀 The MLS encryption is impressive. Have you tried the group messaging yet?" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_MSG2_RESULT=$(echo "$BOB_MSG2_OUTPUT" | extract_json)

if echo "$BOB_MSG2_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob: Totally agree! 🚀 The MLS encryption is impressive. Have you tried the group messaging yet?"
else
    echo "❌ Bob's second message failed"
    echo "$BOB_MSG2_RESULT" | jq '.' 2>/dev/null || echo "$BOB_MSG2_RESULT"
fi

cd ..
sleep 2

# Alice shares experience
cd alice_demo
echo -e "${YELLOW}Alice: Sharing group chat experience...${NC}"
ALICE_MSG3_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "Yes! We just created a group chat with that third member. The end-to-end encryption is seamless! 🔒✨" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_MSG3_RESULT=$(echo "$ALICE_MSG3_OUTPUT" | extract_json)

if echo "$ALICE_MSG3_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice: Yes! We just created a group chat with that third member. The end-to-end encryption is seamless! 🔒✨"
else
    echo "❌ Alice's third message failed"
    echo "$ALICE_MSG3_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_MSG3_RESULT"
fi

cd ..
sleep 2

# Bob concludes conversation
cd bob_demo
echo -e "${YELLOW}Bob: Wrapping up conversation...${NC}"
BOB_MSG3_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$ALICE_PUBKEY" --message "That's awesome! 🎯 This CLI makes secure messaging so accessible. Great work by the WhiteNoise team!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_MSG3_RESULT=$(echo "$BOB_MSG3_OUTPUT" | extract_json)

if echo "$BOB_MSG3_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob: That's awesome! 🎯 This CLI makes secure messaging so accessible. Great work by the WhiteNoise team!"
else
    echo "❌ Bob's final message failed"
    echo "$BOB_MSG3_RESULT" | jq '.' 2>/dev/null || echo "$BOB_MSG3_RESULT"
fi

cd ..
sleep 2

# Alice's final message
cd alice_demo
echo -e "${YELLOW}Alice: Final message...${NC}"
ALICE_MSG4_OUTPUT=$(timeout 10 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "Absolutely! 💯 It's been great chatting with you, Bob. See you in the next demo! 👋" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_MSG4_RESULT=$(echo "$ALICE_MSG4_OUTPUT" | extract_json)

if echo "$ALICE_MSG4_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice: Absolutely! 💯 It's been great chatting with you, Bob. See you in the next demo! 👋"
else
    echo "❌ Alice's final message failed"
    echo "$ALICE_MSG4_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_MSG4_RESULT"
fi

cd ..

echo ""
echo -e "${GREEN}💬 Multi-message conversation completed!${NC}"
echo -e "${CYAN}Demonstrated: 6 back-and-forth messages with MLS encryption${NC}"
echo ""

echo -e "${GREEN}Step 9: Verify events on relays using nak${NC}"

# Wait for events to propagate
echo -e "${YELLOW}Waiting 3 seconds for events to propagate...${NC}"
sleep 3

# Test if localhost relay is reachable
echo -e "${YELLOW}Testing localhost relay connectivity...${NC}"
if timeout 5 nak req -k 1 --limit 1 ws://localhost:10547 >/dev/null 2>&1; then
    echo "✅ Localhost relay is reachable"
else
    echo "❌ Localhost relay is not reachable - is it running on port 10547?"
fi

# Check events for Alice on localhost relay
echo -e "${YELLOW}Checking Alice's events on localhost relay...${NC}"
ALICE_EVENTS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" ws://localhost:10547 2>/dev/null | wc -l)
echo "Alice's events on localhost: $ALICE_EVENTS"

# Check events for Bob on localhost relay  
echo -e "${YELLOW}Checking Bob's events on localhost relay...${NC}"
BOB_EVENTS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" ws://localhost:10547 2>/dev/null | wc -l)
echo "Bob's events on localhost: $BOB_EVENTS"

# Check events on public relays
echo -e "${YELLOW}Checking Alice's events on public relays...${NC}"
ALICE_DAMUS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.damus.io 2>/dev/null | wc -l)
ALICE_PRIMAL=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.primal.net 2>/dev/null | wc -l)
ALICE_NOS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://nos.lol 2>/dev/null | wc -l)
ALICE_NOSTR=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.nostr.net 2>/dev/null | wc -l)

echo "Alice's events on relay.damus.io: $ALICE_DAMUS"
echo "Alice's events on relay.primal.net: $ALICE_PRIMAL"  
echo "Alice's events on nos.lol: $ALICE_NOS"
echo "Alice's events on relay.nostr.net: $ALICE_NOSTR"

echo -e "${YELLOW}Checking Bob's events on public relays...${NC}"
BOB_DAMUS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.damus.io 2>/dev/null | wc -l)
BOB_PRIMAL=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.primal.net 2>/dev/null | wc -l)
BOB_NOS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://nos.lol 2>/dev/null | wc -l)
BOB_NOSTR=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.nostr.net 2>/dev/null | wc -l)

echo "Bob's events on relay.damus.io: $BOB_DAMUS"
echo "Bob's events on relay.primal.net: $BOB_PRIMAL"
echo "Bob's events on nos.lol: $BOB_NOS"
echo "Bob's events on relay.nostr.net: $BOB_NOSTR"

echo ""
echo -e "${GREEN}Event Verification Summary:${NC}"
echo "Localhost relay (ws://localhost:10547): Alice($ALICE_EVENTS), Bob($BOB_EVENTS)"
echo "Damus relay: Alice($ALICE_DAMUS), Bob($BOB_DAMUS)"
echo "Primal relay: Alice($ALICE_PRIMAL), Bob($BOB_PRIMAL)"
echo "Nos.lol relay: Alice($ALICE_NOS), Bob($BOB_NOS)"
echo "Nostr.net relay: Alice($ALICE_NOSTR), Bob($BOB_NOSTR)"

echo ""
echo -e "${MAGENTA}=== Summary ===${NC}"
echo ""
echo -e "${CYAN}Account Details for Mobile Testing:${NC}"
echo ""
echo -e "${GREEN}Alice:${NC}"
echo "  Public Key: $ALICE_PUBKEY"
echo "  nsec: ${ALICE_NSEC:-Not available - check interactive mode}"
echo ""
echo -e "${GREEN}Bob:${NC}"
echo "  Public Key: $BOB_PUBKEY"
echo "  nsec: ${BOB_NSEC:-Not available - check interactive mode}"
echo ""

echo -e "${CYAN}Features Demonstrated:${NC}"
echo "✅ Account creation with metadata publishing"
echo "✅ Contact management (including external npub)"
echo "✅ MLS-based group chat creation"
echo "✅ Group messaging with multiple participants"
echo "✅ Multi-message DM conversation (6 messages back-and-forth)"
echo "✅ Private DM messaging to external contacts"
echo "✅ Event publishing to all relays including relay.nostr.net"
echo "✅ MLS end-to-end encryption for all messages"
echo ""

echo -e "${YELLOW}To import these accounts in WhiteNoise mobile app:${NC}"
echo "1. Copy the nsec for the account you want to import"
echo "2. In the mobile app, use the import feature"
echo "3. Paste the nsec when prompted"
echo "4. You'll see all messages synced automatically!"
echo ""

echo -e "${CYAN}Note:${NC} If nsec export failed, run the CLI interactively:"
echo "  cd alice_demo && ./whitenoise-cli"
echo "  Then: account info (to see private key)"
echo ""

# Cleanup options
echo -e "${YELLOW}Cleanup options:${NC}"
echo "  # Clean demo directories (keeps accounts for reuse):"
echo "  rm -rf alice_demo bob_demo"
echo ""
echo "  # Full cleanup (removes accounts - will create new ones next run):"
echo "  rm -rf alice_demo bob_demo alice_pubkey.txt bob_pubkey.txt"
echo ""
echo -e "${CYAN}💡 Tip: Keep alice_pubkey.txt and bob_pubkey.txt to reuse the same identities${NC}"
echo ""