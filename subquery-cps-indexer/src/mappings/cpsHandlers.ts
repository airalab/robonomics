import { SubstrateEvent } from '@subql/types';
import { Node, NodeHistory, OwnerIndex } from '../types';
import {
  getOrCreateStatistics,
  getOrCreateDailyStats,
  getTimestamp,
  parseNodeData,
  createCompositeId,
  ensureString,
} from './utils';

/**
 * Handler for NodeCreated event
 * Event signature: NodeCreated(NodeId, Option<NodeId>, T::AccountId)
 */
export async function handleNodeCreated(event: SubstrateEvent): Promise<void> {
  const {
    event: {
      data: [nodeId, parentId, owner],
    },
  } = event;

  logger.info(`Processing NodeCreated event for node ${nodeId.toString()}`);

  const blockNumber = BigInt(event.block.block.header.number.toString());
  const timestamp = getTimestamp(event.block);
  const txHash = event.extrinsic?.extrinsic.hash.toString() || 'unknown';

  // Create the node entity
  const node = Node.create({
    id: nodeId.toString(),
    parentId: parentId.isSome ? parentId.unwrap().toString() : undefined,
    owner: ensureString(owner),
    metaType: undefined,
    metaData: undefined,
    metaAlgorithm: undefined,
    payloadType: undefined,
    payloadData: undefined,
    payloadAlgorithm: undefined,
    createdAtBlock: blockNumber,
    createdAt: timestamp,
    updatedAtBlock: blockNumber,
    updatedAt: timestamp,
    isDeleted: false,
    deletedAtBlock: undefined,
    deletedAt: undefined,
  });

  await node.save();

  // Create history record
  const historyId = createCompositeId(nodeId.toString(), blockNumber, event.idx);
  const history = NodeHistory.create({
    id: historyId,
    nodeId: nodeId.toString(),
    action: 'CREATED',
    blockNumber,
    timestamp,
    txHash,
    actor: ensureString(owner),
    oldValue: undefined,
    newValue: JSON.stringify({
      parentId: node.parentId,
      owner: node.owner,
    }),
    oldParentId: undefined,
    newParentId: node.parentId,
  });

  await history.save();

  // Create owner index
  const ownerIndexId = createCompositeId(ensureString(owner), nodeId.toString());
  const ownerIndex = OwnerIndex.create({
    id: ownerIndexId,
    owner: ensureString(owner),
    nodeId: nodeId.toString(),
    createdAtBlock: blockNumber,
  });

  await ownerIndex.save();

  // Update statistics
  const stats = await getOrCreateStatistics();
  stats.totalNodesCreated = stats.totalNodesCreated + BigInt(1);
  stats.activeNodes = stats.activeNodes + BigInt(1);
  
  // Update root nodes count if this is a root
  if (!node.parentId) {
    stats.rootNodes = stats.rootNodes + BigInt(1);
  }
  
  stats.lastUpdatedBlock = blockNumber;
  stats.lastUpdatedAt = timestamp;
  await stats.save();

  // Update daily stats
  const dailyStats = await getOrCreateDailyStats(timestamp);
  dailyStats.nodesCreated += 1;
  await dailyStats.save();

  logger.info(`Successfully processed NodeCreated event for node ${nodeId.toString()}`);
}

/**
 * Handler for MetaSet event
 * Event signature: MetaSet(NodeId, T::AccountId)
 */
