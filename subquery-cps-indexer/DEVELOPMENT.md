# Development Guide

This guide provides detailed information for developers working on the Robonomics CPS SubQuery indexer.

## Table of Contents

- [Setup Development Environment](#setup-development-environment)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Testing Strategy](#testing-strategy)
- [Debugging](#debugging)
- [Performance Optimization](#performance-optimization)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)

## Setup Development Environment

### Prerequisites

1. **Node.js**: Version 18 or higher
   ```bash
   node --version
   ```

2. **Package Manager**: Yarn or npm
   ```bash
   yarn --version
   # or
   npm --version
   ```

3. **Docker**: For running local infrastructure
   ```bash
   docker --version
   docker-compose --version
   ```

4. **SubQuery CLI**: Global installation
   ```bash
   npm install -g @subql/cli
   # or
   yarn global add @subql/cli
   ```

### Initial Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/airalab/robonomics.git
   cd robonomics/subquery-cps-indexer
   ```

2. **Install dependencies**:
   ```bash
   yarn install
   # or
   npm install
   ```

3. **Generate types**:
   ```bash
   yarn codegen
   # or
   npm run codegen
   ```

4. **Build the project**:
   ```bash
   yarn build
   # or
   npm run build
   ```

## Project Structure

```
subquery-cps-indexer/
├── src/
│   ├── index.ts                 # Main entry point, exports handlers
│   ├── mappings/
│   │   ├── cpsHandlers.ts      # Event handler implementations
│   │   └── utils.ts            # Helper utilities
│   ├── types/                   # Generated types (from codegen)
│   │   ├── models/             # Database entity types
│   │   └── abi-interfaces/     # ABI type definitions
│   └── __tests__/              # Test files
│       ├── utils.test.ts       # Unit tests for utilities
│       └── integration.test.ts # Integration test examples
├── docker/
│   └── pg-Dockerfile           # PostgreSQL Docker configuration
├── schema.graphql              # GraphQL schema definition
├── project.yaml                # SubQuery project manifest
├── package.json                # Dependencies and scripts
├── tsconfig.json               # TypeScript configuration
├── jest.config.js              # Jest test configuration
├── docker-compose.yml          # Local development stack
├── README.md                   # Main documentation
├── API_EXAMPLES.md             # GraphQL query examples
└── DEVELOPMENT.md              # This file
```

### Key Files Explained

#### `project.yaml`
- **Purpose**: SubQuery project configuration
- **Key Sections**:
  - `network`: Chain connection details
  - `dataSources`: Event handlers configuration
  - `schema`: GraphQL schema location

#### `schema.graphql`
- **Purpose**: Database schema definition
- **Entities**: Node, NodeHistory, OwnerIndex, Statistics, DailyStats
- **Relationships**: Defined using `@entity` and `@derivedFrom`

#### `src/mappings/cpsHandlers.ts`
- **Purpose**: Event handler implementations
- **Handlers**:
  - `handleNodeCreated`: Process node creation events
  - `handleMetaSet`: Process metadata updates
  - `handlePayloadSet`: Process payload updates
  - `handleNodeMoved`: Process node movements
  - `handleNodeDeleted`: Process node deletions

#### `src/mappings/utils.ts`
- **Purpose**: Shared utility functions
- **Functions**:
  - `getOrCreateStatistics`: Get/create global stats
  - `getOrCreateDailyStats`: Get/create daily stats
  - `getTimestamp`: Extract timestamp from block
  - `parseNodeData`: Parse Plain/Encrypted data
  - `createCompositeId`: Generate composite IDs
  - `ensureString`: Convert values to strings

## Development Workflow

### 1. Making Schema Changes

When you need to modify the database schema:

1. Edit `schema.graphql`
2. Regenerate types:
   ```bash
   yarn codegen
   ```
3. Update handlers in `src/mappings/` to use new schema
4. Test changes locally
5. Rebuild:
   ```bash
   yarn build
   ```

### 2. Adding New Event Handlers

To add support for a new event:

1. **Add handler to `project.yaml`**:
   ```yaml
   - handler: handleNewEvent
     kind: substrate/EventHandler
     filter:
       module: cps
       method: NewEvent
   ```

2. **Implement handler in `src/mappings/cpsHandlers.ts`**:
   ```typescript
   export async function handleNewEvent(event: SubstrateEvent): Promise<void> {
     const { event: { data: [param1, param2] } } = event;
     
     const blockNumber = BigInt(event.block.block.header.number.toString());
     const timestamp = getTimestamp(event.block);
     
     // Your logic here
     
     logger.info(`Processed NewEvent`);
   }
   ```

3. **Export handler from `src/index.ts`**:
   ```typescript
   export * from './mappings/cpsHandlers';
   ```

4. **Regenerate and rebuild**:
   ```bash
   yarn codegen && yarn build
   ```

### 3. Local Testing

#### Quick Test
```bash
# Start local stack
yarn dev

# Monitor logs
docker-compose logs -f subquery-node

# Query GraphQL
open http://localhost:3000
```

#### Running Against Local Node

1. Start a local Robonomics node:
   ```bash
   cd /path/to/robonomics
   cargo run --release -- --dev --tmp
   ```

2. Update `project.yaml`:
   ```yaml
   network:
     endpoint: ws://127.0.0.1:9944
   ```

3. Start indexer:
   ```bash
   yarn dev
   ```

### 4. Testing Strategy

#### Unit Tests

Test individual utility functions:

```bash
yarn test src/__tests__/utils.test.ts
```

Example unit test:
```typescript
import { createCompositeId } from '../mappings/utils';

describe('createCompositeId', () => {
  it('should create composite ID', () => {
    const result = createCompositeId('node', 123);
    expect(result).toBe('node-123');
  });
});
```

#### Integration Tests

Test event handlers with mocked events:

```bash
yarn test src/__tests__/integration.test.ts
```

For full integration testing, use `@subql/testing`:
```bash
yarn add --dev @subql/testing
```

#### Coverage

Generate test coverage report:
```bash
yarn test:coverage
```

### 5. Code Quality

#### Linting

Add ESLint (optional):
```bash
yarn add --dev eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin
```

#### Type Checking

```bash
tsc --noEmit
```

#### Format Code

Add Prettier (optional):
```bash
yarn add --dev prettier
yarn prettier --write "src/**/*.ts"
```

## Debugging

### Enable Debug Logging

Set log level in Docker Compose:
```yaml
subquery-node:
  environment:
    LOG_LEVEL: debug
```

### Using Logger

In your handlers:
```typescript
logger.info('Processing event');
logger.debug('Detailed debug info', { nodeId });
logger.warn('Warning message');
logger.error('Error occurred', error);
```

### Inspect Database

Connect to PostgreSQL:
```bash
docker-compose exec postgres psql -U postgres -d postgres

# List tables
\dt app.*

# Query nodes
SELECT * FROM app.nodes LIMIT 10;

# Query history
SELECT * FROM app.node_histories ORDER BY block_number DESC LIMIT 10;
```

### Query Metadata

Check indexer status:
```graphql
query {
  _metadata {
    lastProcessedHeight
    lastProcessedTimestamp
    targetHeight
    chain
    indexerHealthy
  }
}
```

## Performance Optimization

### 1. Batch Operations

When processing multiple entities:
```typescript
// Bad: Individual saves
await entity1.save();
await entity2.save();
await entity3.save();

// Good: Batch saves (if supported by your SubQuery version)
await Promise.all([
  entity1.save(),
  entity2.save(),
  entity3.save(),
]);
```

### 2. Indexing Strategy

Ensure proper indexes in schema:
```graphql
type Node @entity {
  id: ID!
  owner: String! @index
  parentId: String @index
  isDeleted: Boolean! @index
}
```

### 3. Query Optimization

Use specific filters:
```typescript
// Bad: Load all then filter
const allNodes = await Node.getByFields([]);
const filtered = allNodes.filter(n => n.isDeleted === false);

// Good: Filter at query level
const filtered = await Node.getByFields([['isDeleted', false]]);
```

### 4. Memory Management

For large datasets:
```yaml
subquery-node:
  command:
    - --batch-size=30        # Adjust batch size
    - --workers=4            # Parallel processing
    - --timeout=600000       # Increase timeout if needed
```

## Deployment

### Deploy to SubQuery Managed Service

1. **Build and publish**:
   ```bash
   yarn build
   subql publish
   ```

2. **Deploy**:
   ```bash
   subql deployment:deploy
   ```

3. **Monitor**:
   - Check deployment status in SubQuery dashboard
   - Monitor indexing progress
   - Set up alerts for failures

### Self-Hosted Deployment

#### Using Docker

1. **Build image**:
   ```bash
   docker build -t robonomics-cps-indexer .
   ```

2. **Run with docker-compose**:
   ```bash
   docker-compose -f docker-compose.prod.yml up -d
   ```

#### Using Kubernetes

Create deployment manifests:
- PostgreSQL StatefulSet
- SubQuery Node Deployment
- GraphQL Query Service
- Ingress for external access

### Monitoring

Set up monitoring:
- **Metrics**: Prometheus + Grafana
- **Logs**: ELK stack or CloudWatch
- **Alerts**: PagerDuty or similar

Key metrics to monitor:
- Last processed block height
- Processing rate (blocks/second)
- Query response times
- Error rates
- Database size

## Troubleshooting

### Common Issues

#### 1. "Cannot find module" errors

**Solution**: Regenerate types
```bash
rm -rf src/types
yarn codegen
yarn build
```

#### 2. Database connection errors

**Solution**: Check PostgreSQL is running
```bash
docker-compose ps postgres
docker-compose logs postgres
```

#### 3. Indexing stuck at block

**Solution**: Check logs and restart
```bash
docker-compose logs subquery-node
docker-compose restart subquery-node
```

#### 4. GraphQL schema mismatch

**Solution**: Regenerate schema
```bash
yarn codegen
yarn build
docker-compose down
docker-compose up -d
```

#### 5. Out of memory

**Solution**: Adjust Docker resources or batch size
```yaml
subquery-node:
  command:
    - --batch-size=10  # Reduce batch size
  deploy:
    resources:
      limits:
        memory: 4G
```

### Debug Checklist

- [ ] Check SubQuery node logs
- [ ] Verify network endpoint is accessible
- [ ] Confirm schema matches entities
- [ ] Check database connection
- [ ] Verify chain ID and genesis hash
- [ ] Test event filters in project.yaml
- [ ] Validate handler implementations
- [ ] Check for type mismatches

### Getting Help

- [SubQuery Discord](https://discord.com/invite/subquery)
- [Robonomics Discord](https://discord.gg/robonomics)
- [GitHub Issues](https://github.com/airalab/robonomics/issues)
- [SubQuery Docs](https://academy.subquery.network/)

## Best Practices

1. **Always use TypeScript strict mode**
2. **Write tests for all handlers**
3. **Use meaningful entity IDs**
4. **Add comprehensive logging**
5. **Handle edge cases (null, undefined)**
6. **Keep handlers idempotent** (safe to reprocess)
7. **Version your schema changes**
8. **Document complex logic**
9. **Monitor performance regularly**
10. **Keep dependencies updated**

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

Follow the [Robonomics contribution guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

---

**Happy coding!** 🚀
