import {
  array,
  boolean,
  integer,
  ip,
  length,
  literal,
  maxLength,
  maxValue,
  minLength,
  minValue,
  multipleOf,
  number,
  object,
  pipe,
  regex,
  string,
  union,
} from 'valibot';
import { EAttributeType, VStunMessageType } from './enums';
import type { InferOutput } from 'valibot';

export const VStunHeader = object({
  type: VStunMessageType,
  length: pipe(number(), integer(), minValue(0), maxValue(65_535)),
  transactionId: pipe(string(), length(32)),
});

export const VStunAddress = object({
  family: union([literal(1), literal(2)]),
  port: pipe(number(), integer(), minValue(0), maxValue(65_535)),
  address: pipe(string(), ip()),
});

export const VStunChangeRequest = object({
  type: literal(EAttributeType.ChangeRequest),
  length: literal(4),
  value: object({
    changeIp: boolean(),
    changePort: boolean(),
  }),
});

export const VStunXorMappedAddress = object({
  type: literal(EAttributeType.XorMappedAddress),
  length: literal(8),
  value: object({
    family: union([literal(1), literal(2)]),
    xorPort: pipe(number(), integer(), minValue(0), maxValue(65_535)),
    xorAddress: pipe(string(), ip()),
  }),
});

const createAddressAttribute = (type: EAttributeType) =>
  object({
    type: literal(type),
    length: literal(8),
    value: object({
      family: union([literal(1), literal(2)]),
      port: pipe(number(), integer(), minValue(0), maxValue(65_535)),
      address: pipe(string(), ip()),
    }),
  });

const createStringAttribute = (type: EAttributeType, max: number) =>
  object({
    type: literal(type),
    length: pipe(number(), integer(), minValue(0), maxValue(max)),
    value: pipe(string(), minLength(1), maxLength(max)),
  });

export const VStunMappedAddress = createAddressAttribute(
  EAttributeType.MappedAddress,
);
export const VStunSourceAddress = createAddressAttribute(
  EAttributeType.SourceAddress,
);
export const VStunChangedAddress = createAddressAttribute(
  EAttributeType.ChangedAddress,
);
export const VStunReflectedFrom = createAddressAttribute(
  EAttributeType.ReflectedFrom,
);
export const VStunAlternateServer = createAddressAttribute(
  EAttributeType.AlternateServer,
);

export const VStunUsername = createStringAttribute(
  EAttributeType.Username,
  513,
);
export const VStunPassword = createStringAttribute(
  EAttributeType.Password,
  763,
);
export const VStunRealm = createStringAttribute(EAttributeType.Realm, 763);
export const VStunNonce = createStringAttribute(EAttributeType.Nonce, 763);
export const VStunSoftware = createStringAttribute(
  EAttributeType.Software,
  763,
);

export const VStunMessageIntegrity = object({
  type: literal(EAttributeType.MessageIntegrity),
  length: literal(20),
  value: pipe(string(), length(40), regex(/^[\da-f]{40}$/i)),
});

export const VStunErrorCode = object({
  type: literal(EAttributeType.ErrorCode),
  length: pipe(number(), integer(), minValue(4)),
  value: object({
    errorClass: pipe(number(), integer(), minValue(3), maxValue(6)),
    number: pipe(number(), integer(), minValue(0), maxValue(99)),
    reason: string(),
  }),
});

export const VStunUnknownAttributes = object({
  type: literal(EAttributeType.UnknownAttributes),
  length: pipe(number(), integer(), multipleOf(2)),
  value: array(pipe(number(), integer(), minValue(0), maxValue(0xff_ff))),
});

export const VStunFingerprint = object({
  type: literal(EAttributeType.Fingerprint),
  length: literal(4),
  value: pipe(number(), integer(), minValue(0), maxValue(0xff_ff_ff_ff)),
});

export const VStunAttribute = union([
  VStunMappedAddress,
  VStunChangeRequest,
  VStunXorMappedAddress,
  VStunSourceAddress,
  VStunChangedAddress,
  VStunUsername,
  VStunPassword,
  VStunMessageIntegrity,
  VStunErrorCode,
  VStunUnknownAttributes,
  VStunReflectedFrom,
  VStunRealm,
  VStunNonce,
  VStunSoftware,
  VStunAlternateServer,
  VStunFingerprint,
]);

export const VStunMessage = object({
  header: VStunHeader,
  attributes: array(VStunAttribute),
});

export type TStunMessage = InferOutput<typeof VStunMessage>;
export type TStunHeader = InferOutput<typeof VStunHeader>;
export type TStunAttribute = InferOutput<typeof VStunAttribute>;
export type TStunAddress = InferOutput<typeof VStunAddress>;
export type TStunMappedAddress = InferOutput<typeof VStunMappedAddress>;
export type TStunChangeRequest = InferOutput<typeof VStunChangeRequest>;
export type TStunXorMappedAddress = InferOutput<typeof VStunXorMappedAddress>;
export type TStunSourceAddress = InferOutput<typeof VStunSourceAddress>;
export type TStunChangedAddress = InferOutput<typeof VStunChangedAddress>;
export type TStunUsername = InferOutput<typeof VStunUsername>;
export type TStunPassword = InferOutput<typeof VStunPassword>;
export type TStunMessageIntegrity = InferOutput<typeof VStunMessageIntegrity>;
export type TStunErrorCode = InferOutput<typeof VStunErrorCode>;
export type TStunUnknownAttributes = InferOutput<typeof VStunUnknownAttributes>;
export type TStunReflectedFrom = InferOutput<typeof VStunReflectedFrom>;
export type TStunRealm = InferOutput<typeof VStunRealm>;
export type TStunNonce = InferOutput<typeof VStunNonce>;
export type TStunSoftware = InferOutput<typeof VStunSoftware>;
export type TStunAlternateServer = InferOutput<typeof VStunAlternateServer>;
export type TStunFingerprint = InferOutput<typeof VStunFingerprint>;

export type TStunAttributeValue = TStunAttribute['value'];
