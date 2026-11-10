import { setUserAlive, setUserRoom } from '../db';
import { handleConnect } from './handlers/connect';
import { handleCreateRoom } from './handlers/create-room';
import { handleJoin } from './handlers/join';
import { handleListRooms } from './handlers/list-rooms';
import { handleSendAnswer } from './handlers/send-answer';
import { handleSendIceCandidate } from './handlers/send-ice-candidate';
import { handleSendOffer } from './handlers/send-offer';
import { sendRoomListToMembers, sockets } from './send';
import type { ClientCommand, ClientCommandKeys } from './mod-client';
import type { WSContext, WSMessageReceive } from 'hono/ws';

// Per-connection state. One instance per socket, created in `src/index.ts`.
// `userId` is bound by `Connect`; after that the envelope's `user_id` is
// ignored, so a socket can only act as the user it connected as.
export type Session = { userId: string | null };

export const onMessage = (
  evt: MessageEvent<WSMessageReceive>,
  ws: WSContext,
  session: Session,
): void => {
  const commandMessage = JSON.parse(
    evt.data as string,
  ) as ClientCommand<ClientCommandKeys>;
  const command =
    typeof commandMessage.command.Client === 'string'
      ? commandMessage.command.Client
      : (Object.keys(commandMessage.command.Client)[0] as ClientCommandKeys);

  console.debug('\n\n*** Received ***', { commandMessage, command });

  if (command === 'Connect') {
    handleConnect(ws, session, commandMessage as ClientCommand<'Connect'>);

    return;
  }

  const { userId } = session;

  if (!userId) {
    console.warn(`Ignoring ${command}, socket has not sent Connect`);

    return;
  }

  switch (command) {
    case 'ListRooms': {
      handleListRooms(userId);

      break;
    }

    case 'CreateRoom': {
      handleCreateRoom(userId, commandMessage as ClientCommand<'CreateRoom'>);

      break;
    }

    case 'Join': {
      handleJoin(userId, commandMessage as ClientCommand<'Join'>);

      break;
    }

    case 'Leave': {
      setUserRoom(userId, null);
      sendRoomListToMembers(userId);

      break;
    }

    case 'SendOffer': {
      handleSendOffer(userId, commandMessage as ClientCommand<'SendOffer'>);

      break;
    }

    case 'SendAnswer': {
      handleSendAnswer(userId, commandMessage as ClientCommand<'SendAnswer'>);

      break;
    }

    case 'SendIceCandidate': {
      handleSendIceCandidate(
        userId,
        commandMessage as ClientCommand<'SendIceCandidate'>,
      );

      break;
    }

    default: {
      console.log('Unknown command');

      break;
    }
  }
};

export const onClose = (session: Session): void => {
  const { userId } = session;

  if (!userId) return;

  sockets.delete(userId);
  setUserAlive(userId, false);
  setUserRoom(userId, null);
  sendRoomListToMembers(userId);
};
