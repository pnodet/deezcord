import { setUserAlive, setUserRoom } from '../db';
import { handleConnect } from './handlers/connect';
import { handleCreateRoom } from './handlers/create-room';
import { handleJoin } from './handlers/join';
import { handleListRooms } from './handlers/list-rooms';
import { handleSendAnswer } from './handlers/send-answer';
import { handleSendIceCandidate } from './handlers/send-ice-candidate';
import { handleSendOffer } from './handlers/send-offer';
import { VClientMessage } from './mod-client';
import { prepareCommand, sendRoomListToMembers, sockets } from './send';
import { safeParse } from 'valibot';
import type { ClientCommand, ClientCommandKeys } from './mod-client';
import type { WSContext, WSMessageReceive } from 'hono/ws';

// Per-connection state. One instance per socket, created in `src/index.ts`.
// `userId` is bound by `Connect`; after that the envelope's `user_id` is
// ignored, so a socket can only act as the user it connected as.
export type Session = { userId: string | null };

// Replies on `ws` only. The message is shown to the user: never put error
// details, stack traces or SQL in it.
const sendError = (ws: WSContext, userId: string, message: string) => {
  ws.send(
    prepareCommand({
      user_id: userId,
      command: { Server: { Error: message } },
    }),
  );
};

const parseMessage = (data: WSMessageReceive) => {
  try {
    const result = safeParse(VClientMessage, JSON.parse(String(data)));

    return result.success ? result.output : null;
  } catch {
    return null;
  }
};

const dispatch = (
  commandMessage: ClientCommand<ClientCommandKeys>,
  ws: WSContext,
  session: Session,
) => {
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
    sendError(ws, commandMessage.user_id, 'Send Connect first');

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

export const onMessage = (
  evt: MessageEvent<WSMessageReceive>,
  ws: WSContext,
  session: Session,
): void => {
  const commandMessage = parseMessage(evt.data);

  if (!commandMessage) {
    console.warn('Rejected invalid message', evt.data);
    sendError(ws, session.userId ?? '', 'Invalid message');

    return;
  }

  try {
    dispatch(commandMessage as ClientCommand<ClientCommandKeys>, ws, session);
  } catch (error) {
    console.error('Command failed', error);
    sendError(
      ws,
      session.userId ?? commandMessage.user_id,
      'Internal server error',
    );
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
