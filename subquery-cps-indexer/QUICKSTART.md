# SubQuery CPS Indexer - Quick Start

## What is this?

This SubQuery indexer tracks all events from the Robonomics CPS (Cyber-Physical Systems) pallet and provides a GraphQL API for querying the hierarchical node structure, ownership information, and complete audit trails.

## Features

✅ **5 Event Handlers**:
- NodeCreated
- MetaSet
- PayloadSet
- NodeMoved
- NodeDeleted

✅ **Rich Database Schema**:
- Nodes with parent-child relationships
- Complete history/audit trail
- Owner indexes for fast lookups
- Global and daily statistics

✅ **Production Ready**:
- Full TypeScript with type safety
- 37 passing unit tests
- Comprehensive documentation
- Docker Compose for local dev

## Quick Start (5 minutes)

### 1. Install Dependencies
```bash
cd subquery-cps-indexer
npm install
```

### 2. Generate Types
```bash
npm run codegen
```

### 3. Build
```bash
npm run build
```

### 4. Run Locally
```bash
npm run start:docker
```

This starts:
- PostgreSQL database
- SubQuery indexer (syncing from chain)
- GraphQL API at http://localhost:3000

### 5. Query Your Data

Open http://localhost:3000 in your browser and try:

```graphql
query {
  nodes(first: 10) {
    nodes {
      id
      owner
      createdAt
    }
  }
}
```

## Project Structure

```
subquery-cps-indexer/
├── src/
│   ├── mappings/
│   │   ├── cpsHandlers.ts   # Event handler implementations
│   │   └── utils.ts         # Helper functions
│   ├── __tests__/           # Unit tests
│   ├── index.ts             # Main entry
│   └── global.d.ts          # Type declarations
├── schema.graphql           # Database schema
├── project.yaml             # SubQuery configuration
└── docker-compose.yml       # Local dev stack
```

## Next Steps

- **Documentation**: See [README.md](./README.md) for full documentation
- **API Examples**: Check [API_EXAMPLES.md](./API_EXAMPLES.md) for query examples
- **Development**: Read [DEVELOPMENT.md](./DEVELOPMENT.md) for development guide

## Testing

```bash
# Run all tests
npm test

# Run with coverage
npm run test:coverage
```

## Configuration

Edit `project.yaml` to:
- Change the network endpoint
- Adjust the start block
- Modify chain ID

## Deployment

### SubQuery Managed Service
```bash
subql publish
subql deployment:deploy
```

### Self-Hosted
Use the included `docker-compose.yml` or deploy to Kubernetes.

## Support

- [Robonomics Discord](https://discord.gg/robonomics)
- [SubQuery Discord](https://discord.com/invite/subquery)
- [GitHub Issues](https://github.com/airalab/robonomics/issues)

## License

Apache License 2.0 - See [LICENSE](../LICENSE)
