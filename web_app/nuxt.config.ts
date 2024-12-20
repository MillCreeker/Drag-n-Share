// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
    compatibilityDate: "2024-04-03",
    devtools: { enabled: false },
    modules: ["@nuxtjs/tailwindcss", "@sentry/nuxt/module"],
    app: {
        head: {
            title: "Drag-n-Share",
            meta: [
                {
                    name: "charset",
                    content: "utf-8",
                },
                {
                    name: "description",
                    content: "Drag, Share, Chill",
                },
            ],
            link: [
                {
                    rel: "icon",
                    href: "favicon.ico",
                },
                {
                    rel: "stylesheet",
                    href: "css/materialIcons.css",
                },
            ],
        },
    },
    runtimeConfig: {
        public: {
            apiUri: process.env.NUXT_PUBLIC_API_URI,
            wsUri: process.env.NUXT_PUBLIC_WS_URI,
            trnsUri: process.env.NUXT_PUBLIC_TRNS_URI,
        },
    },
});
