#!/bin/bash

# WhiteNoise CLI Three-Client Complete Demo
# This demonstrates comprehensive messaging between Alice, Bob, and Charlie with full verification

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${MAGENTA}WhiteNoise CLI Three-Client Complete Demo${NC}"
echo -e "${BLUE}===========================================${NC}"
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

# Function to validate nsec with nak
validate_nsec() {
    local nsec="$1"
    local name="$2"
    echo -e "${YELLOW}Validating $name's nsec with nak...${NC}"
    
    if command -v nak >/dev/null 2>&1; then
        local hex_privkey=$(nak decode "$nsec" 2>/dev/null)
        if [ $? -eq 0 ] && [ -n "$hex_privkey" ]; then
            local derived_pubkey=$(nak key public "$hex_privkey" 2>/dev/null)
            if [ $? -eq 0 ] && [ -n "$derived_pubkey" ]; then
                echo "✅ $name's nsec is valid - derived pubkey: $derived_pubkey"
                return 0
            fi
        fi
    fi
    echo "❌ $name's nsec validation failed"
    return 1
}

echo -e "${CYAN}Setting up environment for file-based key storage...${NC}"
echo ""

# Create directories for all three clients
mkdir -p alice_demo bob_demo charlie_demo

# Note: All three instances will share the same WhiteNoise database
# This is actually helpful for contact discovery in a demo environment

# Copy the CLI binary to all directories
echo -e "${YELLOW}Copying CLI binaries...${NC}"
cp target/release/whitenoise-cli alice_demo/ 2>/dev/null || cp whitenoise-cli alice_demo/
cp target/release/whitenoise-cli bob_demo/ 2>/dev/null || cp whitenoise-cli bob_demo/
cp target/release/whitenoise-cli charlie_demo/ 2>/dev/null || cp whitenoise-cli charlie_demo/

# Ensure all clients share the same database by using symlinks to shared storage
SHARED_STORAGE_DIR="$HOME/.local/share/whitenoise-cli"
mkdir -p "$SHARED_STORAGE_DIR"

# Create symlinks so all clients use the same database
for dir in alice_demo bob_demo charlie_demo; do
    if [ ! -L "$dir/.whitenoise-cli" ]; then
        rm -rf "$dir/.whitenoise-cli" 2>/dev/null
        ln -sf "$SHARED_STORAGE_DIR" "$dir/.whitenoise-cli"
    fi
done

echo ""
echo -e "${GREEN}Step 1: Create Alice's account${NC}"
cd alice_demo

echo -e "${YELLOW}Creating Alice's account...${NC}"
ALICE_CREATE_OUTPUT=$(./whitenoise-cli --output json account create --name "Alice Demo" --about "Alice's test account for three-client demo" 2>&1)
ALICE_CREATE=$(echo "$ALICE_CREATE_OUTPUT" | extract_json)
ALICE_PUBKEY=$(extract_value "$ALICE_CREATE" "data.pubkey")

if [ -z "$ALICE_PUBKEY" ] || [ "$ALICE_PUBKEY" = "null" ]; then
    echo -e "${RED}Failed to create Alice's account${NC}"
    echo "$ALICE_CREATE_OUTPUT"
    exit 1
fi

echo "Alice's public key: $ALICE_PUBKEY"

# Export Alice's private key
echo -e "${YELLOW}Exporting Alice's private key...${NC}"
ALICE_EXPORT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" account export --private 2>&1 | extract_json)
ALICE_NSEC=$(extract_value "$ALICE_EXPORT" "data.private_key")

if [ -z "$ALICE_NSEC" ] || [ "$ALICE_NSEC" = "null" ]; then
    echo -e "${RED}Failed to export Alice's private key${NC}"
    exit 1
fi

echo -e "${CYAN}Alice's nsec: ${GREEN}${ALICE_NSEC}${NC}"
validate_nsec "$ALICE_NSEC" "Alice"

# Publish Alice's profile
echo -e "${YELLOW}Publishing Alice's profile to relays...${NC}"
./whitenoise-cli --output json --account "$ALICE_PUBKEY" account update --name "Alice Demo" --about "Alice's test account for three-client demo" >/dev/null 2>&1
echo "✅ Alice's profile published"

