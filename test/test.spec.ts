import {
  Address,
  Contract,
  Keypair,
  Operation,
  TransactionBuilder,
  xdr,
} from '@stellar/stellar-sdk';
import { readFileSync } from 'fs';
import { getLedgerInfo, makeSandbox } from '../src/ts';

describe('MarsRover Stellar Sandbox', () => {
  let sandbox: ReturnType<typeof makeSandbox>;
  let server: ReturnType<typeof makeSandbox>['server'];
  let marsRover: ReturnType<typeof makeSandbox>['marsRover'];

  beforeEach(() => {
    sandbox = makeSandbox();
    server = sandbox.server;
    marsRover = sandbox.marsRover;
  });

  const createFundedAccount = (balance: number = 1_000_000_000): Keypair => {
    const keypair = Keypair.random();
    const accountKey = keypair.xdrPublicKey().toXDR('base64');
    marsRover.fundAccount(accountKey, balance);

    return keypair;
  };

  const buildTransaction = async (operation: xdr.Operation, sourceKeypair: Keypair) => {
    const account = await server.getAccount(sourceKeypair.publicKey());
    const networkInfo = await server.getNetwork();

    const transaction = new TransactionBuilder(account, {
      fee: '1000000',
      networkPassphrase: networkInfo.passphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    return await server.prepareTransaction(transaction);
  };

  const executeTransaction = async (transaction: any, signerKeypair: Keypair) => {
    transaction.sign(signerKeypair);
    const sendResponse = await server.sendTransaction(transaction);

    const txResult = await server.getTransaction(sendResponse.hash);

    if (txResult.status === 'SUCCESS') {
      return txResult.returnValue!;
    }

    throw new Error(`Transaction failed with status: ${txResult.status}`);
  };

  describe('Basic Operations', () => {
    it('should handle transaction without funded account (expect failure)', async () => {
      const invalidTxXdr =
        'AAAAAgAAAADMhyUr2DTDvFw70TSRmUhm52A7PuMt8uIOjFhC0uBuQAADJYEABOVfAAAABAAAAAEAAAAAAAAAAAAAAABo0rExAAAAAAAAAAEAAAAAAAAAGAAAAAAAAAABq4P5a+MLZ/WiVyampwIfs6crA21Ih8/p1VIFkMe4clcAAAAMY2hhbmdlX293bmVyAAAAAQAAABIAAAAAAAAAAPqS9Q/j4wXhAhrzZpNIu33tjelksUUC2T/fWnuxWO1pAAAAAQAAAAEAAAAAAAAAAPqS9Q/j4wXhAhrzZpNIu33tjelksUUC2T/fWnuxWO1pBztbQQm6H94AAAAAAAAAAQAAAAAAAAABq4P5a+MLZ/WiVyampwIfs6crA21Ih8/p1VIFkMe4clcAAAAMY2hhbmdlX293bmVyAAAAAQAAABIAAAAAAAAAAPqS9Q/j4wXhAhrzZpNIu33tjelksUUC2T/fWnuxWO1pAAAAAAAAAAEAAAAAAAAAAgAAAAAAAAAA+pL1D+PjBeECGvNmk0i7fe2N6WSxRQLZP99ae7FY7WkAAAAHDOxN+5wG3QW5dPtODYSdkZ7trvqVPuHZRWiNsaFO32EAAAACAAAABgAAAAAAAAAA+pL1D+PjBeECGvNmk0i7fe2N6WSxRQLZP99ae7FY7WkAAAAVBztbQQm6H94AAAAAAAAABgAAAAGrg/lr4wtn9aJXJqanAh+zpysDbUiHz+nVUgWQx7hyVwAAABQAAAABABH2gwAAAJAAAAEcAAAAAAADJR0AAAAA';

      await expect(async () => {
        await server.sendTransaction(
          TransactionBuilder.fromXDR(
            invalidTxXdr,
            await server.getNetwork().then((n) => n.passphrase),
          ),
        );
      }).rejects.toThrow();
    });

    it('should return network information', async () => {
      const networkInfo = await server.getNetwork();

      expect(networkInfo).toHaveProperty('passphrase');
      expect(networkInfo).toHaveProperty('protocolVersion');
    });

    it('should set time and ledger', () => {
      marsRover.setTime(1200);
      marsRover.setSequence(1301);

      const info = getLedgerInfo(marsRover);

      expect(info.sequence_number).toBe(1301);
      expect(info.timestamp).toBe(1200);
    });

    it('should fund account and retrieve balance', async () => {
      const keypair = createFundedAccount(1000);
      const balance = marsRover.getBalance(keypair.xdrPublicKey().toXDR('base64'));

      expect(Number(balance)).toBe(1000);
    });

    it('should fund account and retrieve account details', async () => {
      const keypair = createFundedAccount(1000);
      const account = await server.getAccount(keypair.publicKey());

      expect(account.accountId()).toBe(keypair.publicKey());
      expect(account.sequenceNumber()).toBe('0');
    });
  });

  describe('Contract Operations', () => {
    let contractWasm: Buffer;

    beforeAll(() => {
      contractWasm = readFileSync('./test/redstone_adapter.wasm');
    });

    it('should deploy contract and execute operations', async () => {
      const ownerKeypair = createFundedAccount();
      const userKeypair = createFundedAccount();

      const uploadTx = await buildTransaction(
        Operation.uploadContractWasm({ wasm: contractWasm }),
        ownerKeypair,
      );

      const uploadResult = await executeTransaction(uploadTx, ownerKeypair);
      const wasmHash = uploadResult.bytes();

      expect(wasmHash).toBeInstanceOf(Buffer);
      expect(wasmHash?.length).toBe(32);

      const createContractTx = await buildTransaction(
        Operation.createCustomContract({
          wasmHash: wasmHash!,
          address: Address.fromString(ownerKeypair.publicKey()),
        }),
        ownerKeypair,
      );

      const createResult = await executeTransaction(createContractTx, ownerKeypair);
      const contractAddress = Address.fromScVal(createResult!);
      const contract = new Contract(contractAddress.toString());

      expect(contractAddress.toString()).toMatch(/^C[A-Z0-9]{55}$/);

      const initTx = await buildTransaction(
        contract.call(
          'init',
          xdr.ScVal.scvAddress(new Address(ownerKeypair.publicKey()).toScAddress()),
        ),
        ownerKeypair,
      );

      await executeTransaction(initTx, ownerKeypair);

      const changeOwnerTx = await buildTransaction(
        contract.call(
          'change_owner',
          xdr.ScVal.scvAddress(new Address(ownerKeypair.publicKey()).toScAddress()),
        ),
        ownerKeypair,
      );

      await executeTransaction(changeOwnerTx, ownerKeypair);

      const unauthorizedChangeOwnerTx = await buildTransaction(
        contract.call(
          'change_owner',
          xdr.ScVal.scvAddress(new Address(ownerKeypair.publicKey()).toScAddress()),
        ),
        userKeypair,
      );

      await expect(async () => {
        await executeTransaction(unauthorizedChangeOwnerTx, userKeypair);
      }).rejects.toThrow();
    });

    it('should handle contract deployment with proper WASM hash format', async () => {
      const keypair = createFundedAccount();

      const uploadTx = await buildTransaction(
        Operation.uploadContractWasm({ wasm: contractWasm }),
        keypair,
      );

      const wasmHashScVal = await executeTransaction(uploadTx, keypair);

      expect(wasmHashScVal.switch()).toBe(xdr.ScValType.scvBytes());
      expect(wasmHashScVal.bytes().length).toBe(32);
    });

    it('should handle contract address creation properly', async () => {
      const keypair = createFundedAccount();

      const uploadTx = await buildTransaction(
        Operation.uploadContractWasm({ wasm: contractWasm }),
        keypair,
      );

      const wasmHashScVal = await executeTransaction(uploadTx, keypair);
      const wasmHash = wasmHashScVal.bytes();

      const createTx = await buildTransaction(
        Operation.createCustomContract({
          wasmHash,
          address: Address.fromString(keypair.publicKey()),
        }),
        keypair,
      );

      const contractAddressScVal = await executeTransaction(createTx, keypair);

      expect(contractAddressScVal.switch()).toBe(xdr.ScValType.scvAddress());

      const contractAddress = Address.fromScVal(contractAddressScVal);
      expect(contractAddress.toString()).toMatch(/^C[A-Z0-9]{55}$/);
    });

    it('should fail when calling non-existing contract function', async () => {
      const ownerKeypair = createFundedAccount();

      const uploadTx = await buildTransaction(
        Operation.uploadContractWasm({ wasm: contractWasm }),
        ownerKeypair,
      );

      const wasmHashScVal = await executeTransaction(uploadTx, ownerKeypair);
      const wasmHash = wasmHashScVal.bytes();

      const createContractTx = await buildTransaction(
        Operation.createCustomContract({
          wasmHash,
          address: Address.fromString(ownerKeypair.publicKey()),
        }),
        ownerKeypair,
      );

      const contractAddressScVal = await executeTransaction(createContractTx, ownerKeypair);
      const contractAddress = Address.fromScVal(contractAddressScVal);
      const contract = new Contract(contractAddress.toString());

      const initTx = await buildTransaction(
        contract.call(
          'init',
          xdr.ScVal.scvAddress(new Address(ownerKeypair.publicKey()).toScAddress()),
        ),
        ownerKeypair,
      );

      await executeTransaction(initTx, ownerKeypair);

      await expect(async () => {
        await buildTransaction(
          contract.call('nonExistentFunction', xdr.ScVal.scvString('test')),
          ownerKeypair,
        );
      }).rejects.toThrow();
    });
  });

  describe('Error Handling', () => {
    it('should handle invalid account keys', () => {
      expect(() => {
        marsRover.fundAccount('invalid-key', 1000);
      }).toThrow();
    });

    it('should handle requests for non-existent accounts', async () => {
      const randomKey = Keypair.random().publicKey();

      await expect(async () => {
        await server.getAccount(randomKey);
      }).rejects.toThrow();
    });

    it('should handle malformed transaction XDR', async () => {
      await expect(async () => {
        const account = await server.getAccount(
          createFundedAccount().xdrPublicKey().toXDR('base64'),
        );
        const networkInfo = await server.getNetwork();
        const malformedTx = new TransactionBuilder(account, {
          fee: '1000000',
          networkPassphrase: networkInfo.passphrase,
        }).build();

        await server.sendTransaction(malformedTx);
      }).rejects.toThrow();
    });
  });

  describe('Authorization', () => {
    it('should reject unauthorized operations', async () => {
      const ownerKeypair = createFundedAccount();
      const unauthorizedKeypair = createFundedAccount();

      const uploadTx = await buildTransaction(
        Operation.uploadContractWasm({ wasm: readFileSync('./test/redstone_adapter.wasm') }),
        ownerKeypair,
      );

      const wasmHashScVal = await executeTransaction(uploadTx, ownerKeypair);
      const wasmHash = wasmHashScVal.bytes();

      const createTx = await buildTransaction(
        Operation.createCustomContract({
          wasmHash,
          address: Address.fromString(ownerKeypair.publicKey()),
        }),
        ownerKeypair,
      );

      const contractAddressScVal = await executeTransaction(createTx, ownerKeypair);
      const contractAddress = Address.fromScVal(contractAddressScVal);
      const contract = new Contract(contractAddress.toString());

      const initTx = await buildTransaction(
        contract.call(
          'init',
          xdr.ScVal.scvAddress(new Address(ownerKeypair.publicKey()).toScAddress()),
        ),
        ownerKeypair,
      );

      await executeTransaction(initTx, ownerKeypair);

      const unauthorizedTx = await buildTransaction(
        contract.call(
          'change_owner',
          xdr.ScVal.scvAddress(new Address(unauthorizedKeypair.publicKey()).toScAddress()),
        ),
        unauthorizedKeypair,
      );

      await expect(async () => {
        await executeTransaction(unauthorizedTx, unauthorizedKeypair);
      }).rejects.toThrow();
    });
  });
});
