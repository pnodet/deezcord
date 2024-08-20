import type { ServerCommand, ServerCommandKeys } from './mod-server';

export const prepareCommand = <K extends ServerCommandKeys>(
  command: ServerCommand<K>,
): string => {
  console.log('\n\n*** Sending ***', { command });

  return JSON.stringify(command);
};
