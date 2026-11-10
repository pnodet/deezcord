import { getRooms } from '../db';
import type { WSContext } from 'hono/ws';
import type {
  ServerCommand,
  ServerCommandData,
  ServerCommandKeys,
} from './mod-server';

export const prepareCommand = <K extends ServerCommandKeys>(
  command: ServerCommand<K>,
): string => {
  console.log('\n\n*** Sending ***', { command });

  return JSON.stringify(command);
};

// The socket of every connected user, keyed by the user id bound on `Connect`.
export const sockets = new Map<string, WSContext>();

// The client drops any envelope whose `user_id` is not its own, so every
// message is addressed to exactly one recipient.
export const sendTo = <K extends ServerCommandKeys>(
  userId: string,
  server: { [P in K]: ServerCommandData[K] },
): boolean => {
  const ws = sockets.get(userId);

  if (!ws) {
    console.warn(`Not sent, user ${userId} is not connected`);

    return false;
  }

  ws.send(prepareCommand({ user_id: userId, command: { Server: server } }));

  return true;
};

// Called after every membership change. Sends the current RoomList to every
// user who is in a room, plus `actorId` (the user who caused the change).
// Users in the lobby are skipped on purpose: the client blocks on keyboard
// input while it shows the lobby, so a list queued during that time would be
// replayed, stale, after the user picks a room.
export const sendRoomListToMembers = (actorId: string) => {
  const rooms = getRooms();
  const recipients = new Set([actorId, ...rooms.flatMap(room => room.users)]);

  for (const userId of recipients) {
    sendTo(userId, { RoomList: rooms });
  }
};
