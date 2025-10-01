import { MarsRover } from '../../index';
import { SandboxServer } from './server';

export * from '../../index';
export * from './server';

export type LedgerInfo = {
  protocol_version: number;
  sequence_number: number;
  timestamp: number;
  network_id: number[];
  base_reserve: number;
  min_temp_entry_ttl: number;
  min_persistent_entry_ttl: number;
  max_entry_ttl: number;
};

export function makeSandbox() {
  const marsRover = new MarsRover();
  const server = new SandboxServer(marsRover);

  return {
    server,
    marsRover,
  };
}

export function getLedgerInfo(marsRover: MarsRover): LedgerInfo {
  return JSON.parse(marsRover.getLedgerInfo());
}
