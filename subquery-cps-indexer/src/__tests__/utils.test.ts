import { parseNodeData, ensureString, createCompositeId } from '../mappings/utils';

describe('Utils', () => {
  describe('parseNodeData', () => {
    it('should parse plain data', () => {
      const mockPlainData = {
        isPlain: true,
        asPlain: {
          toHex: () => '0x48656c6c6f',
        },
        isEncrypted: false,
      };

      const result = parseNodeData(mockPlainData);

      expect(result).toBeDefined();
      expect(result?.type).toBe('Plain');
      expect(result?.data).toBe('0x48656c6c6f');
      expect(result?.algorithm).toBeUndefined();
    });

    it('should parse encrypted data', () => {
      const mockEncryptedData = {
        isPlain: false,
        isEncrypted: true,
        asEncrypted: {
          ciphertext: {
            toHex: () => '0x1234567890abcdef',
          },
          algorithm: {
            toString: () => 'XChaCha20Poly1305',
          },
        },
      };

      const result = parseNodeData(mockEncryptedData);

      expect(result).toBeDefined();
      expect(result?.type).toBe('Encrypted');
      expect(result?.data).toBe('0x1234567890abcdef');
      expect(result?.algorithm).toBe('XChaCha20Poly1305');
    });

    it('should return null for undefined data', () => {
      const result = parseNodeData(undefined);
      expect(result).toBeNull();
    });

    it('should return null for null data', () => {
      const result = parseNodeData(null);
      expect(result).toBeNull();
    });
  });

  describe('ensureString', () => {
    it('should return string as-is', () => {
      const result = ensureString('test');
      expect(result).toBe('test');
    });

    it('should convert AccountId to string', () => {
      const mockAccountId = {
        toString: () => '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
      };
      const result = ensureString(mockAccountId);
      expect(result).toBe('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
    });

    it('should handle number conversion', () => {
      const result = ensureString(123);
      expect(result).toBe('123');
    });
  });

  describe('createCompositeId', () => {
    it('should create composite ID from multiple strings', () => {
      const result = createCompositeId('node', '123', 'block', '456');
      expect(result).toBe('node-123-block-456');
    });

    it('should create composite ID from numbers', () => {
      const result = createCompositeId(0, 100, 5);
      expect(result).toBe('0-100-5');
    });

    it('should create composite ID from bigints', () => {
      const result = createCompositeId(BigInt(1), BigInt(2));
      expect(result).toBe('1-2');
    });

    it('should create composite ID from mixed types', () => {
      const result = createCompositeId('node', 123, BigInt(456));
      expect(result).toBe('node-123-456');
    });

    it('should handle single value', () => {
      const result = createCompositeId('single');
      expect(result).toBe('single');
    });
  });
});
