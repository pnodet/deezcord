import type { Room } from '../db';

export type ServerCommandKeys =
  | 'Ack'
  | 'ConnectedAs'
  | 'Refresh'
  | 'RoomList'
  | 'IncomingOffer'
  | 'IncomingAnswer'
  | 'IncomingIceCandidate';

export type ServerCommandData = {
  Ack: null;
  ConnectedAs: null;
  Refresh: null;
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
};

export type ServerCommand<K extends ServerCommandKeys> = {
  user_id: string;
  command: { Server: { [P in K]: ServerCommandData[K] } };
};
