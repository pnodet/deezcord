import type { WSContext } from 'hono/ws';
import { getRooms } from '../../db';
import type { ClientCommand } from '../mod-client';
import { sendCommand } from '../send';

export const handleListRooms = (
  ws: WSContext,
  cmd: ClientCommand<'ListRooms'>,
) => {
	const rooms = getRooms();

	console.debug({ rooms });

	sendCommand({
		user_id: cmd.user_id,
		command: { Server: { RoomList: rooms } },
	})(ws)
};
