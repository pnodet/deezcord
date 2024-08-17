import { sendCommand } from './send';
import type { WSContext } from 'hono/ws';

export const sendAck = (ws: WSContext, id: string) => {
  sendCommand({
    user_id: id,
    command: { Server: { Ack: null } },
  })(ws);
};
