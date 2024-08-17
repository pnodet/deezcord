export type ServerCommandKeys =
  | 'Ack'
  | 'ConnectedAs'
  | 'Refresh'
  | 'RoomList'
  | 'SendOffer'
  | 'SendAnswer'
  | 'SendIceCandidate';

export type ServerCommandData = {
  Ack: null;
  ConnectedAs: null;
  Refresh: null;
  RoomList: null;
  SendOffer: null;
  SendAnswer: null;
  SendIceCandidate: null;
};

export type ServerCommand<K extends ServerCommandKeys> = {
  user_id: string;
  command: { Server: { [P in K]: ServerCommandData[K] } };
};
