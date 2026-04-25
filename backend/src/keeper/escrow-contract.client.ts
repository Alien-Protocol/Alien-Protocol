import { Injectable } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import {
  Keypair,
  SorobanRpc,
  TransactionBuilder,
  Contract,
  BASE_FEE,
  xdr,
} from '@stellar/stellar-sdk';

@Injectable()
export class EscrowContractClient {
  private readonly server: SorobanRpc.Server;
  private readonly contract: Contract;
  private readonly networkPassphrase: string;

  constructor(private readonly configService: ConfigService) {
    const rpcUrl = this.configService.getOrThrow<string>('STELLAR_RPC_URL');
    const contractId = this.configService.getOrThrow<string>('ESCROW_CONTRACT_ID');
    this.networkPassphrase = this.configService.getOrThrow<string>('STELLAR_NETWORK_PASSPHRASE');
    this.server = new SorobanRpc.Server(rpcUrl);
    this.contract = new Contract(contractId);
  }

  async executeScheduled(paymentId: number, signerSecret: string): Promise<void> {
    const signer = Keypair.fromSecret(signerSecret);
    const source = await this.server.getAccount(signer.publicKey());

    const tx = new TransactionBuilder(source, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call('execute_scheduled', xdr.ScVal.scvU32(paymentId)))
      .setTimeout(30)
      .build();

    const prepared = await this.server.prepareTransaction(tx);
    prepared.sign(signer);
    const response = await this.server.sendTransaction(prepared);

    if (response.status === 'ERROR') {
      const errorXdr = response.errorResult
        ? response.errorResult.toXDR('base64')
        : 'unknown';
      throw new Error(`Transaction failed: ${errorXdr}`);
    }

    if (response.status === 'TRY_AGAIN_LATER') {
      throw new Error('Transaction temporarily rejected: try again later');
    }

    if (response.status === 'PENDING') {
      const hash = response.hash;
      const maxRetries = 15;
      for (let i = 0; i < maxRetries; i++) {
        await new Promise((r) => setTimeout(r, 2000));
        const result = await this.server.getTransaction(hash);
        if (result.status === 'SUCCESS') {
          return;
        }
        if (result.status === 'NOT_FOUND') {
          continue;
        }
        throw new Error(`Transaction failed on-chain: ${result.status}`);
      }
      throw new Error('Transaction polling timed out');
    }

    // DUPLICATE: transaction already in flight, which is acceptable
  }
}
