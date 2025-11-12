#!/bin/bash

# Erbium Blockchain - Production Deployment Script
# This script deploys Erbium nodes to cloud VMs

set -e

# Configuration
NODE_COUNT=${NODE_COUNT:-2}
REGION=${REGION:-us-east-1}
INSTANCE_TYPE=${INSTANCE_TYPE:-t3.medium}
SECURITY_GROUP=${SECURITY_GROUP:-erbium-nodes}

echo "🚀 Deploying Erbium Blockchain to Cloud"
echo "========================================"
echo "Nodes: $NODE_COUNT"
echo "Region: $REGION"
echo "Instance Type: $INSTANCE_TYPE"
echo "Config: config/mainnet.toml"
echo ""

# Check if AWS CLI is configured
if ! aws sts get-caller-identity &>/dev/null; then
    echo "❌ AWS CLI not configured. Please run 'aws configure'"
    exit 1
fi

# Build the project in release mode
echo "📦 Building Erbium in release mode..."
cargo build --release

# Create security group if it doesn't exist
echo "🔒 Creating/updating security group..."
if ! aws ec2 describe-security-groups --group-names "$SECURITY_GROUP" --region "$REGION" &>/dev/null; then
    aws ec2 create-security-group \
        --group-name "$SECURITY_GROUP" \
        --description "Erbium Blockchain Nodes" \
        --region "$REGION" \
        --tag-specifications "ResourceType=security-group,Tags=[{Key=Name,Value=ErbiumNodes}]"
fi

# Configure security group rules
SG_ID=$(aws ec2 describe-security-groups --group-names "$SECURITY_GROUP" --region "$REGION" --query 'SecurityGroups[0].GroupId' --output text)

# Allow P2P communication (3030)
aws ec2 authorize-security-group-ingress \
    --group-id "$SG_ID" \
    --protocol tcp \
    --port 3030 \
    --cidr 0.0.0.0/0 \
    --region "$REGION" || true

# Allow RPC (8545)
aws ec2 authorize-security-group-ingress \
    --group-id "$SG_ID" \
    --protocol tcp \
    --port 8545 \
    --cidr 0.0.0.0/0 \
    --region "$REGION" || true

# Allow REST API (8080)
aws ec2 authorize-security-group-ingress \
    --group-id "$SG_ID" \
    --protocol tcp \
    --port 8080 \
    --cidr 0.0.0.0/0 \
    --region "$REGION" || true

# Allow WebSocket (8546)
aws ec2 authorize-security-group-ingress \
    --group-id "$SG_ID" \
    --protocol tcp \
    --port 8546 \
    --cidr 0.0.0.0/0 \
    --region "$REGION" || true

# Allow SSH (22)
aws ec2 authorize-security-group-ingress \
    --group-id "$SG_ID" \
    --protocol tcp \
    --port 22 \
    --cidr 0.0.0.0/0 \
    --region "$REGION" || true

echo "✅ Security group configured"

# Launch EC2 instances
echo "🚀 Launching $NODE_COUNT EC2 instances..."
INSTANCE_IDS=$(aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1d0 \
    --count "$NODE_COUNT" \
    --instance-type "$INSTANCE_TYPE" \
    --security-group-ids "$SG_ID" \
    --key-name your-key-pair \
    --region "$REGION" \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=ErbiumNode},{Key=Project,Value=ErbiumBlockchain}]" \
    --user-data "#!/bin/bash
yum update -y
yum install -y git curl wget
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
export PATH=\$PATH:~/.cargo/bin
cd /home/ec2-user
git clone https://github.com/your-org/Erbium-Node.git
cd Erbium-Node
./scripts/install_dependencies.sh
cargo build --release
mkdir -p data/mainnet
chown -R ec2-user:ec2-user /home/ec2-user/Erbium-Node" \
    --query 'Instances[*].InstanceId' \
    --output text)

echo "✅ Instances launched: $INSTANCE_IDS"

# Wait for instances to be running
echo "⏳ Waiting for instances to be ready..."
aws ec2 wait instance-running --instance-ids $INSTANCE_IDS --region "$REGION"

# Get public IPs
PUBLIC_IPS=$(aws ec2 describe-instances \
    --instance-ids $INSTANCE_IDS \
    --region "$REGION" \
    --query 'Reservations[*].Instances[*].PublicIpAddress' \
    --output text)

echo "🌐 Instance Public IPs:"
echo "$PUBLIC_IPS"

# Create bootstrap peer list
BOOTSTRAP_PEERS=""
for ip in $PUBLIC_IPS; do
    BOOTSTRAP_PEERS="$BOOTSTRAP_PEERS /ip4/$ip/tcp/3030/p2p/QmYourPeerIdHere,"
done
BOOTSTRAP_PEERS=${BOOTSTRAP_PEERS%,}

echo ""
echo "📋 Bootstrap Peers Configuration:"
echo "$BOOTSTRAP_PEERS"

# Generate node configuration for each instance
NODE_NUM=1
for ip in $PUBLIC_IPS; do
    echo ""
    echo "🔧 Node $NODE_NUM Configuration (IP: $ip):"
    cat << EOF
# Node $NODE_NUM Configuration
[node]
p2p_listen_addr = "/ip4/0.0.0.0/tcp/3030"
rpc_listen_addr = "0.0.0.0:8545"
rest_listen_addr = "0.0.0.0:8080"
ws_listen_addr = "0.0.0.0:8546"

[network]
bootstrap_peers = ["$BOOTSTRAP_PEERS"]
max_peers = 50
min_peers = 3

[discovery]
enable_mdns = false
enable_kademlia = true
EOF

    NODE_NUM=$((NODE_NUM + 1))
done

echo ""
echo "🎉 Deployment Complete!"
echo "======================"
echo "• $NODE_COUNT nodes launched"
echo "• Security configured"
echo "• Bootstrap peers ready"
echo ""
echo "📝 Next Steps:"
echo "1. SSH into each instance"
echo "2. Update bootstrap peer IDs with actual peer IDs"
echo "3. Start nodes with: ./target/release/erbium-node --config config/mainnet.toml"
echo "4. Monitor logs and connectivity"
