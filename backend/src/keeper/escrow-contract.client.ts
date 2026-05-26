import { Injectable } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import * as StellarSdk from '@stellar/stellar-sdk';

@Injectable()
export class EscrowContractClient {
  private readonly server: any;
  private readonly contract: any;
  private readonly networkPassphrase: string;

  constructor(private readonly configService: ConfigService) {
    const rpcUrl = this.configService.getOrThrow<string>('STELLAR_RPC_URL');
    const contractId = this.configService.getOrThrow<string>('ESCROW_CONTRACT_ID');
    this.networkPassphrase = this.configService.getOrThrow<string>('STELLAR_NETWORK_PASSPHRASE');
    this.server = new (StellarSdk as any).SorobanRpc.Server(rpcUrl);
    this.contract = new StellarSdk.Contract(contractId);
  }

  private async submitAndAwait(tx: any, signer: any): Promise<void> {
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

  async executeScheduled(paymentId: number, signerSecret: string): Promise<void> {
    const signer = StellarSdk.Keypair.fromSecret(signerSecret);
    const source = await this.server.getAccount(signer.publicKey());

    const tx = new StellarSdk.TransactionBuilder(source, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call('execute_scheduled', StellarSdk.xdr.ScVal.scvU32(paymentId)))
      .setTimeout(30)
      .build();

    await this.submitAndAwait(tx, signer);
  }

  async triggerAutoPay(fromCommitment: string, ruleId: number, signerSecret: string): Promise<void> {
    const normalized = fromCommitment.trim();
    const hex = normalized.startsWith('0x') ? normalized.slice(2) : normalized;
    if (!/^[0-9a-fA-F]+$/.test(hex) || hex.length % 2 !== 0) {
      throw new Error(`Invalid fromCommitment: expected even-length hex string, got "${fromCommitment}"`);
    }
    if (hex.length !== 64) {
      throw new Error(`Invalid fromCommitment: expected 32 bytes (64 hex chars), got ${hex.length} chars`);
    }

    const signer = StellarSdk.Keypair.fromSecret(signerSecret);
    const source = await this.server.getAccount(signer.publicKey());

    const tx = new StellarSdk.TransactionBuilder(source, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          'trigger_auto_pay',
          StellarSdk.xdr.ScVal.scvBytes(Buffer.from(hex, 'hex')),
          StellarSdk.xdr.ScVal.scvU32(ruleId),
        ),
      )
      .setTimeout(30)
      .build();

    await this.submitAndAwait(tx, signer);
  }
}
