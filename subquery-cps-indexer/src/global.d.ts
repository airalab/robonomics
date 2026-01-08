// Global type declarations for SubQuery runtime
import { Store } from '@subql/types';

declare global {
  const store: Store;
  const logger: {
    info: (message: string, ...args: any[]) => void;
    debug: (message: string, ...args: any[]) => void;
    warn: (message: string, ...args: any[]) => void;
    error: (message: string, ...args: any[]) => void;
  };
}

export {};
