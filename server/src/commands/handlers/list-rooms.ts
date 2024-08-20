import { getRooms } from '../../db';
import { prepareCommand } from '../send';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from '../mod-client';

export const handleListRooms = (
  ws: WSContext,
  cmd: ClientCommand<'ListRooms'>,
) => {
  const rooms = getRooms();

  ws.send(
    prepareCommand({
      user_id: cmd.user_id,
      command: {
        Server: { RoomList: rooms.filter(room => room.users.length > 0) },
      },
    }),
  );
};
