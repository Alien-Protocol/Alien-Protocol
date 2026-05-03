import { HttpException, HttpStatus } from '@nestjs/common';

/**
 * Custom exception for Stellar RPC errors
 */
export class StellarRpcException extends HttpException {
  constructor(
    message: string,
    public readonly originalError?: any,
    public readonly contractId?: string,
    public readonly method?: string,
  ) {
    super(
      {
        message,
        error: 'Stellar RPC Error',
        originalError: originalError?.message || originalError,
        contractId,
        method,
      },
      HttpStatus.BAD_GATEWAY,
    );
  }
}