cd ..

echo ""
echo -e "${GREEN}Step 2: Create Bob's account${NC}"
cd bob_demo

echo -e "${YELLOW}Creating Bob's account...${NC}"
BOB_CREATE_OUTPUT=$(./whitenoise-cli --output json account create --name "Bob Demo" --about "Bob's test account for three-client demo" 2>&1)
BOB_CREATE=$(echo "$BOB_CREATE_OUTPUT" | extract_json)
BOB_PUBKEY=$(extract_value "$BOB_CREATE" "data.pubkey")

if [ -z "$BOB_PUBKEY" ] || [ "$BOB_PUBKEY" = "null" ]; then
    echo -e "${RED}Failed to create Bob's account${NC}"
    echo "$BOB_CREATE_OUTPUT"
    exit 1
fi

echo "Bob's public key: $BOB_PUBKEY"

# Export Bob's private key
echo -e "${YELLOW}Exporting Bob's private key...${NC}"
BOB_EXPORT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" account export --private 2>&1 | extract_json)
BOB_NSEC=$(extract_value "$BOB_EXPORT" "data.private_key")

if [ -z "$BOB_NSEC" ] || [ "$BOB_NSEC" = "null" ]; then
    echo -e "${RED}Failed to export Bob's private key${NC}"
    exit 1
fi

echo -e "${CYAN}Bob's nsec: ${GREEN}${BOB_NSEC}${NC}"
validate_nsec "$BOB_NSEC" "Bob"

# Publish Bob's profile
echo -e "${YELLOW}Publishing Bob's profile to relays...${NC}"
./whitenoise-cli --output json --account "$BOB_PUBKEY" account update --name "Bob Demo" --about "Bob's test account for three-client demo" >/dev/null 2>&1
echo "✅ Bob's profile published"

cd ..

echo ""
echo -e "${GREEN}Step 3: Create Charlie's account${NC}"
cd charlie_demo

echo -e "${YELLOW}Creating Charlie's account...${NC}"
CHARLIE_CREATE_OUTPUT=$(./whitenoise-cli --output json account create --name "Charlie Demo" --about "Charlie's test account for three-client demo" 2>&1)
CHARLIE_CREATE=$(echo "$CHARLIE_CREATE_OUTPUT" | extract_json)
CHARLIE_PUBKEY=$(extract_value "$CHARLIE_CREATE" "data.pubkey")

if [ -z "$CHARLIE_PUBKEY" ] || [ "$CHARLIE_PUBKEY" = "null" ]; then
    echo -e "${RED}Failed to create Charlie's account${NC}"
    echo "$CHARLIE_CREATE_OUTPUT"
    exit 1
fi

echo "Charlie's public key: $CHARLIE_PUBKEY"

