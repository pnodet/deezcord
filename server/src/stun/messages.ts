import { createHmac, randomBytes } from 'node:crypto';
import { safeParse } from 'valibot';
import {
  EAttributeType,
  EMessageType,
  EStunErrorCodes,
  StunError,
  VStunMessage,
} from './types';
import type {
  TStunAddress,
  TStunAttribute,
  TStunChangeRequest,
  TStunErrorCode,
  TStunHeader,
  TStunMessage,
} from './types';

export const calculateMessageLength = (message: TStunMessage): number =>
  message.attributes.reduce((length, attr) => length + 4 + attr.length, 0);

export const encodeAddressAttribute = (value: TStunAddress): Buffer => {
  const buff = Buffer.alloc(8);

  buff.writeUInt8(0, 0); // Reserved
  buff.writeUInt8(value.family, 1);
  buff.writeUInt16BE(value.port, 2);

  for (const [index, octet] of value.address.split('.').entries()) {
    buff.writeUInt8(Number.parseInt(octet, 10), 4 + index);
  }

  return buff;
};

export const encodeErrorCodeAttribute = (
  value: TStunErrorCode['value'],
): Buffer => {
  const reasonBuff = Buffer.from(value.reason);
  const buff = Buffer.alloc(4 + reasonBuff.length);

  buff.writeUInt16BE(0, 0); // Reserved
  buff.writeUInt8(value.errorClass, 2);
  buff.writeUInt8(value.number, 3);
  reasonBuff.copy(new Uint8Array(buff), 4);

  return buff;
};

export const encodeChangeRequestAttribute = (
  value: TStunChangeRequest['value'],
): Buffer => {
  const buff = Buffer.alloc(4);
  let flags = 0;

  if (value.changeIp) flags |= 0x04;

  if (value.changePort) flags |= 0x02;
  buff.writeUInt32BE(flags);

  return buff;
};

export const serializeMessage = (message: TStunMessage): Buffer => {
  let attributesLength = 0;
  const attributeBuffers: Buffer[] = [];

  for (const attr of message.attributes) {
    let attrBuff: Buffer;

    if (typeof attr.value === 'string') {
      attrBuff = Buffer.from(attr.value, 'hex');
    } else if (typeof attr.value === 'number') {
      attrBuff = Buffer.alloc(4);
      attrBuff.writeUInt32BE(attr.value);
    } else if (Array.isArray(attr.value)) {
      attrBuff = Buffer.from(attr.value);
    } else if (typeof attr.value === 'object') {
      switch (attr.type) {
        case EAttributeType.MappedAddress: {
          attrBuff = encodeAddressAttribute(attr.value as TStunAddress);

          break;
        }

        case EAttributeType.XorMappedAddress: {
          attrBuff = encodeAddressAttribute(attr.value as TStunAddress);

          break;
        }

        case EAttributeType.ErrorCode: {
          attrBuff = encodeErrorCodeAttribute(
            attr.value as TStunErrorCode['value'],
          );

          break;
        }

        case EAttributeType.ChangeRequest: {
          attrBuff = encodeChangeRequestAttribute(
            attr.value as TStunChangeRequest['value'],
          );

          break;
        }

        default: {
          throw new Error(`Unsupported attribute type: ${attr.type}`);
        }
      }
    } else {
      throw new TypeError(
        `Unsupported attribute value type for attribute: ${attr.type}`,
      );
    }

    const paddedLength = Math.ceil(attrBuff.length / 4) * 4;
    const paddedBuff = Buffer.alloc(paddedLength);

    attrBuff.copy(paddedBuff);

    const attrHeader = Buffer.alloc(4);

    attrHeader.writeUInt16BE(attr.type, 0);
    attrHeader.writeUInt16BE(attrBuff.length, 2);

    attributeBuffers.push(attrHeader, paddedBuff);
    attributesLength += 4 + paddedLength;
  }

  const header = Buffer.alloc(20);

  header.writeUInt16BE(message.header.type, 0);
  header.writeUInt16BE(attributesLength, 2);
  Buffer.from(message.header.transactionId, 'hex').copy(header, 4);

  return Buffer.concat([header, ...attributeBuffers]);
};

export const encodeAddress = (address: string, port: number): string => {
  const buff = Buffer.alloc(8);
  const parts = address.split('.');

  buff.writeUInt8(0, 0); // Reserved
  buff.writeUInt8(1, 1); // Family (IPv4)
  buff.writeUInt16BE(port, 2);

  for (let i = 0; i < 4; i++) {
    buff.writeUInt8(Number.parseInt(parts[i], 10), 4 + i);
  }

  return buff.toString('hex');
};

export const encodeXorMappedAddress = (
  address: string,
  port: number,
  _transactionId: string,
): string => {
  const buff = Buffer.alloc(8);
  const parts = address.split('.');

  buff.writeUInt8(0, 0); // Reserved
  buff.writeUInt8(1, 1); // Family (IPv4)
  buff.writeUInt16BE(port ^ 0x21_12, 2); // XOR the port with the first 16 bits of the magic cookie

  const magicCookie = Buffer.from('2112A442', 'hex');

  for (let i = 0; i < 4; i++) {
    buff.writeUInt8(Number.parseInt(parts[i], 10) ^ magicCookie[i], 4 + i);
  }

  return buff.toString('hex');
};

