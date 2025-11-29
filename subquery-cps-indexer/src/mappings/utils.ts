import { Statistics, DailyStats } from '../types';

/**
 * Get or create global statistics entity
 */
export async function getOrCreateStatistics(): Promise<Statistics> {
  const id = 'global';
  let stats = await Statistics.get(id);
  
  if (!stats) {
    stats = Statistics.create({
      id,
      totalNodesCreated: BigInt(0),
      activeNodes: BigInt(0),
      deletedNodes: BigInt(0),
      rootNodes: BigInt(0),
      metaUpdates: BigInt(0),
      payloadUpdates: BigInt(0),
      nodeMoves: BigInt(0),
      lastUpdatedBlock: BigInt(0),
      lastUpdatedAt: new Date(0),
    });
  }
  
  return stats;
}

/**
 * Get or create daily statistics for a given date
 */
export async function getOrCreateDailyStats(date: Date): Promise<DailyStats> {
  // Format date as YYYY-MM-DD
  const dateStr = date.toISOString().split('T')[0];
  
  let dailyStats = await DailyStats.get(dateStr);
  
  if (!dailyStats) {
    dailyStats = DailyStats.create({
      id: dateStr,
      nodesCreated: 0,
      nodesDeleted: 0,
      metaUpdates: 0,
      payloadUpdates: 0,
      nodeMoves: 0,
      date,
    });
  }
  
  return dailyStats;
}

/**
 * Convert a Substrate event timestamp to Date
 */
export function getTimestamp(block: any): Date {
  const timestamp = block.timestamp || block.block.timestamp;
  return timestamp ? new Date(timestamp.toNumber()) : new Date();
}

/**
 * Parse node data from event
 * Returns object with type, data, and algorithm (if encrypted)
 */
export function parseNodeData(data: any): {
  type: string;
  data: string;
  algorithm?: string;
} | null {
  if (!data) return null;
  
  // Check if data is Plain or Encrypted
  if (data.isPlain) {
    return {
      type: 'Plain',
      data: data.asPlain.toHex(),
    };
  } else if (data.isEncrypted) {
    const encrypted = data.asEncrypted;
    return {
      type: 'Encrypted',
      data: encrypted.ciphertext.toHex(),
      algorithm: encrypted.algorithm.toString(),
    };
  }
  
  return null;
}

/**
 * Create a composite ID from multiple parts
 */
export function createCompositeId(...parts: (string | number | bigint)[]): string {
  return parts.map(p => p.toString()).join('-');
}

/**
 * Ensure a value is a string (convert from AccountId or other types)
 */
export function ensureString(value: any): string {
  if (typeof value === 'string') {
    return value;
  }
  if (value && typeof value.toString === 'function') {
    return value.toString();
  }
  return String(value);
}

/**
 * Unwrap Substrate Option type to string or undefined
 * Handles Codec Option types from Substrate events
 */
export function unwrapOption(option: any): string | undefined {
  if (!option) return undefined;
  
  // Check if it's a Substrate Option type
  if (typeof option === 'object' && 'isSome' in option) {
    return option.isSome && option.unwrap ? option.unwrap().toString() : undefined;
  }
  
  // Already unwrapped or not an Option
  return undefined;
}
