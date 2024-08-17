import { enum_ } from 'valibot';

export enum EMessageType {
  BINDING_REQUEST = 0x00_01,
  BINDING_RESPONSE = 0x01_01,
  BINDING_ERROR_RESPONSE = 0x01_11,
  SHARED_SECRET_REQUEST = 0x00_02,
  SHARED_SECRET_RESPONSE = 0x01_02,
  SHARED_SECRET_ERROR_RESPONSE = 0x01_12,
}

export const VStunMessageType = enum_(EMessageType);

export enum EAttributeType {
  MappedAddress = 0x00_01,
  ResponseAddress = 0x00_02,
  ChangeRequest = 0x00_03,
  SourceAddress = 0x00_04,
  ChangedAddress = 0x00_05,
  Username = 0x00_06,
  Password = 0x00_07,
  MessageIntegrity = 0x00_08,
  ErrorCode = 0x00_09,
  UnknownAttributes = 0x00_0a,
  ReflectedFrom = 0x00_0b,
  Realm = 0x00_14,
  Nonce = 0x00_15,
  XorMappedAddress = 0x00_20,
  Software = 0x80_22,
  AlternateServer = 0x80_23,
  Fingerprint = 0x80_28,
}

export const VStunAttributeType = enum_(EAttributeType);
