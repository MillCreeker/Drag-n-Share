export default defineEventHandler(async (event) => {

    const { sessionName } = event.context.params;
    const { apiKey } = useRuntimeConfig();

    // const { data } = $fetch(`https://api.drag-n-share.com...`);

    // const { files } = await getBody(event);

    return {
        files: [
            {
                name: `Test.pdf ${sessionName}`,
                size: 123
            }
        ]
    };
});