export async function handleMetaSet(event: SubstrateEvent): Promise<void> {
  const {
    event: {
      data: [nodeId, owner],
    },
  } = event;

  logger.info(`Processing MetaSet event for node ${nodeId.toString()}`);

  const blockNumber = BigInt(event.block.block.header.number.toString());
  const timestamp = getTimestamp(event.block);
  const txHash = event.extrinsic?.extrinsic.hash.toString() || 'unknown';

  // Get the node
  const node = await Node.get(nodeId.toString());
  if (!node) {
    logger.warn(`Node ${nodeId.toString()} not found for MetaSet event`);
    return;
  }

  // Store old metadata
  const oldMeta = {
    type: node.metaType,
    data: node.metaData,
    algorithm: node.metaAlgorithm,
  };

  // Fetch the updated node data from chain state
  // Note: In a real scenario, you would query the chain state here
  // For now, we mark it as updated and will capture the actual data
  // when the state is queried by the subquery node
  
  node.updatedAtBlock = blockNumber;
  node.updatedAt = timestamp;
  
  await node.save();

  // Create history record
  const historyId = createCompositeId(nodeId.toString(), blockNumber, event.idx);
  const history = NodeHistory.create({
    id: historyId,
    nodeId: nodeId.toString(),
    action: 'META_SET',
    blockNumber,
    timestamp,
    txHash,
    actor: ensureString(owner),
    oldValue: JSON.stringify(oldMeta),
    newValue: 'Updated - query state for current value',
    oldParentId: undefined,
    newParentId: undefined,
  });

  await history.save();

  // Update statistics
  const stats = await getOrCreateStatistics();
  stats.metaUpdates = stats.metaUpdates + BigInt(1);
  stats.lastUpdatedBlock = blockNumber;
  stats.lastUpdatedAt = timestamp;
  await stats.save();

  // Update daily stats
  const dailyStats = await getOrCreateDailyStats(timestamp);
  dailyStats.metaUpdates += 1;
  await dailyStats.save();

  logger.info(`Successfully processed MetaSet event for node ${nodeId.toString()}`);
}

/**
 * Handler for PayloadSet event
 * Event signature: PayloadSet(NodeId, T::AccountId)
 */
export async function handlePayloadSet(event: SubstrateEvent): Promise<void> {
  const {
    event: {
      data: [nodeId, owner],
    },
  } = event;

  logger.info(`Processing PayloadSet event for node ${nodeId.toString()}`);

  const blockNumber = BigInt(event.block.block.header.number.toString());
  const timestamp = getTimestamp(event.block);
  const txHash = event.extrinsic?.extrinsic.hash.toString() || 'unknown';

  // Get the node
  const node = await Node.get(nodeId.toString());
  if (!node) {
    logger.warn(`Node ${nodeId.toString()} not found for PayloadSet event`);
    return;
  }

  // Store old payload
  const oldPayload = {
    type: node.payloadType,
    data: node.payloadData,
    algorithm: node.payloadAlgorithm,
  };

  node.updatedAtBlock = blockNumber;
  node.updatedAt = timestamp;
  
  await node.save();

  // Create history record
  const historyId = createCompositeId(nodeId.toString(), blockNumber, event.idx);
  const history = NodeHistory.create({
    id: historyId,
    nodeId: nodeId.toString(),
    action: 'PAYLOAD_SET',
    blockNumber,
    timestamp,
    txHash,
    actor: ensureString(owner),
    oldValue: JSON.stringify(oldPayload),
    newValue: 'Updated - query state for current value',
    oldParentId: undefined,
    newParentId: undefined,
  });

  await history.save();

  // Update statistics
  const stats = await getOrCreateStatistics();
  stats.payloadUpdates = stats.payloadUpdates + BigInt(1);
  stats.lastUpdatedBlock = blockNumber;
  stats.lastUpdatedAt = timestamp;
  await stats.save();

  // Update daily stats
  const dailyStats = await getOrCreateDailyStats(timestamp);
  dailyStats.payloadUpdates += 1;
  await dailyStats.save();

  logger.info(`Successfully processed PayloadSet event for node ${nodeId.toString()}`);
}

/**
 * Handler for NodeMoved event
 * Event signature: NodeMoved(NodeId, Option<NodeId>, NodeId, T::AccountId)
 */
