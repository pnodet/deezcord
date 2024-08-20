/* eslint-disable no-console */
import { createSocket } from 'node:dgram';
import { nanoid } from 'nanoid';
import { getSharedSecret, setSharedSecret } from '../db';
import {
  EAttributeType,
  EMessageType,
  EStunErrorCodes,
  StunError,
} from './types';
import {
  calculateMessageIntegrity,
  encodeAddress,
  encodeXorMappedAddress,
  getErrorResponse,
  getSharedSecretResponse,
  parseMessage,
  serializeMessage,
} from './messages';
import type { RemoteInfo, Socket } from 'node:dgram';
import type { TStunChangeRequest, TStunMessage } from './types';

export const environment = {
  STUN: {
    ALTERNATE_IP: '0.0.0.0',
    ALTERNATE_PORT: '3479',
  },
  APP: {
    APP_PORT_UDP4: process.env.APP_PORT_UDP4 ?? undefined,
    APP_PORT_UDP6: process.env.APP_PORT_UDP6 ?? undefined,
  },
};

export class StunServer {
  private server: Socket;

  constructor(private socketType: 'udp4' | 'udp6') {
    this.server = createSocket(socketType);
    this.registerHandler();
  }

  start(port: number) {
    this.server.bind(port, '0.0.0.0', () => {
      console.log(`[${this.socketType}] STUN server listening on port ${port}`);
    });
  }

  private registerHandler() {
    this.server.on('message', this.handleMessage.bind(this));
  }

  private sendResponse(buffer: Buffer, port: number, ip: string) {
    console.debug(`[${this.socketType}] [STUN] [SEND] [${ip}:${port}]`);
    this.server.send(Uint8Array.from(buffer), port, ip);
  }

  private handleMessage(message: Buffer, rinfo: RemoteInfo) {
    try {
      const stunMessage = parseMessage(message);

      console.debug(
        `[${this.socketType}] [STUN] [RECV] ${stunMessage.header.type}`,
      );

      switch (stunMessage.header.type) {
        case EMessageType.BINDING_REQUEST: {
          this.handleBindingRequest(stunMessage, rinfo);

          break;
        }

        case EMessageType.SHARED_SECRET_REQUEST: {
          this.handleSharedSecretRequest(stunMessage, rinfo);

          break;
        }

        default: {
          throw StunError.fromCode(EStunErrorCodes.INVALID_STUN_MESSAGE_TYPE);
        }
      }
    } catch (error) {
      console.error('Error processing STUN message:', error);

      if (StunError.isStunError(error)) {
        this.sendErrorResponse(
          rinfo,
          error.errorCodes.join(','),
          error.message,
        );
      } else {
        console.error('Unexpected error:', error);
      }
    }
  }

  private handleBindingRequest(message: TStunMessage, rinfo: RemoteInfo) {
    const transactionId = message.header.transactionId;
    let changeRequest: TStunChangeRequest | undefined;
    let username: string | undefined;
    let hasMessageIntegrity = false;

    for (const attribute of message.attributes) {
      switch (attribute.type) {
        case EAttributeType.ChangeRequest: {
          changeRequest = attribute as TStunChangeRequest;

          break;
        }

        case EAttributeType.Username: {
          username = attribute.value as string;

          break;
        }

        case EAttributeType.MessageIntegrity: {
          hasMessageIntegrity = true;

          break;
        }

        default: {
          throw new Error('Unknown attribute type or not implemented yet');
        }
      }
    }

    let sourceAddress = rinfo.address;
    let sourcePort = rinfo.port;

    if (changeRequest) {
      if (changeRequest.value.changeIp) {
        sourceAddress = environment.STUN.ALTERNATE_IP ?? sourceAddress;
      }

      if (changeRequest.value.changePort) {
        sourcePort = Number.parseInt(
          environment.STUN.ALTERNATE_PORT ?? sourcePort.toString(),
          10,
        );
      }
    }

    const response: TStunMessage = {
      header: {
        type: EMessageType.BINDING_RESPONSE,
        length: 0,
        transactionId,
      },
      attributes: [
        {
          type: EAttributeType.XorMappedAddress,
          length: 8,
          value: encodeXorMappedAddress(
            rinfo.address,
            rinfo.port,
            transactionId,
          ),
        },
        {
          type: EAttributeType.MappedAddress,
          length: 8,
          value: encodeAddress(rinfo.address, rinfo.port),
        },
        {
          type: EAttributeType.SourceAddress,
          length: 8,
          value: encodeAddress(sourceAddress, sourcePort),
        },
        {
          type: EAttributeType.ChangedAddress,
          length: 8,
          value: encodeAddress(
            environment.STUN.ALTERNATE_IP ?? '0.0.0.0',
            Number.parseInt(environment.STUN.ALTERNATE_PORT ?? '3479', 10),
          ),
        },
        {
          type: EAttributeType.Software,
          length: 21,
          value: Buffer.from('STUN server').toString('hex'),
        },
      ],
    };

    if (hasMessageIntegrity && username) {
      const sharedSecret = getSharedSecret(username);

      if (sharedSecret) {
        const messageIntegrity = calculateMessageIntegrity(
          response,
          sharedSecret,
        );

        response.attributes.push({
          type: EAttributeType.MessageIntegrity,
          length: 20,
          value: messageIntegrity,
        });
      }
    }

    const responseBuffer = serializeMessage(response);

    this.sendResponse(responseBuffer, rinfo.port, rinfo.address);
  }

  private handleSharedSecretRequest(message: TStunMessage, rinfo: RemoteInfo) {
    try {
      const sharedSecret = nanoid(32);
      const username = `user-${nanoid(10)}`;

      setSharedSecret(username, sharedSecret);

      const responseBuffer = getSharedSecretResponse(
        message,
        username,
        sharedSecret,
      );

      this.sendResponse(responseBuffer, rinfo.port, rinfo.address);
    } catch (error) {
      console.error('Error processing Shared Secret Request:', error);
      this.sendErrorResponse(
        rinfo,
        EStunErrorCodes.STUN_SERVER_ERROR,
        'Server Error',
      );
    }
  }

  private sendErrorResponse(rinfo: RemoteInfo, code: string, reason: string) {
    const responseBuffer = getErrorResponse(code, reason);

    this.sendResponse(responseBuffer, rinfo.port, rinfo.address);
  }
}
