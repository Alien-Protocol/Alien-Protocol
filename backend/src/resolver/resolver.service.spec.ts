import { NotFoundException } from '@nestjs/common';
import { ResolverService, type CoreContractClient } from './resolver.service';
import { Username } from '../username/username.entity';

describe('ResolverService', () => {
  const usernameHash = `0x${'a'.repeat(64)}`;

  const makeRepository = (cached: Username | null = null) => ({
    findOne: jest.fn().mockResolvedValue(cached),
    save: jest.fn().mockImplementation(async (username: Partial<Username>) => username),
  });

  const makeService = (
    repository: ReturnType<typeof makeRepository>,
    coreContract: Partial<CoreContractClient>,
  ) => new ResolverService(repository as never, coreContract as CoreContractClient);

  it('returns cached address data without calling the contract', async () => {
    const cached = {
      usernameHash,
      stellarAddress: 'G_STELLAR',
      owner: 'G_OWNER',
      evmAddress: '0x1111111111111111111111111111111111111111',
      bitcoinAddress: null,
      solanaAddress: 'So11111111111111111111111111111111111111112',
      cosmosAddress: null,
      registeredAt: new Date('2026-01-02T03:04:05.000Z'),
    } as Username;
    const repository = makeRepository(cached);
    const coreContract = { resolve_stellar: jest.fn() };
    const service = makeService(repository, coreContract);

    await expect(service.resolve(usernameHash)).resolves.toEqual({
      stellarAddress: 'G_STELLAR',
      chainAddresses: {
        EVM: '0x1111111111111111111111111111111111111111',
        Solana: 'So11111111111111111111111111111111111111112',
      },
      owner: 'G_OWNER',
      registeredAt: '2026-01-02T03:04:05.000Z',
    });
    expect(coreContract.resolve_stellar).not.toHaveBeenCalled();
    expect(repository.save).not.toHaveBeenCalled();
  });

  it('resolves from contract and saves the result on a cache miss', async () => {
    const repository = makeRepository(null);
    const coreContract = {
      resolve_stellar: jest.fn().mockResolvedValue('G_STELLAR'),
      get_owner: jest.fn().mockResolvedValue('G_OWNER'),
      get_username_record: jest.fn().mockResolvedValue({ registered_at: 1_767_226_800 }),
      get_chain_address: jest.fn().mockImplementation(async (_hash, chain) => {
        if (chain === 'EVM') return '0x2222222222222222222222222222222222222222';
        if (chain === 'Bitcoin') return Buffer.from('bc1qexample');
        return null;
      }),
    };
    const service = makeService(repository, coreContract);

    await expect(service.resolve(usernameHash)).resolves.toEqual({
      stellarAddress: 'G_STELLAR',
      chainAddresses: {
        EVM: '0x2222222222222222222222222222222222222222',
        Bitcoin: 'bc1qexample',
      },
      owner: 'G_OWNER',
      registeredAt: '2026-01-01T00:20:00.000Z',
    });
    expect(coreContract.resolve_stellar).toHaveBeenCalledWith(usernameHash);
    expect(repository.save).toHaveBeenCalledWith({
      usernameHash,
      stellarAddress: 'G_STELLAR',
      owner: 'G_OWNER',
      registeredAt: new Date('2026-01-01T00:20:00.000Z'),
      evmAddress: '0x2222222222222222222222222222222222222222',
      bitcoinAddress: 'bc1qexample',
      solanaAddress: null,
      cosmosAddress: null,
    });
  });

  it('throws NotFoundException when the username hash is absent on-chain', async () => {
    const repository = makeRepository(null);
    const coreContract = {
      resolve_stellar: jest.fn().mockRejectedValue(new Error('ContractError: NotFound')),
    };
    const service = makeService(repository, coreContract);

    await expect(service.resolve(usernameHash)).rejects.toThrow(NotFoundException);
    await expect(service.resolve(usernameHash)).rejects.toThrow(
      `Username hash ${usernameHash} was not found on-chain`,
    );
    expect(repository.save).not.toHaveBeenCalled();
  });
});
