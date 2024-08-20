import { prepareCommand } from '../send';
import type { WSContext } from 'hono/ws';
import type { ClientCommand } from '../mod-client';
import type { ServerWebSocket } from 'bun';

export const handleSendOffer = (
  ws: WSContext,
  cmd: ClientCommand<'SendOffer'>,
) => {
  const data = cmd.command.Client.SendOffer;
  const roomId = data[1];

  const rws = ws.raw as ServerWebSocket;

  rws.subscribe(roomId);

  const res = rws.publish(
    roomId,
    prepareCommand({
      user_id: data[0],
      command: {
        Server: {
          IncomingOffer: [cmd.user_id, roomId, data[2]],
        },
      },
    }),
  );

  if (res === 0) {
    throw new Error('Failed to publish offer');
  }
};
