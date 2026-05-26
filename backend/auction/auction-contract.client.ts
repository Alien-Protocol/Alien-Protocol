import { Injectable, Logger } from '@nestjs/common';
import { rpc, xdr, scValToNative, nativeToScVal } from '@stellar/stellar-sdk';
import { StellarService } from '../src/stellar/stellar.service';

@Injectable()
export class AuctionContractClient {
  private readonly logger = new Logger(AuctionContractClient.name);

  constructor(private readonly stellarService: StellarService) {}

  private get server(): rpc.Server {
    return this.stellarService.getServer();
  }

  private get contract() {
    return this.stellarService.getAuctionContract();
  }

  /** Call a read-only contract function and return the decoded native value */
  private async simulateCall(method: string, args: xdr.ScVal[]): Promise<unknown> {
    const account = await this.server.getAccount(
      // A dummy source — read-only sim doesn't need a real funded account
      'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
    );

    const tx = new (await import('@stellar/stellar-sdk')).TransactionBuilder(account, {
      fee: '100',
      networkPassphrase: (await this.server.getNetwork()).passphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const sim = await this.server.simulateTransaction(tx);

    if (rpc.Api.isSimulationError(sim)) {
      throw new Error(`Contract simulation failed [${method}]: ${sim.error}`);
    }

    const resultVal = (sim as rpc.Api.SimulateTransactionSuccessResponse).result?.retval;
    if (!resultVal) throw new Error(`No return value from contract call [${method}]`);

    return scValToNative(resultVal);
  }

  async getAuctionInfo(auctionId: string): Promise<Record<string, unknown> | null> {
    try {
      const result = await this.simulateCall('get_auction_info', [
        nativeToScVal(auctionId, { type: 'symbol' }),
      ]);
      return result as Record<string, unknown>;
    } catch (err) {
      this.logger.warn(`get_auction_info failed for ${auctionId}: ${err.message}`);
      return null;
    }
  }

  async getAllBidders(auctionId: string): Promise<string[]> {
    try {
      const result = await this.simulateCall('get_all_bidders', [
        nativeToScVal(auctionId, { type: 'symbol' }),
      ]);
      return (result as string[]) ?? [];
    } catch (err) {
      this.logger.warn(`get_all_bidders failed for ${auctionId}: ${err.message}`);
      return [];
    }
  }

  async getBid(auctionId: string, bidder: string): Promise<string | null> {
    try {
      const result = await this.simulateCall('get_bid', [
        nativeToScVal(auctionId, { type: 'symbol' }),
        nativeToScVal(bidder, { type: 'address' }),
      ]);
      return result as string;
    } catch (err) {
      this.logger.warn(`get_bid failed for ${auctionId}/${bidder}: ${err.message}`);
      return null;
    }
  }

  async getAuctionByUsernameHash(usernameHash: string): Promise<Record<string, unknown> | null> {
    try {
      const result = await this.simulateCall('get_auction_by_username_hash', [
        nativeToScVal(usernameHash, { type: 'bytes' }),
      ]);
      return result as Record<string, unknown>;
    } catch (err) {
      this.logger.warn(`get_auction_by_username_hash failed for ${usernameHash}: ${err.message}`);
      return null;
    }
  }
}