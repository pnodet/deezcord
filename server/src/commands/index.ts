import { handleConnect } from './handlers/connect';
import { handleCreateRoom } from './handlers/create-room';
import { handleListRooms } from './handlers/list-rooms';
import type { ClientCommand, ClientCommandKeys } from './mod-client';
import type { WSContext, WSMessageReceive } from 'hono/ws';

export const onMessage = (
  evt: MessageEvent<WSMessageReceive>,
  ws: WSContext,
): void => {
  const commandMessage = JSON.parse(
    evt.data as string,
  ) as ClientCommand<ClientCommandKeys>;
  const command =
	typeof commandMessage.command.Client === 'string' ?
		commandMessage.command.Client : Object.keys(
    commandMessage.command.Client,
  )[0] as ClientCommandKeys;

	console.debug('\n\n*** Received ***',{ commandMessage, command });

  switch (command) {
    case 'Connect': {
      handleConnect(ws, commandMessage as ClientCommand<'Connect'>);

      break;
    }

		case 'ListRooms':{
			handleListRooms(ws, commandMessage as ClientCommand<'ListRooms'>);

			break;
		}

		case 'CreateRoom':{
			handleCreateRoom(ws, commandMessage as ClientCommand<'CreateRoom'>);

			break;
		}

    default: {
      console.log('Unknown command');

      break;
    }
  }
};
