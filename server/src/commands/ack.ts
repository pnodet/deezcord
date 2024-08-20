import { prepareCommand } from './send';
import type { WSContext } from 'hono/ws';

export const sendAck = (ws: WSContext, id: string) => {
  ws.send(
    prepareCommand({
      user_id: id,
      command: { Server: { Ack: null } },
    }),
  );
};
