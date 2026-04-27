import { Controller, Get, Param, ValidationPipe } from '@nestjs/common';
import { UsernameHashDto } from './dto/resolve-username.dto';
import { ResolverService } from './resolver.service';

@Controller('resolve')
export class ResolverController {
  constructor(private readonly resolverService: ResolverService) {}

  @Get(':usernameHash')
  resolve(
    @Param(new ValidationPipe({ transform: true, whitelist: true }))
    params: UsernameHashDto,
  ) {
    return this.resolverService.resolve(params.usernameHash);
  }

  @Get(':usernameHash/stellar')
  resolveStellar(
    @Param(new ValidationPipe({ transform: true, whitelist: true }))
    params: UsernameHashDto,
  ) {
    return this.resolverService.resolveStellar(params.usernameHash);
  }
}
