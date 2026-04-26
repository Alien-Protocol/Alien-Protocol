import { Entity, PrimaryColumn, Column, Index } from 'typeorm';

export enum AuctionStatus {
  OPEN = 'open',
  CLOSED = 'closed',
  CLAIMED = 'claimed',
}

@Entity()
export class Auction {
  @PrimaryColumn()
  auctionId: string;

  @Column({ nullable: true })
  seller: string;

  @Column({ nullable: true })
  asset: string;

  @Column({ nullable: true })
  minBid: string;

  @Column({ type: 'bigint', nullable: true })
  endTime: number;

  @Column({ nullable: true })
  highestBid: string;

  @Column({ nullable: true })
  highestBidder: string;

  @Column({ default: AuctionStatus.OPEN })
  @Index()
  status: AuctionStatus;

  @Column({ default: false })
  isClaimed: boolean;

  /** Unix ms timestamp of last cache refresh */
  @Column({ type: 'bigint', nullable: true })
  cachedAt: number;
}