import { Database } from 'bun:sqlite';
import { v7 as uuid } from 'uuid';

const db = new Database(process.env.DEEZCORD_DB ?? 'deezcord.sqlite', {
  create: true,
  strict: true,
});

db.run('PRAGMA journal_mode = WAL;');

export class User {
  id!: string;
  username!: string;
  alive!: boolean;
  room_id!: string | null;
}

export type Room = {
  id: string;
  name: string;
  users: string[];
};

db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    alive BOOLEAN DEFAULT TRUE,
    room_id TEXT
  )
`);

// Rooms are ephemeral: nobody is in a room after a restart. The table is
// dropped and recreated on every start instead of being migrated, which also
// removes the old `users_ids` column. Membership lives in `users.room_id`.
db.run('DROP TABLE IF EXISTS rooms');
db.run(`
  CREATE TABLE rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
  )
`);

db.run(`
  CREATE TABLE IF NOT EXISTS shared_secrets (
    user_id TEXT PRIMARY KEY,
    secret TEXT NOT NULL
  )
`);

const createUserQuery = db
  .query(
    `
  INSERT INTO users (id, username) VALUES ($id, $username)
`,
  )
  .as(User);

export const createUser = (id: string, username: string) =>
  createUserQuery.get({ id, username });

const getUsersQuery = db
  .query(
    `
  SELECT
    id,
    username,
    alive,
    room_id
  FROM users
`,
  )
  .as(User);

export const getUsers = () => getUsersQuery.all();

const getUserByIdQuery = db
  .query(
    `
  SELECT
    id,
    username,
    alive,
    room_id
  FROM users
  WHERE id = $id
`,
  )
  .as(User);

export const getUserById = (id: string) => getUserByIdQuery.get({ id });

const setUserAliveQuery = db
  .query(
    `
  UPDATE users SET alive = $alive WHERE id = $id RETURNING *
`,
  )
  .as(User);

const setAllUsersDeadQuery = db.query(
  `
  UPDATE users SET alive = false, room_id = null
`,
);

export const setAllUsersDead = () => {
  setAllUsersDeadQuery.run();
};

export const setUserAlive = (id: string, alive: boolean) =>
  setUserAliveQuery.get({ alive, id });

const createRoomQuery = db.query<
  { id: string; name: string },
  [string, string]
>('INSERT INTO rooms (id, name) VALUES (?, ?) RETURNING id, name');

export const createRoom = (name: string): Room => ({
  ...createRoomQuery.get(uuid(), name)!,
  users: [],
});

const getRoomsQuery = db.query<
  { id: string; name: string; users: string },
  []
>(`
  SELECT
    id,
    name,
    (SELECT json_group_array(users.id) FROM users WHERE users.room_id = rooms.id) AS users
  FROM rooms
`);

export const getRooms = (): Room[] =>
  getRoomsQuery.all().map(room => ({ ...room, users: JSON.parse(room.users) }));

export const getRoomById = (id: string): Room | undefined =>
  getRooms().find(room => room.id === id);

const setUserRoomQuery = db.query(
  'UPDATE users SET room_id = $roomId WHERE id = $userId',
);

const deleteRoomIfEmptyQuery = db.query(`
  DELETE FROM rooms
  WHERE id = $id AND NOT EXISTS (SELECT 1 FROM users WHERE room_id = $id)
`);

// A user is in at most one room; `roomId = null` means "leave". The room the
// user was in before is deleted if nobody is left in it.
export const setUserRoom = (userId: string, roomId: string | null) => {
  const previousRoomId = getUserById(userId)?.room_id;

  setUserRoomQuery.run({ userId, roomId });

  if (previousRoomId && previousRoomId !== roomId) {
    deleteRoomIfEmptyQuery.run({ id: previousRoomId });
  }
};

// Who may relay an offer, answer or ICE candidate to whom, naming `roomId`.
// Each party must be in `roomId` or in no room, and at least one must be in
// `roomId`: the client sends its offers and ICE candidates BEFORE it sends
// `Join`, and room members answer a joiner who is not a member yet.
export const canSignal = (
  senderId: string,
  targetId: string,
  roomId: string,
): boolean => {
  const sender = getUserById(senderId);
  const target = getUserById(targetId);

  if (!sender || !target || senderId === targetId) return false;

  const roomIds = [sender.room_id, target.room_id];

  return (
    roomIds.includes(roomId) &&
    roomIds.every(id => id === roomId || id === null)
  );
};

const getSharedSecretQuery = db.query(`
	SELECT secret
	FROM shared_secrets
	WHERE user_id = $id
`);

export const getSharedSecret = (userId: string): string | null =>
  getSharedSecretQuery.get({ id: userId }) as string | null;

export const setSharedSecret = (userId: string, secret: string) => {
  try {
    db.run(
      `
			INSERT OR REPLACE INTO shared_secrets (user_id, secret)
			VALUES (?, ?)
		`,
      [userId, secret],
    );
  } catch (error) {
    console.error('Error setting shared secret:', error);
  }
};

export { db };
