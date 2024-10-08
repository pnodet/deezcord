export type ValueOf<T> = T[keyof T];

export const EStunErrorCodes = {
  INVALID_STUN_MESSAGE_TYPE: 'STUN_00001',
  INVALID_STUN_MESSAGE_SHORT: 'STUN_00002',
  INVALID_STUN_MESSAGE_LENGTH: 'STUN_00003',
  INVALID_STUN_MESSAGE_VALIDATION: 'STUN_00004',
  STUN_SERVER_ERROR: 'STUN_00005',
  BAD_REQUEST: 'STUN_00006',
  UNAUTHORIZED: 'STUN_00007',
  UNKNOWN_ATTRIBUTE: 'STUN_00008',
  STALE_NONCE: 'STUN_00009',
  SERVER_ERROR: 'STUN_00010',
} as const;

const stunErrorMessage = {
  STUN_00001: 'Invalid/Unsupported STUN message type',
  STUN_00002: 'Invalid STUN message: message too short',
  STUN_00003: 'Invalid STUN message: invalid message length',
  STUN_00004: 'Invalid STUN message: validation failed',
  STUN_00005: 'STUN: Server Error',
  STUN_00006: 'STUN: Bad Request',
  STUN_00007: 'STUN: Unauthorized',
  STUN_00008: 'STUN: Unknown Attribute',
  STUN_00009: 'STUN: Stale Nonce',
  STUN_00010: 'STUN: Server Error',
} as const satisfies { [key in TStunErrorCodes]: string };

export type TStunErrorCodes = ValueOf<typeof EStunErrorCodes>;

export class StunError extends Error {
  stunError = true;

  constructor(
    message: string,
    private codes: TStunErrorCodes[],
  ) {
    super(message);
  }

  get errorCodes() {
    return this.codes;
  }

  static fromCode(code: TStunErrorCodes, ...codes: TStunErrorCodes[]) {
    codes.push(code);

    return new StunError(stunErrorMessage[code], codes);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  static isStunError(value: any): value is StunError {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    return !!(typeof value === 'object' && !!value && value.stunError === true);
  }

  toString() {
    return `STUN_ERROR: ${this.message} [${this.codes.join(',')}]`;
  }
}
