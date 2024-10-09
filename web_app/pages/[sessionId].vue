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

        <button v-if="isHost"
            class="mt-4 p-2 rounded-lg text-white font-bold tracking-wide bg-red-500 hover:bg-red-700 active:bg-red-900"
            @click="deleteSession">
            Delete Session
        </button>
        <!-- <File filename="Test.pdf" /> -->
    </div>
</template>

<script setup>
const config = useRuntimeConfig();
const route = useRoute();

let sessionName = ref('');
let sessionId = ref('');
let accessCode = ref('');
let isHost = ref(false);

const jwtCookie = useCookie('jwt');

if (jwtCookie.value) {
    try {
        const data = await $fetch(`${config.public.apiUri}/session`,
            {
                headers: {
                    Authorization: `Bearer ${jwtCookie.value}`
                }
            }
        );

        const results = JSON.parse(data);

        if (results.success) {
            const response = results.response;
            sessionName.value = response.sessionName;
            sessionId.value = response.sessionId;
            accessCode.value = `${response.accessCode.substr(0, 3)} ${response.accessCode.substr(3)}`;
            isHost.value = true;
        } else {
            jwtCookie.value = '';
        }
    } catch (error) {
        const sessionIdFromPath = route.path.split('/')[1];
        console.log(route);
        try {
            const data = await $fetch(`${config.public.apiUri}/session/${sessionIdFromPath}`);

            const results = JSON.parse(data);
            if (results.success) {
                const response = results.response;
                sessionName.value = response.sessionName;
                sessionId.value = sessionIdFromPath;
                isHost.value = false;
            }
        } catch (error) {
            navigateTo('/');
        }
    }
}

const deleteSession = async () => {
    try {
        const data = await $fetch(`${config.public.apiUri}/session/${sessionId.value}`,
            {
                method: 'DELETE',
                headers: {
                    Authorization: `Bearer ${jwtCookie.value}`
                }
            }
        );

        const results = JSON.parse(data);

        if (results.success) {
            jwtCookie.value = '';
            navigateTo('/');
        }

    } catch (error) {
        console.error(error);
    }
}
</script>