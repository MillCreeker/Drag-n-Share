// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
    compatibilityDate: "2024-04-03",
    devtools: { enabled: true },
    modules: ["@nuxtjs/tailwindcss"],
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
                    href: "favicon.ico"
                },
                {
                    rel: "stylesheet",
                    href: "https://fonts.googleapis.com/icon?family=Material+Icons",
                },
            ],
        },
    },
    runtimeConfig: {
        apiKey: process.env.API_KEY,
    }
});
