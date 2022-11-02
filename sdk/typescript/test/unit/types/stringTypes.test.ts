// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import {
  isValidTransactionDigest,
  isValidSuiAddress,
} from '../../../src/index';

describe('String type guards', () => {
  function expectAll<T>(data: T[], check: (value: T) => any, expected: any) {
    data.forEach((d) => expect(check(d)).toBe(expected));
  }

  describe('isValidTransactionDigest()', () => {
    it('rejects non base64 strings', () => {
      expectAll(
        [
          'MDpQc 1IIzkie1dJdj nfm85XmRCJmk KHVUU05Abg==',
          'X09wJFxwQDdTU1tzMy5NJXdSTnknPCh9J0tNUCdmIw  ',
        ],
        isValidTransactionDigest,
        false
      );
    });

    it('rejects base64 strings of the wrong length', () => {
      expectAll(
        [
          'ZVteaEsxe0Q6XU53UExxWEFjKy98UD5qfmM+',
          'J3pwOz9GdS5JSEB8Lz9ILGxdJi9sXTxbdFU2OHpP',
          'UUQmaXAmQiYxSERrQH5VWEJmQm8pMXMiYEQzJ2wpPnkuYg==',
        ],
        isValidTransactionDigest,
        false
      );
    });

    it('accepts base64 strings of the correct length', () => {
      expectAll(
        [
          'UYKbz61ny/+E+r07JatGyrtrv/FyjNeqUEQisJJXPHM=',
          'obGrcB0a+aMJXyRMGQ+7to5GaJ6a1Kfd6tS+sAM0d/8=',
          'pMmQoBeSSErk96hKMtkilwCZub3FaOF3IIdii16/DBo=',
        ],
        isValidTransactionDigest,
        true
      );
    });
  });

  describe('isValidSuiAddress', () => {
    it('rejects invalid address', () => {
      expectAll(
        ['MDpQc 1IIzkie1dJdj nfm85XmRCJmk KHVUU05Abg==', // base64
        '0x0000000000000000000000000000000000000000000000000000000000000000', // hex of 32 bytes
        '0x0000000000000000000000000000000000000000', // hex of 20 bytes
        '0000000000000000000000000000000000000000000000000000000000000000', // hex of 32 bytes no 0x prefix
        '0000000000000000000000000000000000000000', // hex of 20 bytes no 0x prefix
        'sui1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqg40c04', // bech32 of 20 bytes (incorrect length)
        'bc1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqaj76hn', // bech32 with 32 bytes with wrong hrp
        'sui1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqgzlz8e', // bech32m (wrong scheme) with 32 bytes
        ],
        isValidSuiAddress,
        false
      );
    });

    it('accepts string with sui prefix and of correct length', () => {
      expectAll(
        [
          'sui1hexrm8m3zre03hjl5t8psga34427ply4kz29dze62w8zrkjlt9esv4rnx2',
          'sui1mne690jmzjda8jj34cmsd6kju5vlct88azu3z8q5l2jf7yk9f24sdu9738',
          'sui1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqa70wzm'
        ],
        isValidSuiAddress,
        true
      );
    });
  });
});
