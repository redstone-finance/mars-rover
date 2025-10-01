# MarsRover

⚠️ **Early Development Notice**: This project is under active development. Many features are incomplete or may not work as expected. Use at your own risk.

A Stellar blockchain sandbox for smart contract testing built with Rust and NAPI-RS bindings. Provides a Soroban simulation environment with TypeScript integration via an overridden Server class from the Stellar SDK.

## Desired Features

This is a best-effort blockchain simulation with the following limitations:

- Only HostFunctions are supported
- The deducted fee is not yet calculated, just taken from the transaction
- Errors are not always exactly as they should be (the sandbox errors out correctly but doesn't distinguish errors for the user)
- TypeScript integration via overridden Server from the Stellar SDK

**SandboxServer** is a drop-in replacement for `stellar-rpc.Server` designed for tests.

Core functionality:

- Stellar ledger state management with account funding and transaction execution
- Soroban contract deployment and invocation
- Memory-based storage with TTL support
- Rust-based execution with TypeScript bindings

## Installation

```bash
npm install mars-rover
```

## Usage

### Basic Setup

```typescript
import { makeSandbox } from 'mars-rover';
import { Keypair } from '@stellar/stellar-sdk';

const { server, marsRover } = makeSandbox();

// Fund an account
const keypair = Keypair.random();
const accountKey = keypair.xdrPublicKey().toXDR('base64');
marsRover.fundAccount(accountKey, 1_000_000_000);

// Get account info
const account = await server.getAccount(accountKey);
```

### Contract Operations

```typescript
import { makeSandbox } from 'mars-rover';
import {
  Address,
  Contract,
  Keypair,
  Operation,
  TransactionBuilder,
  xdr,
} from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';

const { server, marsRover } = makeSandbox();

async function deployContract() {
  const keypair = Keypair.random();
  marsRover.fundAccount(keypair.xdrPublicKey().toXDR('base64'), 1_000_000_000);

  const account = await server.getAccount(keypair.xdrPublicKey().toXDR('base64'));
  const networkInfo = await server.getNetwork();

  // Upload WASM
  const contractWasm = readFileSync('./contract.wasm');
  const uploadTx = new TransactionBuilder(account, {
    fee: '1000000',
    networkPassphrase: networkInfo.passphrase,
  })
    .addOperation(Operation.uploadContractWasm({ wasm: contractWasm }))
    .setTimeout(30)
    .build();

  const preparedUploadTx = await server.prepareTransaction(uploadTx);
  preparedUploadTx.sign(keypair);

  const uploadResponse = await server.sendTransaction(preparedUploadTx);
  const uploadResult = await server.getTransaction(uploadResponse.hash);
  const wasmHash = uploadResult.returnValue.bytes();

  // Create contract
  const createTx = new TransactionBuilder(account, {
    fee: '1000000',
    networkPassphrase: networkInfo.passphrase,
  })
    .addOperation(
      Operation.createCustomContract({
        wasmHash,
        address: Address.fromString(keypair.publicKey()),
      }),
    )
    .setTimeout(30)
    .build();

  const preparedCreateTx = await server.prepareTransaction(createTx);
  preparedCreateTx.sign(keypair);

  const createResponse = await server.sendTransaction(preparedCreateTx);
  const createResult = await server.getTransaction(createResponse.hash);
  const contractAddress = Address.fromScVal(createResult.returnValue);

  return { contractAddress, keypair };
}

// Invoke contract function
const { contractAddress, keypair } = await deployContract();
const contract = new Contract(contractAddress.toString());

const invokeTx = new TransactionBuilder(account, {
  fee: '1000000',
  networkPassphrase: networkInfo.passphrase,
})
  .addOperation(contract.call('hello', xdr.ScVal.scvString('world')))
  .setTimeout(30)
  .build();

const preparedInvokeTx = await server.prepareTransaction(invokeTx);
preparedInvokeTx.sign(keypair);

const invokeResponse = await server.sendTransaction(preparedInvokeTx);
const invokeResult = await server.getTransaction(invokeResponse.hash);
```

### Reading Contract Data

```typescript
const contractData = await server.getContractData(
  contractAddress,
  xdr.ScVal.scvString('key'),
  'persistent',
);

console.log('Contract data:', contractData);
```

## API Reference

### MarsRover Class

```typescript
class MarsRover {
  constructor();

  // Time and ledger control
  setTime(time: number): void;
  setSequence(seq: number): void;
  getLedgerInfo(): string;

  // Account management
  fundAccount(account: string, balance: number): void;
  getBalance(account: string): string;

  // Network information
  networkPassphrase(): string;

  // Internal functions (use SandboxServer instead)
  getNetworkInfo(): string;
  getAccount(account: string): string;
  simulateTx(transactionEnvelope: string): string;
  sendTransaction(transactionEnvelope: string): string;
  getContractData(contractAddress: string, key: string, durability: string): string;
  getTransaction(hash: string): string;
}
```

### SandboxServer Class

Drop-in replacement for `stellar-rpc.Server`:

- Only functions listed bellow are overridden - they should be sufficient to interact with contracts.

```typescript
class SandboxServer extends rpc.Server {
  getAccount(address: string): Promise<Account>;
  getNetwork(): Promise<Api.GetNetworkResponse>;
  simulateTransaction(tx: Transaction): Promise<Api.SimulateTransactionResponse>;
  sendTransaction(tx: Transaction): Promise<Api.SendTransactionResponse>;
  getTransaction(hash: string): Promise<Api.GetTransactionResponse>;
  getContractData(
    contract: string | Address | Contract,
    key: xdr.ScVal,
    durability?: rpc.Durability,
  ): Promise<Api.LedgerEntryResult>;
}
```

## Testing

Replace your RPC server in tests:

```typescript
describe('Contract Tests', () => {
  let server: SandboxServer;

  beforeEach(() => {
    const { server: sandboxServer, marsRover } = makeSandbox();
    server = sandboxServer;
  });

  it('should deploy and invoke contract', async () => {
    // Use server.prepareTransaction(), server.sendTransaction(), etc.
    // exactly like you would with stellar-rpc.Server

    // manipulate ledger, for example set time
    marsRover.setTime(10);
  });
});
```

## Development

### Prerequisites

- Rust (latest stable)
- Node.js >= 16.0.0
- Yarn 4.x

### Building & Tests

```bash
# Install dependencies
yarn install

# Build both Rust and TypeScript
yarn build

# Build in watch mode
yarn build:ts:watch

# Run all tests, really slow
yarn test

# Run tests in debug mode, faster compilation prefer this
yarn test:debug
```

## Credits

This project is built using [napi-rs](https://github.com/napi-rs/napi-rs), which provides excellent Rust bindings for Node.js. The project structure and build configuration are based on the [napi-rs package template](https://github.com/napi-rs/package-template).

## License

MIT License - see LICENSE file for details.
