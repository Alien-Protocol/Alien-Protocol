export class AuctionInfoDto {
    auctionId: string;
    seller: string;
    asset: string;
    minBid: string;
    endTime: number;
    highestBid: string;
    highestBidder: string;
    status: string;
    isClaimed: boolean;
  }
  
  export class AuctionBidsDto {
    auctionId: string;
    bidders: string[];
  }
  
  export class AuctionBidDto {
    auctionId: string;
    bidder: string;
    amount: string;
  }