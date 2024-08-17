import type { WSContext } from 'hono/ws';
import { createRoom, getRooms,  } from '../../db';
import type { ClientCommand } from '../mod-client';
import { sendCommand } from '../send';

export const handleCreateRoom = (
  ws: WSContext,
  cmd: ClientCommand<'CreateRoom'>,
) => {
	const hostId = cmd.user_id;
  const roomname = cmd.command.Client.CreateRoom;
	const rooms = getRooms();

	if(rooms.find(room => room.name === roomname)){
		sendCommand({
			user_id: cmd.user_id,
			command: { Server: { RoomList: rooms } },
		})(ws)

		return;
	}

	const room = createRoom(hostId, roomname)!;
	sendCommand({
		user_id: cmd.user_id,
		command: { Server: { RoomList: [...rooms, room] } },
	})(ws)
};
