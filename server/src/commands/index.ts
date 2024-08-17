import { sendAck } from './ack';
import { handleConnect } from './connect';
import type { ClientCommand, ClientCommandKeys } from './mod-client';
import type { WSContext, WSMessageReceive } from 'hono/ws';

export const onMessage = (
  evt: MessageEvent<WSMessageReceive>,
  ws: WSContext,
): void => {
  const commandMessage = JSON.parse(
    evt.data as string,
  ) as ClientCommand<ClientCommandKeys>;
  const command = Object.keys(
    commandMessage.command.Client,
  )[0] as ClientCommandKeys;

  const id = commandMessage.user_id;

  switch (command) {
    case 'Connect': {
      handleConnect(ws, commandMessage as ClientCommand<'Connect'>);
      sendAck(ws, id);

      break;
    }

    default: {
      console.log('Unknown command');

      break;
    }
  }
};
