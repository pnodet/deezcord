import type { WSContext } from 'hono/ws';
import { getUserById, createUser, setUserAlive } from '../../db';
import { sendAck } from '../ack';
import type { ClientCommand } from '../mod-client';

export const handleConnect = (
  ws: WSContext,
  cmd: ClientCommand<'Connect'>,
) => {
  const id = cmd.user_id;
  const username = cmd.command.Client.Connect;
  let user = getUserById(cmd.user_id);

  if (!user) {
    user = createUser(id, username);
  } else if (!user.alive) {
    user = setUserAlive(id, true);
  }

	sendAck(ws, id);
};
