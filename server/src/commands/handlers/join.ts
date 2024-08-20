import { addUserToRoom, getRoomById, getRooms } from '../../db';
import { prepareCommand } from '../send';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from '../mod-client';
import type { ServerWebSocket } from 'bun';

export const handleJoin = (ws: WSContext, cmd: ClientCommand<'Join'>) => {
  const userId = cmd.user_id;
  const roomId = cmd.command.Client.Join;
  let room = getRoomById(roomId);

  if (!room) {
    console.error('Room not found');

    return;
  }

  room = addUserToRoom(roomId, userId);

  if (!room) {
    throw new Error('Room not found');
  }

  const rawWs = ws.raw as ServerWebSocket;

  rawWs.subscribe(room.id);

  let cache = JSON.stringify([]);

  setInterval(() => {
    const rooms = getRooms();
    const str = JSON.stringify(rooms);

    if (str !== cache) {
      cache = str;
      ws.send(
        prepareCommand({
          user_id: userId,
          command: { Server: { RoomList: rooms } },
        }),
      );
    }
  }, 1000);
};
