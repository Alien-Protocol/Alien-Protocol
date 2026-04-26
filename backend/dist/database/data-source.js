"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AppDataSource = void 0;
require("reflect-metadata");
const typeorm_1 = require("typeorm");
const dotenv_1 = require("dotenv");
const entities_1 = require("./entities");
(0, dotenv_1.config)();
exports.AppDataSource = new typeorm_1.DataSource({
    type: 'postgres',
    url: process.env.DATABASE_URL,
    synchronize: false,
    logging: true,
    entities: [entities_1.Username, entities_1.Vault, entities_1.Payment, entities_1.AutoPayRule],
    migrations: [__dirname + '/migrations/**/*{.ts,.js}'],
    subscribers: [],
});
//# sourceMappingURL=data-source.js.map