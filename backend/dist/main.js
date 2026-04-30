"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const core_1 = require("@nestjs/core");
p;
const common_1 = require("@nestjs/common");
async function bootstrap() {
    const app = await core_1.NestFactory.create(app_module_1.AppModule);
    app.useGlobalPipes(new common_1.ValidationPipe({
        whitelist: true,
        transform: true,
        forbidNonWhitelisted: true,
    }));
    await app.listen(process.env.PORT ?? 3000);
    import { AppModule } from './app.module';
    async function bootstrap() {
        try {
            const app = await core_1.NestFactory.create(app_module_1.AppModule);
            const port = process.env.PORT ? parseInt(process.env.PORT, 10) : 3000;
            await app.listen(port);
        }
        catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            console.error(`Failed to start application: ${message}`);
            process.exit(1);
        }
    }
    bootstrap();
}
//# sourceMappingURL=main.js.map