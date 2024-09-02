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
    const { sessionName } = useRoute().params;


    // TODO
    const { data } = await useFetch(`/api/files/${sessionName}`);

    if (!data) {
        throw createError({ statusCode: 404, statusMessage: 'Not found', fatal: true });
    }
</script>