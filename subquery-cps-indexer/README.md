# Robonomics CPS SubQuery Indexer

A comprehensive [SubQuery](https://subquery.network/) indexer for the Robonomics CPS (Cyber-Physical Systems) Pallet. This indexer tracks all CPS pallet events, builds relational database structures, maintains audit trails, and exposes a GraphQL API for querying hierarchical node structures.

## 🌟 Features

- **Complete Event Coverage**: Indexes all CPS pallet events:
  - `NodeCreated` - Track node creation in the hierarchy
  - `MetaSet` - Monitor metadata updates
  - `PayloadSet` - Track payload changes
  - `NodeMoved` - Record tree reorganizations
  - `NodeDeleted` - Monitor node deletions

- **Rich Data Model**:
  - Hierarchical node structure with parent-child relationships
  - Complete audit trail with historical records
  - Owner-based indexing for fast lookups
  - Global and daily statistics
  
- **Powerful GraphQL API**: Query nodes by owner, traverse tree structures, access full history
  
- **Battle-tested Technology**: Built on SubQuery - proven in the Polkadot ecosystem

## 📋 Prerequisites

- [Node.js](https://nodejs.org/) v18 or higher
- [Docker](https://www.docker.com/) and Docker Compose (for local development)
- [Yarn](https://yarnpkg.com/) or npm

## 🚀 Quick Start

### 1. Install Dependencies

```bash
cd subquery-cps-indexer
npm install
# or
yarn install
```

### 2. Generate TypeScript Types

```bash
npm run codegen
# or
yarn codegen
```

This generates TypeScript types from the GraphQL schema.

### 3. Build the Project

```bash
npm run build
# or
yarn build
```

### 4. Run Locally with Docker

Start the full stack (PostgreSQL, SubQuery Node, GraphQL Engine):

```bash
npm run start:docker
# or
yarn start:docker
```

This will:
- Start PostgreSQL database on port 5432
- Start SubQuery indexer node (begins syncing from block 1)
- Start GraphQL API on http://localhost:3000

### 5. Access the GraphQL Playground

Open your browser to http://localhost:3000 to access the GraphQL playground and start querying!

## 📊 GraphQL Schema

### Core Entities

#### Node
Represents a node in the CPS hierarchical tree:
```graphql
type Node {
  id: ID!                      # Node ID
  parentId: String             # Parent node ID (null for roots)
  parent: Node                 # Parent node reference
  children: [Node!]            # Child nodes
  owner: String!               # Owner account address
  metaType: String             # Plain or Encrypted
  metaData: String             # Metadata content (hex)
  metaAlgorithm: String        # Encryption algorithm (if encrypted)
  payloadType: String          # Plain or Encrypted
  payloadData: String          # Payload content (hex)
  payloadAlgorithm: String     # Encryption algorithm (if encrypted)
  createdAtBlock: BigInt!      # Creation block number
  createdAt: Date!             # Creation timestamp
  updatedAtBlock: BigInt!      # Last update block number
  updatedAt: Date!             # Last update timestamp
  isDeleted: Boolean!          # Deletion flag
  deletedAtBlock: BigInt       # Deletion block (if deleted)
  deletedAt: Date              # Deletion timestamp (if deleted)
  history: [NodeHistory!]      # Audit trail
}
```

#### NodeHistory
Complete audit trail for all node changes:
```graphql
type NodeHistory {
  id: ID!                      # Unique ID
  node: Node!                  # Reference to node
  action: String!              # CREATED, META_SET, PAYLOAD_SET, MOVED, DELETED
  blockNumber: BigInt!         # Block number
  timestamp: Date!             # Timestamp
  txHash: String!              # Transaction hash
  actor: String!               # Account that performed action
  oldValue: String             # Old value (JSON)
  newValue: String             # New value (JSON)
  oldParentId: String          # Old parent (for MOVED)
  newParentId: String          # New parent (for MOVED)
}
```

#### OwnerIndex
Fast owner-based lookups:
```graphql
type OwnerIndex {
  id: ID!                      # Composite: owner-nodeId
  owner: String!               # Owner account
  node: Node!                  # Node reference
  createdAtBlock: BigInt!      # Block number
}
```

#### Statistics
Global statistics:
```graphql
type Statistics {
  id: ID!                      # Always "global"
  totalNodesCreated: BigInt!   # Total nodes ever created
  activeNodes: BigInt!         # Current active nodes
  deletedNodes: BigInt!        # Total deleted nodes
  rootNodes: BigInt!           # Current root nodes
  metaUpdates: BigInt!         # Total metadata updates
  payloadUpdates: BigInt!      # Total payload updates
  nodeMoves: BigInt!           # Total node moves
  lastUpdatedBlock: BigInt!    # Last update block
  lastUpdatedAt: Date!         # Last update timestamp
}
```

## 🔍 Example Queries

### Get a specific node with all details

```graphql
query {
  node(id: "0") {
    id
    owner
    parentId
    metaType
    metaData
    payloadType
    payloadData
    createdAt
    updatedAt
    isDeleted
    children {
      id
      owner
    }
  }
}
```

### Get all nodes owned by an account

```graphql
query {
  ownerIndices(filter: { owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" } }) {
    nodes {
      node {
        id
        parentId
        owner
        createdAt
        isDeleted
      }
    }
  }
}
```

### Get a node's complete history (audit trail)

```graphql
query {
  nodeHistories(
    filter: { nodeId: { equalTo: "0" } }
    orderBy: BLOCK_NUMBER_ASC
  ) {
    nodes {
      action
      blockNumber
      timestamp
      actor
      oldValue
      newValue
      txHash
    }
  }
}
```

### Get all root nodes (nodes without parents)

```graphql
query {
  nodes(filter: { parentId: { isNull: true }, isDeleted: { equalTo: false } }) {
    nodes {
      id
      owner
      createdAt
      children {
        id
        owner
      }
    }
  }
}
```

### Get tree structure by materializing from a root

```graphql
query {
  node(id: "0") {
    id
    owner
    createdAt
    children {
      id
      owner
      children {
        id
        owner
        children {
          id
          owner
        }
      }
    }
  }
}
```

### Get global statistics

```graphql
query {
  statistics(id: "global") {
    totalNodesCreated
    activeNodes
    deletedNodes
    rootNodes
    metaUpdates
    payloadUpdates
    nodeMoves
    lastUpdatedAt
  }
}
```

### Get daily statistics for a specific date

```graphql
query {
  dailyStats(id: "2025-01-15") {
    nodesCreated
    nodesDeleted
    metaUpdates
    payloadUpdates
    nodeMoves
    date
  }
}
```

### Search nodes with filters

```graphql
query {
  nodes(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
      isDeleted: { equalTo: false }
      createdAt: { greaterThan: "2025-01-01" }
    }
    orderBy: CREATED_AT_DESC
    first: 10
  ) {
    totalCount
    nodes {
      id
      parentId
      metaType
      createdAt
    }
  }
}
```

### Get recent activity across all nodes

```graphql
query {
  nodeHistories(
    orderBy: BLOCK_NUMBER_DESC
    first: 20
  ) {
    nodes {
      action
      blockNumber
      timestamp
      actor
      node {
        id
        owner
      }
    }
  }
}
```

## 🏗️ Architecture

```
Robonomics Node → SubQuery Indexer → PostgreSQL → GraphQL API → UI/Apps
```

### Components

1. **Robonomics Node**: Source of blockchain events
2. **SubQuery Indexer**: Processes events and populates database
3. **PostgreSQL**: Relational database with indexed tables
4. **GraphQL API**: Query interface for applications

### Database Schema

```sql
-- Core tables
nodes (
  id, parent_id, owner, 
  meta_type, meta_data, meta_algorithm,
  payload_type, payload_data, payload_algorithm,
  created_at_block, created_at,
  updated_at_block, updated_at,
  is_deleted, deleted_at_block, deleted_at
)

node_history (
  id, node_id, action,
  block_number, timestamp, tx_hash, actor,
  old_value, new_value,
  old_parent_id, new_parent_id
)

owner_index (
  id, owner, node_id, created_at_block
)

statistics (
  id, total_nodes_created, active_nodes, deleted_nodes,
  root_nodes, meta_updates, payload_updates, node_moves,
  last_updated_block, last_updated_at
)

daily_stats (
  id, nodes_created, nodes_deleted,
  meta_updates, payload_updates, node_moves, date
)
```

## 🔧 Configuration

### Network Configuration

Edit `project.yaml` to change the network:

```yaml
network:
  chainId: '0x631ccc82a078481584041656af292834e3a7e4be7fc3e1e4e87eefdaf19ee23a'
  endpoint: wss://kusama.rpc.robonomics.network
  # For local development:
  # endpoint: ws://127.0.0.1:9944
```

### Start Block

Adjust the `startBlock` in `project.yaml` to begin indexing from a specific block:

```yaml
dataSources:
  - kind: substrate/Runtime
    startBlock: 1  # Start from block 1
```

### Environment Variables

Create a `.env` file for custom configuration:

```env
DB_HOST=postgres
DB_PORT=5432
DB_USER=postgres
DB_PASS=postgres
DB_DATABASE=postgres
```

## 🧪 Testing

### Unit Tests

```bash
npm test
# or
yarn test
```

### Test Coverage

```bash
npm run test:coverage
# or
yarn test:coverage
```

### Integration Testing

The project includes integration test examples in `src/tests/`. These tests verify:
- Event handler logic
- Database entity creation
- Statistical calculations
- Historical record tracking

## 📦 Deployment

### Deploy to SubQuery Managed Service

1. Create a project on [SubQuery Managed Service](https://managedservice.subquery.network/)
2. Build and publish:

```bash
# Build the project
npm run build

# Publish to IPFS
subql publish

# Deploy to managed service
subql deployment:deploy
```

### Self-Hosted Deployment

Use the Docker Compose setup or deploy to Kubernetes:

```bash
# Production build
npm run build

# Deploy with your infrastructure
```

## 🛠️ Development

### Project Structure

```
subquery-cps-indexer/
├── src/
│   ├── index.ts              # Main entry point
│   ├── mappings/
│   │   ├── cpsHandlers.ts    # Event handlers
│   │   └── utils.ts          # Helper utilities
│   └── types/                # Generated types (from codegen)
├── docker/
│   └── pg-Dockerfile         # PostgreSQL Dockerfile
├── schema.graphql            # GraphQL schema definition
├── project.yaml              # SubQuery project manifest
├── package.json              # Dependencies and scripts
├── tsconfig.json             # TypeScript configuration
├── docker-compose.yml        # Local development stack
└── README.md                 # This file
```

### Adding New Event Handlers

1. Add the event to `project.yaml`:
```yaml
- handler: handleMyEvent
  kind: substrate/EventHandler
  filter:
    module: cps
    method: MyEvent
```

2. Implement the handler in `src/mappings/cpsHandlers.ts`:
```typescript
export async function handleMyEvent(event: SubstrateEvent): Promise<void> {
  // Your implementation
}
```

3. Export it from `src/index.ts`

4. Regenerate types and rebuild:
```bash
npm run codegen
npm run build
```

## 📚 Resources

- [SubQuery Documentation](https://academy.subquery.network/)
- [Robonomics Documentation](https://wiki.robonomics.network/)
- [CPS Pallet README](../frame/cps/README.md)
- [GraphQL Documentation](https://graphql.org/learn/)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

Apache License 2.0 - See [LICENSE](../LICENSE) for details.

## 💬 Support

- [Robonomics Discord](https://discord.gg/robonomics)
- [Robonomics Forum](https://forum.robonomics.network/)
- [GitHub Issues](https://github.com/airalab/robonomics/issues)

---

**Built with ❤️ by the Robonomics Network community**
