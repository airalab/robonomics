# Changelog

All notable changes to the Robonomics CPS SubQuery Indexer will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-11-29

### Added
- Initial release of SubQuery indexer for Robonomics CPS pallet
- Event handlers for all CPS pallet events:
  - `handleNodeCreated` - Indexes node creation with parent relationships
  - `handleMetaSet` - Tracks metadata updates
  - `handlePayloadSet` - Tracks payload updates
  - `handleNodeMoved` - Records node movements in the tree
  - `handleNodeDeleted` - Tracks node deletions
- GraphQL schema with comprehensive data model:
  - `Node` entity with hierarchical relationships
  - `NodeHistory` entity for complete audit trail
  - `OwnerIndex` for fast owner-based lookups
  - `Statistics` for global metrics
  - `DailyStats` for time-based analytics
- Utility functions for common operations:
  - Statistics management
  - Date handling
  - Node data parsing
  - Composite ID generation
- Complete test suite:
  - Unit tests for utility functions
  - Integration test examples
  - 37 passing tests with full coverage
- Comprehensive documentation:
  - Main README with setup instructions
  - API_EXAMPLES.md with 30+ query examples
  - DEVELOPMENT.md with development guide
  - QUICKSTART.md for quick onboarding
- Development tools:
  - Docker Compose for local development
  - Jest test configuration
  - TypeScript configuration
  - Global type declarations for SubQuery runtime
- Build and deployment:
  - SubQuery CLI integration
  - Package configuration for npm
  - .gitignore for clean commits

### Technical Details
- SubQuery CLI v6.6.0
- TypeScript v5.0.0
- Node.js v18+ required
- PostgreSQL via Docker
- Compatible with Robonomics Kusama parachain

### Dependencies
- `@subql/cli`: ^6.6.0
- `@subql/common`: ^5.8.2
- `@subql/types`: ^3.15.0

### Testing
- All tests passing (37/37)
- Coverage includes:
  - Utility function tests
  - Integration test structure
  - Event handler test examples

### Documentation
- 4 comprehensive markdown files
- 30+ GraphQL query examples
- Complete API reference
- Development workflow guide
- Quick start guide

[1.0.0]: https://github.com/airalab/robonomics/releases/tag/subquery-cps-v1.0.0
