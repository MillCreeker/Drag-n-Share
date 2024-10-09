<template>

    <Head>
        <Title>Drag-n-Share | {{ sessionName }}</Title>
    </Head>

    <div class="text-center">
        <h1 class="text-5xl font-bold tracking-tight text-yellow-500">{{ sessionName }}</h1>
        <form @submit.prevent="joinSession">
            <input id="codeInput" type="text" name="code" placeholder="000 000" pattern="[0-9\s]{7}" inputmode="numeric"
                maxlength="7"
                class="rounded-lg border-2 border-gray-300 px-0 py-2 text-2xl transition focus:outline-none focus:border-orange-500 text-center"
                v-model="formattedCode" @input="formatCode" autofocus>
        </form>
    </div>
</template>

<script setup>
const config = useRuntimeConfig();

let sessionName = ref('');
let formattedCode = ref('');
let code = ref('');

const route = useRoute();
const sessionId = route.path.split('/')[2];

try {
    const data = await $fetch(`${config.public.apiUri}/session/${sessionId}`, {
        method: 'GET'
    });

    const results = JSON.parse(data);
    if (results.success) {
        sessionName.value = results.response.sessionName;
    }
} catch (error) {
    navigateTo('/');
}

const formatCode = () => {
    code.value = formattedCode.value.replace(/\D/g, '');

    if (code.value.length > 3) {
        formattedCode.value = code.value.replace(/(\d{3})(\d{1,3})/, '$1 $2');
    } else {
        formattedCode.value = code.value;
    }
};

const joinSession = async () => {
    const hash = await encodeSha256(code.value);

    try {
        const data = await $fetch(`${config.public.apiUri}/access/${sessionId}`, {
            method: 'GET',
            headers: {
                Authorization: hash
            }
        });

        const results = JSON.parse(data);
        if (results.success) {
            const response = results.response;
            console.log('response', response);
            const jwtCookie = useCookie('jwt');
            jwtCookie.value = response.jwt;

            navigateTo(`/${sessionId}`);
        }
    } catch (error) {
        console.log(error);
    }
};

const encodeSha256 = async (input) => {
    const encoder = new TextEncoder();
    const data = encoder.encode(input);

    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hash = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

    return hash;
}
</script>