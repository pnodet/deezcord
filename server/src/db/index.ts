import { Database } from 'bun:sqlite';
import { v7 as uuid } from 'uuid';

const db = new Database('deezcord.sqlite', { create: true, strict: true });

db.run('PRAGMA journal_mode = WAL;');

export class User {
  id!: string;
  username!: string;
  alive!: boolean;
  room_id!: string;
}

export class Room {
  id!: string;
  name!: string;
  users_ids!: string;

  get users(): string[] {
    return this.users_ids?.split(',') ?? [];
  }
}

db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    alive BOOLEAN DEFAULT TRUE,
    room_id TEXT
  )
`);

db.run(`
  CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    users_ids TEXT
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

const createRoomQuery = db
  .query(
    `
  INSERT INTO rooms (id, name, users_ids) VALUES ($id, $roomname, $usersIds) RETURNING *
`,
  )
  .as(Room);

export const createRoom = (userId: string, roomname: string): Room =>
  ({
    ...createRoomQuery.get({
      id: uuid(),
      roomname,
      usersIds: userId,
    }),
    users: [userId],
  }) as Room;

const getRoomsQuery = db
  .query(
    `
		SELECT
			id,
			name,
			users_ids
		FROM rooms
	`,
  )
  .as(Room);

export const getRooms = () =>
  getRoomsQuery
    .all()
    .map(room => ({ ...room, users: room.users_ids?.split(',') ?? [] }));

const getRoomByIdQuery = db
  .query(
    `
		SELECT
			id,
			name,
			users_ids
		FROM rooms WHERE id = $id
	`,
  )
  .as(Room);

export const getRoomById = (id: string) => getRoomByIdQuery.get({ id });

const getRoomQuery = db
  .query(
    `
  SELECT
    id,
    name,
    users_ids
  FROM rooms
  WHERE id = $id
`,
  )
  .as(Room);

export const getRoom = (id: string) => getRoomQuery.get({ id });

const deleteRoomQuery = db
  .query(
    `
	DELETE
	FROM rooms
	WHERE id = $id
`,
  )
  .as(Room);

const deleteRoom = (id: string) => deleteRoomQuery.run({ id });

const addUserToRoomQuery = db
  .query(
    `
		UPDATE rooms SET users_ids = $usersIds WHERE id = $roomId RETURNING *
	`,
  )
  .as(Room);

export const addUserToRoom = (roomId: string, userId: string) => {
  const room = getRoomById(roomId);

  if (!room) throw new Error('Room not found');

  if (room.users.includes(userId)) return room;

  return addUserToRoomQuery.get({
    usersIds: [...room.users, userId].join(','),
    roomId,
  });
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

export const clearRooms = () => {
  for (const room of getRooms().filter(room => room.users.length === 0)) {
    deleteRoom(room.id);
  }
};

export { db };
