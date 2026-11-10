import { createRoom, getRooms, setUserRoom } from '../../db';
import { sendRoomListToMembers } from '../send';
import type { ClientCommand } from '../mod-client';

export const handleCreateRoom = (
  userId: string,
  cmd: ClientCommand<'CreateRoom'>,
) => {
  const roomname = cmd.command.Client.CreateRoom;
  const room =
    getRooms().find(room => room.name === roomname) ?? createRoom(roomname);

  setUserRoom(userId, room.id);
  sendRoomListToMembers(userId);
};
