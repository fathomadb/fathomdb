// Private admin owner for the TypeScript SDK package root.
import { native } from "./binding.js";

import { validateFfiString } from "./validation.js";

import { WriteReceipt } from "./write.js";

import { Engine } from "./core.js";

import { intercept, interceptSync } from "./native-call.js";

export interface AdminConfigureOptions {
  name: string;
  body: string;
}

export type RuntimeSqliteMode = "performance" | "diagnostics";

export interface RuntimeConfigureOptions {
  sqliteMode: RuntimeSqliteMode;
}

export interface RuntimeConfiguration {
  sqliteMode: RuntimeSqliteMode;
}

export const admin = {
  configureRuntime(options: RuntimeConfigureOptions): RuntimeConfiguration {
    if (options.sqliteMode !== "performance" && options.sqliteMode !== "diagnostics") {
      throw new RangeError("sqliteMode must be 'performance' or 'diagnostics'");
    }
    return interceptSync(() => native.adminConfigureRuntime(options));
  },
  async configure(engine: Engine, options: AdminConfigureOptions): Promise<WriteReceipt> {
    validateFfiString(options.name);
    validateFfiString(options.body);
    return intercept(() => native.adminConfigure(engine._native, options));
  },
};
