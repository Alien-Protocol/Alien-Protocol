import { Inject, Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Username } from '../username/username.entity';

export const CORE_CONTRACT_CLIENT = Symbol('CORE_CONTRACT_CLIENT');

export type ChainAddressKey = 'EVM' | 'Bitcoin' | 'Solana' | 'Cosmos';

export interface ResolvedUsername {
  stellarAddress: string;
  chainAddresses: Partial<Record<ChainAddressKey, string>>;
  owner: string | null;
  registeredAt: string | null;
}

export interface CoreContractClient {
  resolve_stellar(usernameHash: string): Promise<unknown>;
  get_owner?(usernameHash: string): Promise<unknown>;
  get_chain_address?(usernameHash: string, chain: ChainAddressKey): Promise<unknown>;
  get_username_record?(usernameHash: string): Promise<{
    owner?: unknown;
    registeredAt?: unknown;
    registered_at?: unknown;
  } | null>;
}

@Injectable()
export class ResolverService {
  private readonly chainKeys: ChainAddressKey[] = ['EVM', 'Bitcoin', 'Solana', 'Cosmos'];

  constructor(
    @InjectRepository(Username)
    private readonly usernames: Repository<Username>,
    @Inject(CORE_CONTRACT_CLIENT)
    private readonly coreContract: CoreContractClient,
  ) {}

  async resolve(usernameHash: string): Promise<ResolvedUsername> {
    const cached = await this.findCached(usernameHash);
    if (cached?.stellarAddress) {
      return this.toResponse(cached);
    }

    const live = await this.resolveFromContract(usernameHash);
    const saved = await this.usernames.save({ ...cached, ...live });

    return this.toResponse(saved);
  }

  async resolveStellar(usernameHash: string): Promise<{ stellarAddress: string }> {
    const cached = await this.findCached(usernameHash);
    if (cached?.stellarAddress) {
      return { stellarAddress: cached.stellarAddress };
    }

    const stellarAddress = await this.callResolveStellar(usernameHash);
    await this.usernames.save({
      ...cached,
      usernameHash,
      stellarAddress,
    });

    return { stellarAddress };
  }

  private async findCached(usernameHash: string): Promise<Username | null> {
    return this.usernames.findOne({ where: { usernameHash } });
  }

  private async resolveFromContract(usernameHash: string): Promise<Partial<Username>> {
    const stellarAddress = await this.callResolveStellar(usernameHash);
    const [owner, registeredAt, chainAddresses] = await Promise.all([
      this.resolveOwner(usernameHash),
      this.resolveRegisteredAt(usernameHash),
      this.resolveChainAddresses(usernameHash),
    ]);

    return {
      usernameHash,
      stellarAddress,
      owner,
      registeredAt,
      evmAddress: chainAddresses.EVM ?? null,
      bitcoinAddress: chainAddresses.Bitcoin ?? null,
      solanaAddress: chainAddresses.Solana ?? null,
      cosmosAddress: chainAddresses.Cosmos ?? null,
    };
  }

  private async callResolveStellar(usernameHash: string): Promise<string> {
    try {
      const result = await this.coreContract.resolve_stellar(usernameHash);
      const stellarAddress = this.normalizeAddress(result);

      if (!stellarAddress) {
        throw this.notFound(usernameHash);
      }

      return stellarAddress;
    } catch (error) {
      if (error instanceof NotFoundException) {
        throw error;
      }

      if (this.isContractNotFound(error)) {
        throw this.notFound(usernameHash);
      }

      throw error;
    }
  }

  private async resolveOwner(usernameHash: string): Promise<string | null> {
    if (this.coreContract.get_owner) {
      return this.normalizeAddress(await this.coreContract.get_owner(usernameHash));
    }

    const record = await this.coreContract.get_username_record?.(usernameHash);
    return this.normalizeAddress(record?.owner);
  }

  private async resolveRegisteredAt(usernameHash: string): Promise<Date | null> {
    const record = await this.coreContract.get_username_record?.(usernameHash);
    const value = record?.registeredAt ?? record?.registered_at;

    if (value instanceof Date) {
      return value;
    }

    if (typeof value === 'number') {
      return new Date(value * 1000);
    }

    if (typeof value === 'string') {
      const parsed = new Date(value);
      return Number.isNaN(parsed.getTime()) ? null : parsed;
    }

    return null;
  }

  private async resolveChainAddresses(
    usernameHash: string,
  ): Promise<Partial<Record<ChainAddressKey, string>>> {
    if (!this.coreContract.get_chain_address) {
      return {};
    }

    const entries = await Promise.all(
      this.chainKeys.map(async (chain) => [
        chain,
        this.normalizeAddress(await this.coreContract.get_chain_address?.(usernameHash, chain)),
      ] as const),
    );

    return entries.reduce<Partial<Record<ChainAddressKey, string>>>((addresses, [chain, address]) => {
      if (address) {
        addresses[chain] = address;
      }
      return addresses;
    }, {});
  }

  private toResponse(username: Partial<Username>): ResolvedUsername {
    return {
      stellarAddress: username.stellarAddress as string,
      chainAddresses: {
        ...(username.evmAddress ? { EVM: username.evmAddress } : {}),
        ...(username.bitcoinAddress ? { Bitcoin: username.bitcoinAddress } : {}),
        ...(username.solanaAddress ? { Solana: username.solanaAddress } : {}),
        ...(username.cosmosAddress ? { Cosmos: username.cosmosAddress } : {}),
      },
      owner: username.owner ?? null,
      registeredAt: username.registeredAt ? username.registeredAt.toISOString() : null,
    };
  }

  private normalizeAddress(value: unknown): string | null {
    if (!value) {
      return null;
    }

    if (typeof value === 'string') {
      return value;
    }

    if (Buffer.isBuffer(value)) {
      return value.toString('utf8');
    }

    if (value instanceof Uint8Array) {
      return Buffer.from(value).toString('utf8');
    }

    return String(value);
  }

  private isContractNotFound(error: unknown): boolean {
    if (!error || typeof error !== 'object') {
      return false;
    }

    const candidate = error as { code?: unknown; message?: unknown; name?: unknown };
    const message = String(candidate.message ?? candidate.name ?? '').toLowerCase();

    return candidate.code === 1 || message.includes('notfound') || message.includes('not found');
  }

  private notFound(usernameHash: string): NotFoundException {
    return new NotFoundException(`Username hash ${usernameHash} was not found on-chain`);
  }
}
