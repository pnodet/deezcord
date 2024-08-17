import { Database } from 'bun:sqlite';

const db = new Database('deezcord.sqlite', { create: true, strict: true });

db.run('PRAGMA journal_mode = WAL;');

class User {
  id!: string;
  username!: string;
  alive!: boolean;
  roomId!: string;
}

class Room {
  id!: string;
  name!: string;
  hostId!: string;
  usersIds!: string;

  get users() {
    return this.usersIds.split(',');
  }
}

db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    alive BOOLEAN DEFAULT TRUE,
    roomId TEXT
  )
`);

db.run(`
  CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    hostId TEXT,
    usersIds TEXT
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

const getUserByIdQuery = db
  .query(
    `
  SELECT
    id,
    username,
    alive,
    roomId
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

export const setUserAlive = (id: string, alive: boolean) =>
  setUserAliveQuery.get({ alive, id });

const getRoomQuery = db
  .query(
    `
  SELECT
    id,
    name,
    hostId,
    usersIds
  FROM rooms
  WHERE id = $id
`,
  )
  .as(Room);

export const getRoom = (id: string) => getRoomQuery.get({ id });

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
