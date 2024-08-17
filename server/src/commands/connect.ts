import { createUser, getUserById, setUserAlive } from '../db';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from './mod-client';

export const handleConnect = (
  _ws: WSContext,
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

  console.debug({ user });
};
