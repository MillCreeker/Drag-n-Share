import * as Sentry from "@sentry/nuxt";

Sentry.init({
    dsn: "https://8901dba96154528335ed3ea9c8caa352@o4508227941367808.ingest.de.sentry.io/4508227946414160",
    integrations: [Sentry.replayIntegration()],
    // Tracing
    // We recommend adjusting this value in production, or using a tracesSampler for finer control.
    tracesSampleRate: 1.0, //  Capture 100% of the transactions
    // Set 'tracePropagationTargets' to control for which URLs distributed tracing should be enabled
    tracePropagationTargets: ["localhost", /^https:\/\/drag-n-share.com/],
    // Session Replay
    replaysSessionSampleRate: 0.1, // This sets the sample rate at 10%. You may want to change it to 100% while in development and then sample at a lower rate in production.
    replaysOnErrorSampleRate: 1.0, // If you're not already sampling the entire session, change the sample rate to 100% when sampling sessions where errors occur.
});