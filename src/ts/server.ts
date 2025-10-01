import { MarsRover } from '../../index';
import {
  Account,
  Address,
  Contract,
  FeeBumpTransaction,
  Transaction,
  xdr,
  rpc,
  Keypair,
} from '@stellar/stellar-sdk';
import { SorobanDataBuilder } from '@stellar/stellar-base';

export class SandboxServer extends rpc.Server {
  constructor(private readonly sandbox: MarsRover) {
    super('NA', { allowHttp: true });
  }

  override getAccount(address: string): Promise<Account> {
    const accountBs64 = Keypair.fromPublicKey(address).xdrPublicKey().toXDR('base64');

    const accountData: { account_id: string; seq_num: string } = JSON.parse(
      this.sandbox.getAccount(accountBs64),
    );

    return Promise.resolve(new Account(accountData.account_id, accountData.seq_num));
  }

  override getNetwork(): Promise<rpc.Api.GetNetworkResponse> {
    return Promise.resolve(JSON.parse(this.sandbox.getNetworkInfo()));
  }

  override async simulateTransaction(
    tx: Transaction | FeeBumpTransaction,
    _addlResources?: rpc.Server.ResourceLeeway,
    _authMode?: rpc.Api.SimulationAuthMode,
  ): Promise<rpc.Api.SimulateTransactionResponse> {
    const simulation = JSON.parse(this.sandbox.simulateTx(tx.toEnvelope().toXDR('base64')));

    if ('error' in simulation) {
      return simulation;
    }

    simulation.transactionData = new SorobanDataBuilder(
      xdr.SorobanTransactionData.fromXDR(simulation.transactionData, 'base64'),
    );
    simulation.result.auth = simulation.result.auth.map((authEntry: string) =>
      xdr.SorobanAuthorizationEntry.fromXDR(authEntry, 'base64'),
    );
    simulation.result.retval = xdr.ScVal.fromXDR(simulation.result.retval, 'base64');

    return simulation;
  }

  override async getContractData(
    contract: string | Address | Contract,
    key: xdr.ScVal,
    durability = rpc.Durability.Persistent,
  ): Promise<rpc.Api.LedgerEntryResult> {
    let contractAddress: Address;

    if (typeof contract === 'string') {
      contractAddress = Address.fromString(contract);
    } else if (contract instanceof Contract) {
      contractAddress = Address.fromString(contract.contractId());
    } else {
      contractAddress = contract;
    }

    const responseJson = this.sandbox.getContractData(
      contractAddress.toScAddress().toXDR('base64'),
      key.toXDR('base64'),
      durability,
    );

    const response = JSON.parse(responseJson);

    response.key = xdr.LedgerKey.fromXDR(response.key, 'base64');
    response.val = xdr.LedgerEntryData.fromXDR(response.val, 'base64');

    return await Promise.resolve(response);
  }

  override sendTransaction(
    transaction: Transaction | FeeBumpTransaction,
  ): Promise<rpc.Api.SendTransactionResponse> {
    return Promise.resolve(
      JSON.parse(this.sandbox.sendTransaction(transaction.toEnvelope().toXDR('base64'))),
    );
  }

  override getTransaction(hash: string): Promise<rpc.Api.GetTransactionResponse> {
    const response = JSON.parse(this.sandbox.getTransaction(hash));

    response.envelopeXdr = xdr.TransactionEnvelope.fromXDR(response.envelopeXdr, 'base64');
    response.resultXdr = xdr.TransactionResult.fromXDR(response.resultXdr, 'base64');

    if ('returnValue' in response) {
      response.returnValue = xdr.ScVal.fromXDR(Buffer.from(response.returnValue));
    }

    return Promise.resolve(response);
  }
}