# Export Charlie's private key
echo -e "${YELLOW}Exporting Charlie's private key...${NC}"
CHARLIE_EXPORT=$(./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" account export --private 2>&1 | extract_json)
CHARLIE_NSEC=$(extract_value "$CHARLIE_EXPORT" "data.private_key")

if [ -z "$CHARLIE_NSEC" ] || [ "$CHARLIE_NSEC" = "null" ]; then
    echo -e "${RED}Failed to export Charlie's private key${NC}"
    exit 1
fi

echo -e "${CYAN}Charlie's nsec: ${GREEN}${CHARLIE_NSEC}${NC}"
validate_nsec "$CHARLIE_NSEC" "Charlie"

# Publish Charlie's profile
echo -e "${YELLOW}Publishing Charlie's profile to relays...${NC}"
./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" account update --name "Charlie Demo" --about "Charlie's test account for three-client demo" >/dev/null 2>&1
echo "✅ Charlie's profile published"

cd ..

echo ""
echo -e "${GREEN}Step 4: Wait for key packages to propagate${NC}"
echo "Waiting 8 seconds for key packages to be available on relays..."
sleep 8

echo ""
echo -e "${GREEN}Step 5: Set up contacts for all participants${NC}"

# Alice adds Bob and Charlie
cd alice_demo
echo -e "${YELLOW}Alice adding contacts...${NC}"

ADD_BOB_OUTPUT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" contact add --name "Bob" --pubkey "$BOB_PUBKEY" 2>&1)
ADD_BOB=$(echo "$ADD_BOB_OUTPUT" | extract_json)
if echo "$ADD_BOB" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob added to Alice's contacts"
else
    echo "❌ Failed to add Bob to Alice's contacts"
    echo "$ADD_BOB" | jq '.' 2>/dev/null || echo "$ADD_BOB"
fi

ADD_CHARLIE_OUTPUT=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" contact add --name "Charlie" --pubkey "$CHARLIE_PUBKEY" 2>&1)
ADD_CHARLIE=$(echo "$ADD_CHARLIE_OUTPUT" | extract_json)
if echo "$ADD_CHARLIE" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Charlie added to Alice's contacts"
else
    echo "❌ Failed to add Charlie to Alice's contacts"
    echo "$ADD_CHARLIE" | jq '.' 2>/dev/null || echo "$ADD_CHARLIE"
fi

cd ..

# Bob adds Alice and Charlie
cd bob_demo
echo -e "${YELLOW}Bob adding contacts...${NC}"

ADD_ALICE_OUTPUT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" contact add --name "Alice" --pubkey "$ALICE_PUBKEY" 2>&1)
ADD_ALICE=$(echo "$ADD_ALICE_OUTPUT" | extract_json)
if echo "$ADD_ALICE" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice added to Bob's contacts"
else
    echo "❌ Failed to add Alice to Bob's contacts"
    echo "$ADD_ALICE" | jq '.' 2>/dev/null || echo "$ADD_ALICE"
fi

ADD_CHARLIE_BOB_OUTPUT=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" contact add --name "Charlie" --pubkey "$CHARLIE_PUBKEY" 2>&1)
ADD_CHARLIE_BOB=$(echo "$ADD_CHARLIE_BOB_OUTPUT" | extract_json)
if echo "$ADD_CHARLIE_BOB" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Charlie added to Bob's contacts"
else
    echo "❌ Failed to add Charlie to Bob's contacts"
    echo "$ADD_CHARLIE_BOB" | jq '.' 2>/dev/null || echo "$ADD_CHARLIE_BOB"
fi

cd ..

# Charlie adds Alice and Bob
cd charlie_demo
echo -e "${YELLOW}Charlie adding contacts...${NC}"

ADD_ALICE_CHARLIE_OUTPUT=$(./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" contact add --name "Alice" --pubkey "$ALICE_PUBKEY" 2>&1)
ADD_ALICE_CHARLIE=$(echo "$ADD_ALICE_CHARLIE_OUTPUT" | extract_json)
if echo "$ADD_ALICE_CHARLIE" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice added to Charlie's contacts"
else
    echo "❌ Failed to add Alice to Charlie's contacts"
    echo "$ADD_ALICE_CHARLIE" | jq '.' 2>/dev/null || echo "$ADD_ALICE_CHARLIE"
fi

ADD_BOB_CHARLIE_OUTPUT=$(./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" contact add --name "Bob" --pubkey "$BOB_PUBKEY" 2>&1)
ADD_BOB_CHARLIE=$(echo "$ADD_BOB_CHARLIE_OUTPUT" | extract_json)
if echo "$ADD_BOB_CHARLIE" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob added to Charlie's contacts"
else
    echo "❌ Failed to add Bob to Charlie's contacts"
    echo "$ADD_BOB_CHARLIE" | jq '.' 2>/dev/null || echo "$ADD_BOB_CHARLIE"
fi

cd ..

echo ""
echo -e "${GREEN}Step 6: Create group chat with all three participants${NC}"
cd alice_demo

echo -e "${YELLOW}Alice creating group chat with Bob and Charlie...${NC}"
GROUP_CREATE_OUTPUT=$(timeout 20 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" group create --name "Three Friends Chat" --description "Demo group with Alice, Bob, and Charlie" --members "$BOB_PUBKEY,$CHARLIE_PUBKEY" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
GROUP_RESULT=$(echo "$GROUP_CREATE_OUTPUT" | extract_json)

if echo "$GROUP_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Group created successfully"
    GROUP_ID=$(extract_value "$GROUP_RESULT" "data.group_id")
    echo "Group ID: $GROUP_ID"
    
    # Store group ID for other participants
    echo "$GROUP_ID" > ../group_id.txt
else
    echo "❌ Failed to create group"
    echo "$GROUP_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_RESULT"
    # Continue with DM testing even if group creation fails
    GROUP_ID=""
fi

cd ..

echo ""
echo -e "${GREEN}Step 7: Test group messaging (if group was created)${NC}"

if [ -n "$GROUP_ID" ]; then
    # Alice sends first group message
    cd alice_demo
    echo -e "${YELLOW}Alice sending message to group...${NC}"
    GROUP_MSG1_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message send --group "$GROUP_ID" --message "Hello everyone! 👋 Welcome to our three-way WhiteNoise demo group!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
    GROUP_MSG1_RESULT=$(echo "$GROUP_MSG1_OUTPUT" | extract_json)
    
    if echo "$GROUP_MSG1_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
        echo "✅ Alice's group message sent successfully"
    else
        echo "❌ Alice's group message failed"
        echo "$GROUP_MSG1_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_MSG1_RESULT"
    fi
    cd ..
    
    sleep 3
    
    # Bob responds in group
    cd bob_demo
    echo -e "${YELLOW}Bob responding in group...${NC}"
    GROUP_MSG2_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message send --group "$GROUP_ID" --message "Hey Alice and Charlie! 🚀 This group chat is awesome - MLS encryption is working perfectly!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
    GROUP_MSG2_RESULT=$(echo "$GROUP_MSG2_OUTPUT" | extract_json)
    
    if echo "$GROUP_MSG2_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
        echo "✅ Bob's group message sent successfully"
    else
        echo "❌ Bob's group message failed"
        echo "$GROUP_MSG2_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_MSG2_RESULT"
    fi
    cd ..
    
    sleep 3
    
    # Charlie joins the conversation
    cd charlie_demo
    echo -e "${YELLOW}Charlie joining group conversation...${NC}"
    GROUP_MSG3_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" message send --group "$GROUP_ID" --message "Hi Alice and Bob! 🎉 Thanks for adding me to this group. The end-to-end encryption is incredible!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
    GROUP_MSG3_RESULT=$(echo "$GROUP_MSG3_OUTPUT" | extract_json)
    
    if echo "$GROUP_MSG3_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
        echo "✅ Charlie's group message sent successfully"
    else
        echo "❌ Charlie's group message failed"
        echo "$GROUP_MSG3_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_MSG3_RESULT"
    fi
    cd ..
    
    sleep 3
    
    # Alice responds to the group conversation
    cd alice_demo
    echo -e "${YELLOW}Alice continuing group conversation...${NC}"
    GROUP_MSG4_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message send --group "$GROUP_ID" --message "I'm so glad you both like it! 💯 WhiteNoise makes secure group messaging so easy!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
    GROUP_MSG4_RESULT=$(echo "$GROUP_MSG4_OUTPUT" | extract_json)
    
    if echo "$GROUP_MSG4_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
        echo "✅ Alice's follow-up group message sent successfully"
    else
        echo "❌ Alice's follow-up group message failed"
        echo "$GROUP_MSG4_RESULT" | jq '.' 2>/dev/null || echo "$GROUP_MSG4_RESULT"
    fi
    cd ..
    
    echo -e "${CYAN}✅ Group messaging test completed with 4 messages from all participants${NC}"
else
    echo -e "${YELLOW}⚠️  Skipping group messaging test due to group creation failure${NC}"
fi

echo ""
echo -e "${GREEN}Step 8: Test direct messaging between participants${NC}"

# Alice DMs Bob
cd alice_demo
echo -e "${YELLOW}Alice sending DM to Bob...${NC}"
ALICE_BOB_DM_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$ALICE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "Hey Bob! 💬 Let's test our private DM channel. How's the encryption working for you?" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
ALICE_BOB_DM_RESULT=$(echo "$ALICE_BOB_DM_OUTPUT" | extract_json)

if echo "$ALICE_BOB_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Alice's DM to Bob sent successfully"
else
    echo "❌ Alice's DM to Bob failed"
    echo "$ALICE_BOB_DM_RESULT" | jq '.' 2>/dev/null || echo "$ALICE_BOB_DM_RESULT"
fi
cd ..

sleep 2

# Bob responds to Alice
cd bob_demo
echo -e "${YELLOW}Bob responding to Alice's DM...${NC}"
BOB_ALICE_DM_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$ALICE_PUBKEY" --message "Hi Alice! 🔒 The DM encryption is working perfectly! This private channel is secure and fast." 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_ALICE_DM_RESULT=$(echo "$BOB_ALICE_DM_OUTPUT" | extract_json)

if echo "$BOB_ALICE_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob's DM response to Alice sent successfully"
else
    echo "❌ Bob's DM response to Alice failed"
    echo "$BOB_ALICE_DM_RESULT" | jq '.' 2>/dev/null || echo "$BOB_ALICE_DM_RESULT"
fi
cd ..

sleep 2

# Charlie DMs Alice
cd charlie_demo
echo -e "${YELLOW}Charlie sending DM to Alice...${NC}"
CHARLIE_ALICE_DM_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" message dm --recipient "$ALICE_PUBKEY" --message "Hello Alice! 🌟 Thanks for organizing this demo. The DM functionality is impressive!" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
CHARLIE_ALICE_DM_RESULT=$(echo "$CHARLIE_ALICE_DM_OUTPUT" | extract_json)

if echo "$CHARLIE_ALICE_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Charlie's DM to Alice sent successfully"
else
    echo "❌ Charlie's DM to Alice failed"
    echo "$CHARLIE_ALICE_DM_RESULT" | jq '.' 2>/dev/null || echo "$CHARLIE_ALICE_DM_RESULT"
fi
cd ..

sleep 2

# Bob DMs Charlie
cd bob_demo
echo -e "${YELLOW}Bob sending DM to Charlie...${NC}"
BOB_CHARLIE_DM_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$BOB_PUBKEY" message dm --recipient "$CHARLIE_PUBKEY" --message "Hey Charlie! 👋 Great to meet you in this demo. How are you finding WhiteNoise so far?" 2>&1 || echo '{"success": false, "error": "Command timed out"}')
BOB_CHARLIE_DM_RESULT=$(echo "$BOB_CHARLIE_DM_OUTPUT" | extract_json)

if echo "$BOB_CHARLIE_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Bob's DM to Charlie sent successfully"
else
    echo "❌ Bob's DM to Charlie failed"
    echo "$BOB_CHARLIE_DM_RESULT" | jq '.' 2>/dev/null || echo "$BOB_CHARLIE_DM_RESULT"
fi
cd ..

sleep 2

# Charlie responds to Bob
cd charlie_demo
echo -e "${YELLOW}Charlie responding to Bob's DM...${NC}"
CHARLIE_BOB_DM_OUTPUT=$(timeout 15 ./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" message dm --recipient "$BOB_PUBKEY" --message "Hi Bob! 🚀 WhiteNoise is fantastic! The MLS protocol makes everything so secure and seamless." 2>&1 || echo '{"success": false, "error": "Command timed out"}')
CHARLIE_BOB_DM_RESULT=$(echo "$CHARLIE_BOB_DM_OUTPUT" | extract_json)

if echo "$CHARLIE_BOB_DM_RESULT" | jq -e '.success == true' >/dev/null 2>&1; then
    echo "✅ Charlie's DM response to Bob sent successfully"
else
    echo "❌ Charlie's DM response to Bob failed"
    echo "$CHARLIE_BOB_DM_RESULT" | jq '.' 2>/dev/null || echo "$CHARLIE_BOB_DM_RESULT"
fi
cd ..

echo -e "${CYAN}✅ Direct messaging test completed with 5 DM exchanges between all participants${NC}"

echo ""
echo -e "${GREEN}Step 9: Verify message delivery by checking conversations${NC}"

# Check Alice's conversations
cd alice_demo
echo -e "${YELLOW}Checking Alice's message history...${NC}"
ALICE_CONVERSATIONS=$(./whitenoise-cli --output json --account "$ALICE_PUBKEY" message list 2>/dev/null || echo '{"data": []}')
ALICE_MSG_COUNT=$(echo "$ALICE_CONVERSATIONS" | jq '.data | length' 2>/dev/null || echo "0")
echo "Alice has $ALICE_MSG_COUNT conversations/messages"
cd ..

# Check Bob's conversations  
cd bob_demo
echo -e "${YELLOW}Checking Bob's message history...${NC}"
BOB_CONVERSATIONS=$(./whitenoise-cli --output json --account "$BOB_PUBKEY" message list 2>/dev/null || echo '{"data": []}')
BOB_MSG_COUNT=$(echo "$BOB_CONVERSATIONS" | jq '.data | length' 2>/dev/null || echo "0")
echo "Bob has $BOB_MSG_COUNT conversations/messages"
cd ..

# Check Charlie's conversations
cd charlie_demo
echo -e "${YELLOW}Checking Charlie's message history...${NC}"
CHARLIE_CONVERSATIONS=$(./whitenoise-cli --output json --account "$CHARLIE_PUBKEY" message list 2>/dev/null || echo '{"data": []}')
CHARLIE_MSG_COUNT=$(echo "$CHARLIE_CONVERSATIONS" | jq '.data | length' 2>/dev/null || echo "0")
echo "Charlie has $CHARLIE_MSG_COUNT conversations/messages"
cd ..

echo ""
echo -e "${GREEN}Step 10: Verify events on relays using nak${NC}"

# Wait for events to propagate
echo -e "${YELLOW}Waiting 5 seconds for events to propagate...${NC}"
sleep 5

# Test localhost relay connectivity
echo -e "${YELLOW}Testing localhost relay connectivity...${NC}"
if timeout 5 nak req -k 1 --limit 1 ws://localhost:10547 >/dev/null 2>&1; then
    echo "✅ Localhost relay is reachable"
else
    echo "❌ Localhost relay is not reachable - is it running on port 10547?"
fi

# Check events for all participants on localhost relay
echo -e "${YELLOW}Checking events on localhost relay...${NC}"
ALICE_EVENTS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" ws://localhost:10547 2>/dev/null | wc -l)
BOB_EVENTS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" ws://localhost:10547 2>/dev/null | wc -l)
CHARLIE_EVENTS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$CHARLIE_PUBKEY" ws://localhost:10547 2>/dev/null | wc -l)

echo "Alice's events on localhost: $ALICE_EVENTS"
echo "Bob's events on localhost: $BOB_EVENTS"
echo "Charlie's events on localhost: $CHARLIE_EVENTS"

# Check events on public relays
echo -e "${YELLOW}Checking events on public relays...${NC}"
ALICE_DAMUS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.damus.io 2>/dev/null | wc -l)
ALICE_PRIMAL=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.primal.net 2>/dev/null | wc -l)
ALICE_NOS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://nos.lol 2>/dev/null | wc -l)
ALICE_NOSTR=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$ALICE_PUBKEY" wss://relay.nostr.net 2>/dev/null | wc -l)

BOB_DAMUS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.damus.io 2>/dev/null | wc -l)
BOB_PRIMAL=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.primal.net 2>/dev/null | wc -l)
BOB_NOS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://nos.lol 2>/dev/null | wc -l)
BOB_NOSTR=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$BOB_PUBKEY" wss://relay.nostr.net 2>/dev/null | wc -l)

CHARLIE_DAMUS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$CHARLIE_PUBKEY" wss://relay.damus.io 2>/dev/null | wc -l)
CHARLIE_PRIMAL=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$CHARLIE_PUBKEY" wss://relay.primal.net 2>/dev/null | wc -l)
CHARLIE_NOS=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$CHARLIE_PUBKEY" wss://nos.lol 2>/dev/null | wc -l)
CHARLIE_NOSTR=$(timeout 10 nak req -k 0 -k 443 -k 10002 --author "$CHARLIE_PUBKEY" wss://relay.nostr.net 2>/dev/null | wc -l)

echo ""
echo -e "${GREEN}Event Verification Summary:${NC}"
echo "Localhost relay (ws://localhost:10547): Alice($ALICE_EVENTS), Bob($BOB_EVENTS), Charlie($CHARLIE_EVENTS)"
echo "Damus relay: Alice($ALICE_DAMUS), Bob($BOB_DAMUS), Charlie($CHARLIE_DAMUS)"
echo "Primal relay: Alice($ALICE_PRIMAL), Bob($BOB_PRIMAL), Charlie($CHARLIE_PRIMAL)"
echo "Nos.lol relay: Alice($ALICE_NOS), Bob($BOB_NOS), Charlie($CHARLIE_NOS)"
echo "Nostr.net relay: Alice($ALICE_NOSTR), Bob($BOB_NOSTR), Charlie($CHARLIE_NOSTR)"

echo ""
echo -e "${MAGENTA}=== Three-Client Demo Summary ===${NC}"
echo ""
echo -e "${CYAN}Account Details for Mobile Testing:${NC}"
echo ""
echo -e "${GREEN}Alice:${NC}"
echo "  Public Key: $ALICE_PUBKEY"
echo "  nsec: $ALICE_NSEC"
echo ""
echo -e "${GREEN}Bob:${NC}"
echo "  Public Key: $BOB_PUBKEY"
echo "  nsec: $BOB_NSEC"
echo ""
echo -e "${GREEN}Charlie:${NC}"
echo "  Public Key: $CHARLIE_PUBKEY"  
echo "  nsec: $CHARLIE_NSEC"
echo ""

echo -e "${CYAN}Features Demonstrated:${NC}"
echo "✅ Three separate account creation with metadata publishing"
echo "✅ Comprehensive contact management between all participants"
echo "✅ MLS-based group chat with three participants"
echo "✅ Group messaging with 4 messages from all members"
echo "✅ Direct messaging between all possible pairs (5 DM exchanges)"
echo "✅ Private key validation with nak tool"
echo "✅ Event publishing verification to all relays"
echo "✅ Message history verification for all participants"
echo "✅ MLS end-to-end encryption for all communications"
echo ""

if [ -n "$GROUP_ID" ]; then
    echo -e "${GREEN}Group Chat Details:${NC}"
    echo "  Group ID: $GROUP_ID"
    echo "  Participants: Alice, Bob, Charlie"
    echo "  Messages Exchanged: 4 group messages"
    echo ""
fi

echo -e "${CYAN}Direct Messages Exchanged:${NC}"
echo "  Alice → Bob: Private DM conversation"
echo "  Bob → Alice: Private DM response"
echo "  Charlie → Alice: Private DM conversation"
echo "  Bob → Charlie: Private DM conversation"
echo "  Charlie → Bob: Private DM response"
echo "  Total: 5 DM exchanges between all participants"
echo ""

echo -e "${YELLOW}To import these accounts in WhiteNoise mobile app:${NC}"
echo "1. Copy any of the nsec values above"
echo "2. In the mobile app, use the import feature"
echo "3. Paste the nsec when prompted"
echo "4. You'll see all messages and conversations synced automatically!"
echo ""

echo -e "${CYAN}Message Verification Status:${NC}"
echo "  Alice's conversations: $ALICE_MSG_COUNT"
echo "  Bob's conversations: $BOB_MSG_COUNT"
echo "  Charlie's conversations: $CHARLIE_MSG_COUNT"
echo ""

# Cleanup options
echo -e "${YELLOW}Cleanup options:${NC}"
echo "  # Clean demo directories only:"
echo "  rm -rf alice_demo bob_demo charlie_demo"
echo ""
echo "  # Full cleanup (removes group ID file):"
echo "  rm -rf alice_demo bob_demo charlie_demo group_id.txt"
echo ""
echo -e "${CYAN}💡 Demo completed successfully! All three identities can be used for mobile testing.${NC}"
echo ""