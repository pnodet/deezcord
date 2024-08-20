import { addUserToRoom, createRoom, getRooms } from '../../db';
import { prepareCommand } from '../send';
import type { Room } from '../../db';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from '../mod-client';
import type { ServerWebSocket } from 'bun';

export const handleCreateRoom = (
  ws: WSContext,
  cmd: ClientCommand<'CreateRoom'>,
) => {
  const userId = cmd.user_id;
  const roomname = cmd.command.Client.CreateRoom;
  let existingRoom: Room | undefined | null = getRooms().find(
    room => room.name === roomname,
  );

  if (existingRoom) {
    existingRoom = addUserToRoom(existingRoom.id, userId);
  } else {
    existingRoom = createRoom(userId, roomname);
  }

  if (!existingRoom) {
    throw new Error('Room not found and could not be created');
  }

  const rawWs = ws.raw as ServerWebSocket;

  rawWs.subscribe(existingRoom.id);

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
