import { Test, TestingModule } from '@nestjs/testing';
import { getRepositoryToken } from '@nestjs/typeorm';
import { NotFoundException } from '@nestjs/common';
import { Repository } from 'typeorm';
import { AuctionService } from './auction.service';
import { AuctionContractClient } from './auction-contract.client';
import { Auction, AuctionStatus } from './entities/auction.entity';

const mockRepo = () => ({
  findOne: jest.fn(),
  save: jest.fn(),
  update: jest.fn(),
  create: jest.fn((v) => v),
});

const mockContractClient = () => ({
  getAuctionInfo: jest.fn(),
  getAllBidders: jest.fn(),
  getBid: jest.fn(),
  getAuctionByUsernameHash: jest.fn(),
});

const RAW_OPEN_AUCTION = {
  seller: 'GABC123',
  asset: 'XLM',
  min_bid: '100',
  end_time: 9999999999,
  highest_bid: '150',
  highest_bidder: 'GXYZ456',
  status: 'open',
  is_claimed: false,
};

describe('AuctionService', () => {
  let service: AuctionService;
  let repo: jest.Mocked<Repository<Auction>>;
  let client: jest.Mocked<AuctionContractClient>;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [
        AuctionService,
        { provide: getRepositoryToken(Auction), useFactory: mockRepo },
        { provide: AuctionContractClient, useFactory: mockContractClient },
      ],
    }).compile();

    service = module.get(AuctionService);
    repo = module.get(getRepositoryToken(Auction));
    client = module.get(AuctionContractClient);
  });

  // ─── getAuctionInfo ──────────────────────────────────────────────────────

  describe('getAuctionInfo', () => {
    it('returns cached data if cache is still valid (open, within 30s)', async () => {
      const cached: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now() - 5_000, // 5s ago — still valid
      };
      repo.findOne.mockResolvedValue(cached);

      const result = await service.getAuctionInfo('auction-1');

      expect(result.auctionId).toBe('auction-1');
      expect(result.status).toBe('open');
      expect(client.getAuctionInfo).not.toHaveBeenCalled();
    });

    it('fetches from contract when cache is stale (open, >30s)', async () => {
      const stale: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now() - 60_000, // 1 min ago — stale
      };
      repo.findOne.mockResolvedValue(stale);
      client.getAuctionInfo.mockResolvedValue(RAW_OPEN_AUCTION);
      repo.update.mockResolvedValue(undefined);

      const result = await service.getAuctionInfo('auction-1');

      expect(client.getAuctionInfo).toHaveBeenCalledWith('auction-1');
      expect(repo.update).toHaveBeenCalled();
      expect(result.seller).toBe('GABC123');
    });

    it('fetches from contract and saves when not cached', async () => {
      repo.findOne.mockResolvedValue(null);
      client.getAuctionInfo.mockResolvedValue(RAW_OPEN_AUCTION);
      repo.save.mockResolvedValue({ ...RAW_OPEN_AUCTION, auctionId: 'auction-1', cachedAt: Date.now() } as any);

      const result = await service.getAuctionInfo('auction-1');

      expect(client.getAuctionInfo).toHaveBeenCalledWith('auction-1');
      expect(repo.save).toHaveBeenCalled();
      expect(result.minBid).toBe('100');
    });

    it('never re-fetches a closed auction (indefinite cache)', async () => {
      const closed: Auction = {
        auctionId: 'auction-2',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 1000,
        highestBid: '200',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.CLOSED,
        isClaimed: false,
        cachedAt: Date.now() - 999_999_999, // very old — still valid because closed
      };
      repo.findOne.mockResolvedValue(closed);

      const result = await service.getAuctionInfo('auction-2');

      expect(client.getAuctionInfo).not.toHaveBeenCalled();
      expect(result.status).toBe('closed');
    });

    it('throws NotFoundException for unknown auction ID', async () => {
      repo.findOne.mockResolvedValue(null);
      client.getAuctionInfo.mockResolvedValue(null);

      await expect(service.getAuctionInfo('unknown-id')).rejects.toThrow(NotFoundException);
    });
  });

  // ─── getAuctionBids ──────────────────────────────────────────────────────

  describe('getAuctionBids', () => {
    it('returns all bidders for a known auction', async () => {
      const cached: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now(),
      };
      repo.findOne.mockResolvedValue(cached);
      client.getAllBidders.mockResolvedValue(['GXYZ456', 'GAAA111']);

      const result = await service.getAuctionBids('auction-1');

      expect(result.bidders).toHaveLength(2);
      expect(result.auctionId).toBe('auction-1');
    });

    it('throws NotFoundException if auction does not exist', async () => {
      repo.findOne.mockResolvedValue(null);
      client.getAuctionInfo.mockResolvedValue(null);

      await expect(service.getAuctionBids('unknown-id')).rejects.toThrow(NotFoundException);
    });
  });

  // ─── getBid ──────────────────────────────────────────────────────────────

  describe('getBid', () => {
    it('returns a specific bid', async () => {
      const cached: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now(),
      };
      repo.findOne.mockResolvedValue(cached);
      client.getBid.mockResolvedValue('175');

      const result = await service.getBid('auction-1', 'GBIDDER1');

      expect(result.amount).toBe('175');
      expect(result.bidder).toBe('GBIDDER1');
    });

    it('throws NotFoundException for a non-existent bid', async () => {
      const cached: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now(),
      };
      repo.findOne.mockResolvedValue(cached);
      client.getBid.mockResolvedValue(null);

      await expect(service.getBid('auction-1', 'GNOTABIDDER')).rejects.toThrow(NotFoundException);
    });
  });

  // ─── getAuctionByUsernameHash ─────────────────────────────────────────────

  describe('getAuctionByUsernameHash', () => {
    it('resolves to auction info via contract', async () => {
      client.getAuctionByUsernameHash.mockResolvedValue({ auction_id: 'auction-1' });
      const cached: Auction = {
        auctionId: 'auction-1',
        seller: 'GABC123',
        asset: 'XLM',
        minBid: '100',
        endTime: 9999999999,
        highestBid: '150',
        highestBidder: 'GXYZ456',
        status: AuctionStatus.OPEN,
        isClaimed: false,
        cachedAt: Date.now(),
      };
      repo.findOne.mockResolvedValue(cached);

      const result = await service.getAuctionByUsernameHash('0xabc123');

      expect(result.auctionId).toBe('auction-1');
    });

    it('throws NotFoundException for unknown username hash', async () => {
      client.getAuctionByUsernameHash.mockResolvedValue(null);

      await expect(service.getAuctionByUsernameHash('0xunknown')).rejects.toThrow(NotFoundException);
    });
  });
});