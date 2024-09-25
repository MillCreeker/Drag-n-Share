<template>
    <div>

        <Head>
            <Title>Drag-n-Share | {{ sessionName }}</Title>
        </Head>

        <p>{{ sessionName }}</p>
        <p>{{ data }}</p>

        <File filename="Test.pdf" />
    </div>
</template>

<script setup>
const config = useRuntimeConfig();

$fetch(`${config.public.apiUri}/session`).then((res) => {
    const results = JSON.parse(res);
    console.log(results);
}).catch((err) => {
    createError({
        statusCode: 500,
        statusMessage: 'Internal Server Error',
        message: err.message,
        fatal: true
    });

    navigateTo('/error');
});
</script>