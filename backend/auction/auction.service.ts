import { Injectable, Logger, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Auction, AuctionStatus } from './entities/auction.entity';
import { AuctionContractClient } from './auction-contract.client';
import { AuctionInfoDto, AuctionBidsDto, AuctionBidDto } from './dto/auction.dto';

const OPEN_AUCTION_TTL_MS = 30_000; // 30 seconds

@Injectable()
export class AuctionService {
  private readonly logger = new Logger(AuctionService.name);

  constructor(
    @InjectRepository(Auction)
    private readonly auctionRepo: Repository<Auction>,
    private readonly contractClient: AuctionContractClient,
  ) {}

  // ─── helpers ──────────────────────────────────────────────────────────────

  private isCacheValid(auction: Auction): boolean {
    if (auction.status !== AuctionStatus.OPEN) {
      // Closed / claimed — cache forever
      return true;
    }
    // Open — refresh after 30s
    return Date.now() - Number(auction.cachedAt) < OPEN_AUCTION_TTL_MS;
  }

  private mapStatus(raw: string): AuctionStatus {
    switch (raw?.toLowerCase()) {
      case 'closed':
        return AuctionStatus.CLOSED;
      case 'claimed':
        return AuctionStatus.CLAIMED;
      default:
        return AuctionStatus.OPEN;
    }
  }

  private toInfoDto(auction: Auction): AuctionInfoDto {
    return {
      auctionId: auction.auctionId,
      seller: auction.seller,
      asset: auction.asset,
      minBid: auction.minBid,
      endTime: Number(auction.endTime),
      highestBid: auction.highestBid,
      highestBidder: auction.highestBidder,
      status: auction.status,
      isClaimed: auction.isClaimed,
    };
  }

  // ─── public API ───────────────────────────────────────────────────────────

  async getAuctionInfo(auctionId: string): Promise<AuctionInfoDto> {
    let cached = await this.auctionRepo.findOne({ where: { auctionId } });

    if (cached && this.isCacheValid(cached)) {
      return this.toInfoDto(cached);
    }

    const raw = await this.contractClient.getAuctionInfo(auctionId);
    if (!raw) throw new NotFoundException(`Auction ${auctionId} not found`);

    const status = this.mapStatus(raw['status'] as string);

    const entity: Partial<Auction> = {
      auctionId,
      seller: raw['seller'] as string,
      asset: raw['asset'] as string,
      minBid: String(raw['min_bid'] ?? raw['minBid'] ?? '0'),
      endTime: Number(raw['end_time'] ?? raw['endTime'] ?? 0),
      highestBid: String(raw['highest_bid'] ?? raw['highestBid'] ?? '0'),
      highestBidder: (raw['highest_bidder'] ?? raw['highestBidder'] ?? '') as string,
      status,
      isClaimed: Boolean(raw['is_claimed'] ?? raw['isClaimed'] ?? false),
      cachedAt: Date.now(),
    };

    if (cached) {
      await this.auctionRepo.update({ auctionId }, entity);
      cached = { ...cached, ...entity } as Auction;
    } else {
      cached = await this.auctionRepo.save(this.auctionRepo.create(entity));
    }

    return this.toInfoDto(cached);
  }

  async getAuctionBids(auctionId: string): Promise<AuctionBidsDto> {
    // Validate auction exists first
    await this.getAuctionInfo(auctionId);

    const bidders = await this.contractClient.getAllBidders(auctionId);
    return { auctionId, bidders };
  }

  async getBid(auctionId: string, bidder: string): Promise<AuctionBidDto> {
    // Validate auction exists first
    await this.getAuctionInfo(auctionId);

    const amount = await this.contractClient.getBid(auctionId, bidder);
    if (amount === null) {
      throw new NotFoundException(`No bid from ${bidder} on auction ${auctionId}`);
    }

    return { auctionId, bidder, amount };
  }

  async getAuctionByUsernameHash(usernameHash: string): Promise<AuctionInfoDto> {
    const raw = await this.contractClient.getAuctionByUsernameHash(usernameHash);
    if (!raw) throw new NotFoundException(`No auction for username hash ${usernameHash}`);

    const auctionId = (raw['auction_id'] ?? raw['auctionId']) as string;
    if (!auctionId) throw new NotFoundException(`No auction for username hash ${usernameHash}`);

    return this.getAuctionInfo(auctionId);
  }
}