export const parseChangeRequest = (
  type: number,
  length: number,
  value: Buffer,
): TStunChangeRequest => {
  return {
    type,
    length: length as 4,
    value: {
      changeIp: (value.readUInt32BE(0) & 0x04) !== 0,
      changePort: (value.readUInt32BE(0) & 0x02) !== 0,
    },
  };
};

export const parseAttribute = (
  type: number,
  length: number,
  value: Buffer,
): TStunAttribute => {
  switch (type) {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-enum-comparison
    case EAttributeType.ChangeRequest: {
      return parseChangeRequest(type, length, value);
    }

    // Add cases for other attribute types
    default: {
      return {
        type,
        length,
        value: value.toString('hex'),
      };
    }
  }
};

export const parseMessage = (message: Buffer): TStunMessage => {
  if (message.length < 20) {
    throw StunError.fromCode(EStunErrorCodes.INVALID_STUN_MESSAGE_SHORT);
  }

  const header: TStunHeader = {
    type: message.readUInt16BE(0),
    length: message.readUInt16BE(2),
    transactionId: message.subarray(4, 20).toString('hex'),
  };

  if (header.length !== message.length - 20) {
    throw StunError.fromCode(EStunErrorCodes.INVALID_STUN_MESSAGE_LENGTH);
  }

  const attributes: TStunAttribute[] = [];
  let offset = 20;

  while (offset < message.length) {
    if (offset + 4 > message.length) {
      throw new Error('Invalid STUN message: incomplete attribute header');
    }

    const type = message.readUInt16BE(offset);
    const length = message.readUInt16BE(offset + 2);
    const value = message.subarray(offset + 4, offset + 4 + length);

    if (offset + 4 + length > message.length) {
      throw new Error(
        'Invalid STUN message: attribute length exceeds message boundary',
      );
    }

    attributes.push(parseAttribute(type, length, value));

    offset += 4 + length;
    offset += (4 - (offset % 4)) % 4; // Padding to align to 4-byte boundary
  }

  const stunMessage: TStunMessage = {
    header,
    attributes,
  };

  // Validate the parsed STUN message against the VStunMessage schema
  const validationResult = safeParse(VStunMessage, stunMessage);

  if (!validationResult.success) {
    throw StunError.fromCode(EStunErrorCodes.INVALID_STUN_MESSAGE_VALIDATION);
  }

  return stunMessage;
};

export const calculateMessageIntegrity = (
  message: TStunMessage,
  key: string,
): string => {
  const integrityCopy = structuredClone(message);

  // Remove any existing MESSAGE-INTEGRITY attribute
  integrityCopy.attributes = integrityCopy.attributes.filter(
    (attr: TStunAttribute) => attr.type !== EAttributeType.MessageIntegrity,
  );

  integrityCopy.header.length = calculateMessageLength(integrityCopy) + 24;
  const serializedMessage = serializeMessage(integrityCopy);
  const hmac = createHmac('sha1', key);

  hmac.update(new Uint8Array(serializedMessage));

  return hmac.digest('hex');
};

export const getErrorResponse = (code: string, reason: string) => {
  const errorCode = Number.parseInt(code.split('_').pop() ?? '500', 10);
  const errorClass = Math.floor(errorCode / 100);
  const errorNumber = errorCode % 100;
  const errorBuffer = new Uint8Array(
    Buffer.from([0, 0, errorClass, errorNumber]),
  );
  const reasonBuffer = new Uint8Array(Buffer.from(reason));
  const errorMessage: TStunMessage = {
    header: {
      type: EMessageType.BINDING_ERROR_RESPONSE,
      length: 0,
      transactionId: randomBytes(16).toString('hex'),
    },
    attributes: [
      {
        type: EAttributeType.ErrorCode,
        length: 4 + reason.length,
        value: Buffer.from(
          new Uint8Array([...errorBuffer, ...reasonBuffer]),
        ).toString('hex'),
      },
    ],
  };

  const responseBuffer = serializeMessage(errorMessage);

  return responseBuffer;
};

export const getSharedSecretResponse = (
  message: TStunMessage,
  username: string,
  sharedSecret: string,
) => {
  const response: TStunMessage = {
    header: {
      type: EMessageType.SHARED_SECRET_RESPONSE,
      length: 0,
      transactionId: message.header.transactionId,
    },
    attributes: [
      {
        type: EAttributeType.Username,
        length: username.length,
        value: Buffer.from(username).toString('hex'),
      },
      {
        type: EAttributeType.Password,
        length: sharedSecret.length,
        value: Buffer.from(sharedSecret).toString('hex'),
      },
    ],
  };

  const responseBuffer = serializeMessage(response);

  return responseBuffer;
};
