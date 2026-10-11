// src/index.ts
import { initializeFirebase, env as env2, logger as logger2 } from "@almadar/server";

// src/app.ts
import express from "express";
import cors from "cors";
import helmet from "helmet";
import rateLimit from "express-rate-limit";
import {
  env,
  errorHandler,
  notFoundHandler,
  debugEventsRouter
} from "@almadar/server";

// src/routes.ts
function registerRoutes(app2) {
}

// src/app.ts
var app = express();
app.use(helmet());
app.use(cors({ origin: env.CORS_ORIGIN, credentials: true }));
app.use(express.json({ limit: "1mb" }));
app.use(express.urlencoded({ extended: true, limit: "1mb" }));
app.use("/api", rateLimit({ windowMs: 15 * 60 * 1e3, max: 300, standardHeaders: true, legacyHeaders: false }));
app.get("/health", (_req, res) => {
  res.json({ status: "ok" });
});
app.use("/api/debug", debugEventsRouter());
registerRoutes(app);
app.use(notFoundHandler);
app.use(errorHandler);

// src/index.ts
initializeFirebase();
var PORT = env2.PORT || 3030;
async function start() {
  if (env2.USE_MOCK_DATA) {
    try {
      const { initializeMockData } = await import(
        /* @vite-ignore */
        "./seedMockData.js"
      );
      await initializeMockData();
    } catch {
      logger2.warn("seedMockData.ts not found \u2014 skipping mock data seeding");
    }
  }
  app.listen(PORT, () => {
    logger2.info(`Server running on port ${PORT}`);
  });
}
start();
