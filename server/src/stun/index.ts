import { StunServer, environment } from './server';

export const initStunServer = () => {
  const udp4Port = Number.parseInt(environment.APP.APP_PORT_UDP4 ?? `3479`, 10);
  const udp6Port = Number.parseInt(environment.APP.APP_PORT_UDP6 ?? `3478`, 10);

  if (udp4Port) {
    const serverUDP4 = new StunServer('udp4');

    serverUDP4.start(udp4Port);
  }

  if (udp6Port) {
    const serverUDP6 = new StunServer('udp6');

    serverUDP6.start(udp6Port);
  }
};
