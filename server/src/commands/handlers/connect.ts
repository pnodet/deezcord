import { createUser, getUserById, setUserAlive } from '../../db';
import { sendAck } from '../ack';
import { sockets } from '../send';
import type { WSContext } from 'hono/ws';
import type { Session } from '..';
import type { ClientCommand } from '../mod-client';

export const handleConnect = (
  ws: WSContext,
  session: Session,
  cmd: ClientCommand<'Connect'>,
) => {
  const userId = cmd.user_id;

  if (session.userId) {
    console.warn(`Ignoring Connect, socket is already ${session.userId}`);

    return;
  }

  if (sockets.has(userId)) {
    console.warn(`Rejecting Connect, ${userId} is connected on another socket`);
    ws.close(1008, 'User already connected');

    return;
  }

  session.userId = userId;
  sockets.set(userId, ws);

  const user = getUserById(userId);

  if (!user) {
    createUser(userId, cmd.command.Client.Connect);
  } else if (!user.alive) {
    setUserAlive(userId, true);
  }

  sendAck(ws, userId);
};
