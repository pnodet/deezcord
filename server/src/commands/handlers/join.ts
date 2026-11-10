import { getRoomById, setUserRoom } from '../../db';
import { sendRoomListToMembers, sendTo } from '../send';
import type { ClientCommand } from '../mod-client';

export const handleJoin = (userId: string, cmd: ClientCommand<'Join'>) => {
  const roomId = cmd.command.Client.Join;

  if (!getRoomById(roomId)) {
    sendTo(userId, { Error: 'Room not found' });

    return;
  }

  setUserRoom(userId, roomId);
  sendRoomListToMembers(userId);
};
