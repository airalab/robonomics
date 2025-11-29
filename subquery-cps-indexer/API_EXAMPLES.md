# GraphQL API Query Examples

This document provides comprehensive examples of GraphQL queries you can use with the Robonomics CPS SubQuery indexer.

## Table of Contents

- [Basic Queries](#basic-queries)
- [Tree Navigation](#tree-navigation)
- [Owner Queries](#owner-queries)
- [History and Audit Trail](#history-and-audit-trail)
- [Statistics](#statistics)
- [Advanced Filtering](#advanced-filtering)
- [Pagination](#pagination)
- [Aggregations](#aggregations)

## Basic Queries

### Get a Single Node

```graphql
query {
  node(id: "0") {
    id
    owner
    parentId
    metaType
    metaData
    metaAlgorithm
    payloadType
    payloadData
    payloadAlgorithm
    createdAt
    updatedAt
    isDeleted
  }
}
```

### Get All Active Nodes

```graphql
query {
  nodes(filter: { isDeleted: { equalTo: false } }) {
    totalCount
    nodes {
      id
      owner
      parentId
      createdAt
    }
  }
}
```

### Get All Root Nodes

```graphql
query {
  nodes(
    filter: {
      parentId: { isNull: true }
      isDeleted: { equalTo: false }
    }
  ) {
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

## Tree Navigation

### Get Complete Tree from Root (3 levels deep)

```graphql
query {
  node(id: "0") {
    id
    owner
    metaType
    createdAt
    children {
      id
      owner
      metaType
      createdAt
      children {
        id
        owner
        metaType
        createdAt
        children {
          id
          owner
          metaType
          createdAt
        }
      }
    }
  }
}
```

### Get Parent Chain

```graphql
query {
  node(id: "5") {
    id
    owner
    parent {
      id
      owner
      parent {
        id
        owner
        parent {
          id
          owner
        }
      }
    }
  }
}
```

### Get All Children of a Node

```graphql
query {
  node(id: "0") {
    id
    owner
    children {
      id
      owner
      metaType
      payloadType
      createdAt
    }
  }
}
```

### Get Siblings (Nodes with Same Parent)

```graphql
query {
  nodes(filter: { parentId: { equalTo: "0" } }) {
    nodes {
      id
      owner
      createdAt
    }
  }
}
```

## Owner Queries

### Get All Nodes by Owner

```graphql
query {
  ownerIndices(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
    }
  ) {
    nodes {
      node {
        id
        parentId
        owner
        createdAt
        isDeleted
        metaType
        payloadType
      }
    }
  }
}
```

### Get Active Nodes by Owner

```graphql
query {
  nodes(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
      isDeleted: { equalTo: false }
    }
  ) {
    totalCount
    nodes {
      id
      parentId
      createdAt
      updatedAt
    }
  }
}
```

### Get Root Nodes by Owner

```graphql
query {
  nodes(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
      parentId: { isNull: true }
      isDeleted: { equalTo: false }
    }
  ) {
    nodes {
      id
      createdAt
      children {
        id
      }
    }
  }
}
```

## History and Audit Trail

### Get Complete History for a Node

```graphql
query {
  nodeHistories(
    filter: { nodeId: { equalTo: "0" } }
    orderBy: BLOCK_NUMBER_ASC
  ) {
    nodes {
      id
      action
      blockNumber
      timestamp
      actor
      oldValue
      newValue
      oldParentId
      newParentId
      txHash
    }
  }
}
```

### Get Recent Activity Across All Nodes

```graphql
query {
  nodeHistories(
    orderBy: BLOCK_NUMBER_DESC
    first: 50
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
      txHash
    }
  }
}
```

### Get All Node Creations

```graphql
query {
  nodeHistories(
    filter: { action: { equalTo: "CREATED" } }
    orderBy: BLOCK_NUMBER_DESC
  ) {
    nodes {
      nodeId
      blockNumber
      timestamp
      actor
      newValue
    }
  }
}
```

### Get All Node Movements

```graphql
query {
  nodeHistories(
    filter: { action: { equalTo: "MOVED" } }
    orderBy: BLOCK_NUMBER_DESC
  ) {
    nodes {
      nodeId
      blockNumber
      timestamp
      oldParentId
      newParentId
      actor
    }
  }
}
```

### Get Activity by Actor

```graphql
query {
  nodeHistories(
    filter: {
      actor: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
    }
    orderBy: TIMESTAMP_DESC
  ) {
    nodes {
      action
      timestamp
      node {
        id
      }
    }
  }
}
```

### Get History in Date Range

```graphql
query {
  nodeHistories(
    filter: {
      timestamp: {
        greaterThan: "2025-01-01T00:00:00Z"
        lessThan: "2025-01-31T23:59:59Z"
      }
    }
    orderBy: TIMESTAMP_ASC
  ) {
    nodes {
      action
      timestamp
      nodeId
      actor
    }
  }
}
```

## Statistics

### Get Global Statistics

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
    lastUpdatedBlock
    lastUpdatedAt
  }
}
```

### Get Daily Statistics

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

### Get Statistics for Date Range

```graphql
query {
  dailyStats(
    filter: {
      date: {
        greaterThanOrEqualTo: "2025-01-01"
        lessThanOrEqualTo: "2025-01-31"
      }
    }
    orderBy: DATE_ASC
  ) {
    nodes {
      id
      date
      nodesCreated
      nodesDeleted
      metaUpdates
      payloadUpdates
      nodeMoves
    }
  }
}
```

### Get Most Active Days

```graphql
query {
  dailyStats(
    orderBy: NODES_CREATED_DESC
    first: 10
  ) {
    nodes {
      date
      nodesCreated
      nodesDeleted
      metaUpdates
    }
  }
}
```

## Advanced Filtering

### Search Nodes with Multiple Filters

```graphql
query {
  nodes(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
      isDeleted: { equalTo: false }
      metaType: { equalTo: "Plain" }
      createdAt: { greaterThan: "2025-01-01T00:00:00Z" }
    }
    orderBy: CREATED_AT_DESC
  ) {
    totalCount
    nodes {
      id
      parentId
      metaData
      createdAt
    }
  }
}
```

### Find Encrypted Nodes

```graphql
query {
  nodes(
    filter: {
      payloadType: { equalTo: "Encrypted" }
      isDeleted: { equalTo: false }
    }
  ) {
    nodes {
      id
      owner
      payloadAlgorithm
      createdAt
    }
  }
}
```

### Find Recently Updated Nodes

```graphql
query {
  nodes(
    filter: {
      isDeleted: { equalTo: false }
      updatedAt: { greaterThan: "2025-01-15T00:00:00Z" }
    }
    orderBy: UPDATED_AT_DESC
    first: 20
  ) {
    nodes {
      id
      owner
      updatedAt
      history(first: 1, orderBy: BLOCK_NUMBER_DESC) {
        nodes {
          action
          timestamp
        }
      }
    }
  }
}
```

### Find Deleted Nodes

```graphql
query {
  nodes(
    filter: { isDeleted: { equalTo: true } }
    orderBy: DELETED_AT_DESC
  ) {
    nodes {
      id
      owner
      deletedAt
      deletedAtBlock
    }
  }
}
```

## Pagination

### Basic Pagination

```graphql
query {
  nodes(
    first: 10
    offset: 0
    orderBy: CREATED_AT_DESC
  ) {
    totalCount
    nodes {
      id
      owner
      createdAt
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
  }
}
```

### Cursor-based Pagination

```graphql
query {
  nodes(
    first: 10
    after: "WyJjcmVhdGVkX2F0IiwiMjAyNS0wMS0xNVQxMDozMDowMFoiXQ=="
    orderBy: CREATED_AT_DESC
  ) {
    nodes {
      id
      owner
      createdAt
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

## Aggregations

### Count Nodes by Owner

```graphql
query {
  ownerIndices(
    groupBy: OWNER
  ) {
    groupedAggregates {
      keys
      count {
        id
      }
    }
  }
}
```

### Count Active vs Deleted Nodes

```graphql
query {
  active: nodes(filter: { isDeleted: { equalTo: false } }) {
    totalCount
  }
  deleted: nodes(filter: { isDeleted: { equalTo: true } }) {
    totalCount
  }
}
```

### Count Encrypted vs Plain Nodes

```graphql
query {
  encrypted: nodes(filter: { payloadType: { equalTo: "Encrypted" } }) {
    totalCount
  }
  plain: nodes(filter: { payloadType: { equalTo: "Plain" } }) {
    totalCount
  }
  noPayload: nodes(filter: { payloadType: { isNull: true } }) {
    totalCount
  }
}
```

## Complex Queries

### Get Node with Full Context

```graphql
query {
  node(id: "5") {
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
    
    # Get parent info
    parent {
      id
      owner
    }
    
    # Get children
    children {
      id
      owner
    }
    
    # Get recent history
    history(first: 10, orderBy: BLOCK_NUMBER_DESC) {
      nodes {
        action
        timestamp
        actor
      }
    }
  }
}
```

### Dashboard Query (Multiple Stats)

```graphql
query {
  stats: statistics(id: "global") {
    totalNodesCreated
    activeNodes
    deletedNodes
    rootNodes
    metaUpdates
    payloadUpdates
    nodeMoves
    lastUpdatedAt
  }
  
  recentNodes: nodes(
    first: 5
    orderBy: CREATED_AT_DESC
    filter: { isDeleted: { equalTo: false } }
  ) {
    nodes {
      id
      owner
      createdAt
    }
  }
  
  recentActivity: nodeHistories(
    first: 10
    orderBy: BLOCK_NUMBER_DESC
  ) {
    nodes {
      action
      timestamp
      nodeId
      actor
    }
  }
  
  today: dailyStats(id: "2025-01-15") {
    nodesCreated
    nodesDeleted
    metaUpdates
    payloadUpdates
    nodeMoves
  }
}
```

### Owner Dashboard

```graphql
query {
  ownerIndices(
    filter: {
      owner: { equalTo: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" }
    }
  ) {
    totalCount
    nodes {
      node {
        id
        parentId
        isDeleted
        createdAt
        
        # Get children count
        children {
          id
        }
        
        # Get recent activity for this node
        history(first: 5, orderBy: BLOCK_NUMBER_DESC) {
          nodes {
            action
            timestamp
          }
        }
      }
    }
  }
}
```

## Subscriptions (if enabled)

### Subscribe to New Nodes

```graphql
subscription {
  nodeCreated {
    id
    owner
    parentId
    createdAt
  }
}
```

### Subscribe to Node Updates

```graphql
subscription {
  nodeHistoryCreated {
    nodeId
    action
    timestamp
    actor
  }
}
```

---

## Tips for Efficient Queries

1. **Use Specific Filters**: Always filter as much as possible to reduce result set size
2. **Limit Results**: Use `first` parameter to limit results, especially for large datasets
3. **Use Indexes**: Queries on `owner`, `parentId`, and `isDeleted` are optimized with indexes
4. **Pagination**: For large result sets, use cursor-based pagination
5. **Avoid Deep Nesting**: Limit depth when traversing tree structures to avoid performance issues
6. **Use Projections**: Only query the fields you need

## Common Patterns

### Pattern: Find a user's tree structure
1. Query owner's root nodes
2. For each root, recursively query children

### Pattern: Audit trail analysis
1. Query NodeHistory with date filters
2. Group by action type
3. Analyze patterns

### Pattern: Real-time monitoring
1. Query recent NodeHistory entries
2. Poll at regular intervals or use subscriptions
3. Alert on specific actions

---

For more query examples and GraphQL documentation, visit:
- [GraphQL Official Docs](https://graphql.org/learn/)
- [SubQuery Documentation](https://academy.subquery.network/)
