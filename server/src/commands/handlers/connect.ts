import { createUser, getUserById, setUserAlive } from '../../db';
import { sendAck } from '../ack';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from '../mod-client';

export const handleConnect = (ws: WSContext, cmd: ClientCommand<'Connect'>) => {
  const userId = cmd.user_id;
  let user = getUserById(userId);

  if (!user) {
    const username = cmd.command.Client.Connect;

    user = createUser(userId, username);
  } else if (!user.alive) {
    user = setUserAlive(userId, true);
  }

  sendAck(ws, userId);
};
