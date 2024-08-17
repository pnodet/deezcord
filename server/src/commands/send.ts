import type { ServerCommand, ServerCommandKeys } from './mod-server';
import type { WSContext } from 'hono/ws';

export const sendCommand =
  <K extends ServerCommandKeys>(command: ServerCommand<K>) =>
  (ws: WSContext) => {
    console.log('Sending', { command }, JSON.stringify(command));
    ws.send(JSON.stringify(command));
  };
