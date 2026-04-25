import { Injectable, OnModuleInit, Logger } from '@nestjs/common';
import { rpc, Contract, Address, xdr } from '@stellar/stellar-sdk';
import { ConfigService } from '../config/config.service';
import { StellarRpcException } from './stellar.exceptions';
import {
  ChainType,
  GetOwnerResult,
  ResolveStellarResult,
  GetChainAddressResult,
  GetVaultBalanceResult,
  GetScheduledPaymentResult,
  IsVaultActiveResult,
  GetCreatedAtResult,
  ScheduledPayment,
} from './stellar.types';

@Injectable()
export class StellarService implements OnModuleInit {
  private readonly logger = new Logger(StellarService.name);
  private server: rpc.Server;

  constructor(private configService: ConfigService) {
    this.server = new rpc.Server(this.configService.stellarRpcUrl);
  }

  async onModuleInit() {
    try {
      // Test the connection by getting network info
      const network = await this.server.getNetwork();
      this.logger.log(`Connected to Stellar network: ${network.passphrase} at ${this.configService.stellarRpcUrl}`);
    } catch (error) {
      this.logger.error(`Failed to connect to Stellar RPC: ${error.message}`);
    }
  }

  getServer(): rpc.Server {
    return this.server;
  }

  getCoreContract(): Contract {
    return new Contract(this.configService.coreContractId);
  }

  getEscrowContract(): Contract {
    return new Contract(this.configService.escrowContractId);
  }

  getFactoryContract(): Contract {
    return new Contract(this.configService.factoryContractId);
  }

  getAuctionContract(): Contract {
    return new Contract(this.configService.auctionContractId);
  }

  /**
   * Get the owner of a commitment from the core contract
   */
  async getOwner(commitment: string): Promise<GetOwnerResult> {
    try {
      const contract = this.getCoreContract();
      const result = await this.server.simulateTransaction(
        contract.call('get_owner', commitment),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      return Address.fromScVal(parsed).toString();
    } catch (error) {
      this.logger.error(`Failed to get owner for commitment ${commitment}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to get owner for commitment`,
        error,
        this.configService.coreContractId,
        'get_owner',
      );
    }
  }

  /**
   * Resolve a username hash to a Stellar address from the core contract
   */
  async resolveUsername(usernameHash: string): Promise<ResolveStellarResult> {
    try {
      const contract = this.getCoreContract();
      const result = await this.server.simulateTransaction(
        contract.call('resolve_stellar', usernameHash),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        throw new Error('No return value from resolve_stellar');
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      return Address.fromScVal(parsed).toString();
    } catch (error) {
      this.logger.error(`Failed to resolve username hash ${usernameHash}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to resolve username hash`,
        error,
        this.configService.coreContractId,
        'resolve_stellar',
      );
    }
  }

  /**
   * Get a chain address for a username hash and chain type from the core contract
   */
  async getChainAddress(usernameHash: string, chain: ChainType): Promise<GetChainAddressResult> {
    try {
      const contract = this.getCoreContract();
      const result = await this.server.simulateTransaction(
        contract.call('get_chain_address', usernameHash, chain),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      // Convert bytes to string
      const bytes = parsed.bytes();
      return Buffer.from(bytes).toString('utf8');
    } catch (error) {
      this.logger.error(`Failed to get chain address for ${usernameHash} on ${chain}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to get chain address`,
        error,
        this.configService.coreContractId,
        'get_chain_address',
      );
    }
  }

  /**
   * Get the balance of a vault from the escrow contract
   */
  async getVaultBalance(commitment: string): Promise<GetVaultBalanceResult> {
    try {
      const contract = this.getEscrowContract();
      const result = await this.server.simulateTransaction(
        contract.call('get_balance', commitment),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      // Convert i128 to string
      return parsed.i128().toString();
    } catch (error) {
      this.logger.error(`Failed to get vault balance for commitment ${commitment}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to get vault balance`,
        error,
        this.configService.escrowContractId,
        'get_balance',
      );
    }
  }

  /**
   * Get a scheduled payment by ID from the escrow contract
   */
  async getScheduledPayment(paymentId: number): Promise<GetScheduledPaymentResult> {
    try {
      const contract = this.getEscrowContract();
      const result = await this.server.simulateTransaction(
        contract.call('get_scheduled_payment', paymentId),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      // Parse the ScheduledPayment struct
      const instance = parsed.instance();
      const fields = instance.instanceValue().map();

      const payment: ScheduledPayment = {
        from: Buffer.from(fields[0].val().bytes()).toString('hex'),
        to: Buffer.from(fields[1].val().bytes()).toString('hex'),
        token: Address.fromScVal(fields[2].val()).toString(),
        amount: fields[3].val().i128().toString(),
        release_at: Number(fields[4].val().u64()),
        executed: fields[5].val().b(),
      };

      return payment;
    } catch (error) {
      this.logger.error(`Failed to get scheduled payment ${paymentId}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to get scheduled payment`,
        error,
        this.configService.escrowContractId,
        'get_scheduled_payment',
      );
    }
  }

  /**
   * Check if a vault is active from the escrow contract
   */
  async isVaultActive(commitment: string): Promise<IsVaultActiveResult> {
    try {
      const contract = this.getEscrowContract();
      const result = await this.server.simulateTransaction(
        contract.call('is_vault_active', commitment),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      return parsed.b();
    } catch (error) {
      this.logger.error(`Failed to check if vault is active for commitment ${commitment}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to check vault active status`,
        error,
        this.configService.escrowContractId,
        'is_vault_active',
      );
    }
  }

  /**
   * Get the creation timestamp of a commitment from the core contract
   */
  async getCreatedAt(commitment: string): Promise<GetCreatedAtResult> {
    try {
      const contract = this.getCoreContract();
      const result = await this.server.simulateTransaction(
        contract.call('get_created_at', commitment),
      );

      if (result.error) {
        throw new Error(result.error);
      }

      const returnValue = result.result?.retval;
      if (!returnValue) {
        return null;
      }

      // Parse the XDR result
      const parsed = xdr.ScVal.fromXDR(returnValue, 'base64');
      if (parsed.switch() === xdr.ScValType.scvVoid()) {
        return null;
      }

      return Number(parsed.u64());
    } catch (error) {
      this.logger.error(`Failed to get created_at for commitment ${commitment}: ${error.message}`);
      throw new StellarRpcException(
        `Failed to get creation timestamp`,
        error,
        this.configService.coreContractId,
        'get_created_at',
      );
    }
  }
}
