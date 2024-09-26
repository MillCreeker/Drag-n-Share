<template>

    <Head>
        <Title>Drag-n-Share | {{ sessionName }}</Title>
    </Head>

    <div class="text-center">
        <h1 class="text-5xl font-bold tracking-tight text-yellow-500">{{ sessionName }}</h1>
        <p class="text-3xl tracking-widest font-medium text-orange-500"
            style="text-shadow: -1px -1px 0 black, 1px -1px 0 black, -1px 1px 0 black, 1px 1px 0 black">
            {{ accessCode }}
        </p>

        <!-- <File filename="Test.pdf" /> -->
    </div>
</template>

<script setup>
const config = useRuntimeConfig();

let sessionName = ref('');
let accessCode = ref('');

const jwtCookie = useCookie('jwt');

try {
    $fetch(`${config.public.apiUri}/session`,
        {
            headers: {
                Authorization: `Bearer ${jwtCookie.value}`
            }
        }
    ).then((res) => {
        const results = JSON.parse(res);

        if (results.success) {
            const response = results.response;
            console.log(response);
            sessionName.value = response.sessionName;
            accessCode.value = `${response.accessCode.substr(0, 3)} ${response.accessCode.substr(3)}`;
        } else {

        }
    });
} catch (err) {
    // TODO try auth as guest
    console.log(err);
}
</script>