export async function handleNodeMoved(event: SubstrateEvent): Promise<void> {
  const {
    event: {
      data: [nodeId, oldParentId, newParentId, owner],
    },
  } = event;

  logger.info(`Processing NodeMoved event for node ${nodeId.toString()}`);

  const blockNumber = BigInt(event.block.block.header.number.toString());
  const timestamp = getTimestamp(event.block);
  const txHash = event.extrinsic?.extrinsic.hash.toString() || 'unknown';

  // Get the node
  const node = await Node.get(nodeId.toString());
  if (!node) {
    logger.warn(`Node ${nodeId.toString()} not found for NodeMoved event`);
    return;
  }

  const oldParent = oldParentId.isSome ? oldParentId.unwrap().toString() : undefined;
  const newParent = newParentId.toString();

  // Update node parent
  node.parentId = newParent;
  node.updatedAtBlock = blockNumber;
  node.updatedAt = timestamp;
  
  await node.save();

  // Create history record
  const historyId = createCompositeId(nodeId.toString(), blockNumber, event.idx);
  const history = NodeHistory.create({
    id: historyId,
    nodeId: nodeId.toString(),
    action: 'MOVED',
    blockNumber,
    timestamp,
    txHash,
    actor: ensureString(owner),
    oldValue: JSON.stringify({ parentId: oldParent }),
    newValue: JSON.stringify({ parentId: newParent }),
    oldParentId: oldParent,
    newParentId: newParent,
  });

  await history.save();

  // Update statistics
  const stats = await getOrCreateStatistics();
  stats.nodeMoves = stats.nodeMoves + BigInt(1);
  
  // Update root nodes count
  if (!oldParent && newParent) {
    // Was root, now has parent
    stats.rootNodes = stats.rootNodes - BigInt(1);
  } else if (oldParent && !newParent) {
    // Had parent, now is root
    stats.rootNodes = stats.rootNodes + BigInt(1);
  }
  
  stats.lastUpdatedBlock = blockNumber;
  stats.lastUpdatedAt = timestamp;
  await stats.save();

  // Update daily stats
  const dailyStats = await getOrCreateDailyStats(timestamp);
  dailyStats.nodeMoves += 1;
  await dailyStats.save();

  logger.info(`Successfully processed NodeMoved event for node ${nodeId.toString()}`);
}

/**
 * Handler for NodeDeleted event
 * Event signature: NodeDeleted(NodeId, T::AccountId)
 */
export async function handleNodeDeleted(event: SubstrateEvent): Promise<void> {
  const {
    event: {
      data: [nodeId, owner],
    },
  } = event;

  logger.info(`Processing NodeDeleted event for node ${nodeId.toString()}`);

  const blockNumber = BigInt(event.block.block.header.number.toString());
  const timestamp = getTimestamp(event.block);
  const txHash = event.extrinsic?.extrinsic.hash.toString() || 'unknown';

  // Get the node
  const node = await Node.get(nodeId.toString());
  if (!node) {
    logger.warn(`Node ${nodeId.toString()} not found for NodeDeleted event`);
    return;
  }

  // Mark as deleted
  node.isDeleted = true;
  node.deletedAtBlock = blockNumber;
  node.deletedAt = timestamp;
  node.updatedAtBlock = blockNumber;
  node.updatedAt = timestamp;
  
  await node.save();

  // Create history record
  const historyId = createCompositeId(nodeId.toString(), blockNumber, event.idx);
  const history = NodeHistory.create({
    id: historyId,
    nodeId: nodeId.toString(),
    action: 'DELETED',
    blockNumber,
    timestamp,
    txHash,
    actor: ensureString(owner),
    oldValue: JSON.stringify({
      parentId: node.parentId,
      owner: node.owner,
    }),
    newValue: undefined,
    oldParentId: undefined,
    newParentId: undefined,
  });

  await history.save();

  // Update statistics
  const stats = await getOrCreateStatistics();
  stats.activeNodes = stats.activeNodes - BigInt(1);
  stats.deletedNodes = stats.deletedNodes + BigInt(1);
  
  // Update root nodes count if this was a root
  if (!node.parentId) {
    stats.rootNodes = stats.rootNodes - BigInt(1);
  }
  
  stats.lastUpdatedBlock = blockNumber;
  stats.lastUpdatedAt = timestamp;
  await stats.save();

  // Update daily stats
  const dailyStats = await getOrCreateDailyStats(timestamp);
  dailyStats.nodesDeleted += 1;
  await dailyStats.save();

  logger.info(`Successfully processed NodeDeleted event for node ${nodeId.toString()}`);
}
