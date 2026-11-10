import type { Room } from '../db';

export type ServerCommandKeys =
  | 'Ack'
  | 'ConnectedAs'
  | 'Refresh'
  | 'RoomList'
  | 'IncomingOffer'
  | 'IncomingAnswer'
  | 'IncomingIceCandidate'
  | 'Error';

// Mirrors `ServerCommand` in client/src/commands/mod.rs.
export type ServerCommandData = {
  // Rust unit variant. serde writes it as `"Ack"` but also accepts the
  // `{ "Ack": null }` the server sends.
  Ack: null;
  ConnectedAs: string;
  Refresh: string[];
  RoomList: Room[];
  IncomingOffer: [
    string, // user_id
    string, // room_id
    string, // sdp
  ];
  IncomingAnswer: [
    string, // user_id
    string, // room_id
    string, // sdp
  ];
  IncomingIceCandidate: [
    string, // user_id
    string, // room_id
    string, // candidate
  ];
  Error: string;
};

export type ServerCommand<K extends ServerCommandKeys> = {
  user_id: string;
  command: { Server: { [P in K]: ServerCommandData[K] } };
};
