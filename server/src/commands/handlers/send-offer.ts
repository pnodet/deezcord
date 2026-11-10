import { canSignal } from '../../db';
import { sendTo } from '../send';
import type { ClientCommand } from '../mod-client';

export const handleSendOffer = (
  userId: string,
  cmd: ClientCommand<'SendOffer'>,
) => {
  const [targetId, roomId, sdp] = cmd.command.Client.SendOffer;

  if (!canSignal(userId, targetId, roomId)) {
    console.warn(
      `Dropping SendOffer from ${userId} to ${targetId} in ${roomId}`,
    );

    return;
  }

  sendTo(targetId, { IncomingOffer: [userId, roomId, sdp] });
};
