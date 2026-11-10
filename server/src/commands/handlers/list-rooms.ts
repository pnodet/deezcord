import { getRooms } from '../../db';
import { sendTo } from '../send';

export const handleListRooms = (userId: string) => {
  sendTo(userId, { RoomList: getRooms() });
};
