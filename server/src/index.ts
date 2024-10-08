import { Hono } from 'hono';
import { createBunWebSocket } from 'hono/bun';
import { initStunServer } from './stun';
import { onMessage } from './commands';
import { clearRooms, setAllUsersDead } from './db';

initStunServer();

const { upgradeWebSocket, websocket } = createBunWebSocket();

const app = new Hono();

app.get(
  '/ws',
  upgradeWebSocket(_ctx => ({
    onMessage,
  })),
);

setAllUsersDead();
clearRooms();

const port = 3030;

console.log(`Listening on port ${port}`);

Bun.serve({
  fetch: app.fetch,
  websocket,
  port,
});
