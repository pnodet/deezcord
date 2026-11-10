import { expect, test } from 'bun:test';
import { EMessageType } from './types';
import { parseMessage, serializeMessage } from './messages';

test('a plain binding request survives parse then serialize', () => {
  // 20-byte header, no attributes: type 0x0001, length 0, then 16 bytes the
  // parser treats as the transaction id (the RFC 5389 magic cookie 2112a442
  // followed by 12 random bytes).
  const request = Buffer.from(
    '000100002112a442000102030405060708090a0b',
    'hex',
  );

  const parsed = parseMessage(request);

  expect(parsed).toEqual({
    header: {
      type: EMessageType.BINDING_REQUEST,
      length: 0,
      transactionId: '2112a442000102030405060708090a0b',
    },
    attributes: [],
  });
  expect(serializeMessage(parsed).equals(request)).toBe(true);
});
