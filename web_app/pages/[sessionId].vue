<template>

    <Head>
        <Title>Drag-n-Share | {{ sessionName }}</Title>
    </Head>

    <div class="text-center">
        <div v-if="sessionName !== ''">
            <DropArea>
                <div class="relative mt-4">
                    <h1 v-if="!isEditing" class="text-5xl font-bold tracking-tight text-yellow-500 cursor-pointer"
                        @click="isHost ? isEditing = true : null">{{ sessionName }}</h1>
                    <form v-else @submit.prevent="updateSessionName">
                        <input type="text"
                            class="p-2 rounded-lg text-white font-bold tracking-wide bg-yellow-500 transition duration-150 ease-in-out"
                            autofocus style="width: min-content;" v-model="sessionName" @blur="isEditing = false" />
                    </form>
                </div>
                <p class="text-3xl tracking-widest font-medium text-orange-500"
                    style="text-shadow: -1px -1px 0 black, 1px -1px 0 black, -1px 1px 0 black, 1px 1px 0 black">
                    {{ accessCode }}
                </p>

                <UploadButton />

                <button v-if="isHost"
                    class="mt-4 p-2 rounded-lg text-white font-bold tracking-wide bg-red-500 hover:bg-red-700 active:bg-red-900"
                    @click="deleteSession">
                    Delete Session
                </button>
            </DropArea>

            <!-- <File filename="Test.pdf" /> -->
        </div>

        <div v-else>
            <NuxtLink to="/">
                <button
                    class="mt-4 p-2 rounded-lg text-white font-bold tracking-wide bg-gray-500 hover:bg-gray-700 active:bg-gray-900">
                    Fly Back
                </button>
            </NuxtLink>
        </div>
    </div>
</template>

<script setup>

import DropArea from '../components/DropArea.vue';
import UploadButton from '../components/UploadButton.vue';
const config = useRuntimeConfig();
const route = useRoute();

let sessionName = ref('');
let sessionId = ref('');
let accessCode = ref('');
let isHost = ref(false);
let isEditing = ref(false);

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
            console.error(error);
            // navigateTo('/');
        }
    }
}

const updateSessionName = async () => {
    sessionName.value = sessionName.value.trim();
    isEditing.value = false;

    try {
        const data = await $fetch(`${config.public.apiUri}/session/${sessionId.value}`,
            {
                method: 'PUT',
                headers: {
                    Authorization: `Bearer ${jwtCookie.value}`
                },
                body: {
                    name: sessionName.value
                }
            }
        );

        const results = JSON.parse(data);

        if (results.success) {
            const response = results.response;
            accessCode.value = `${response.accessCode.substr(0, 3)} ${response.accessCode.substr(3)}`;
        }
    } catch (error) {
        console.error(error);
    }
};

const deleteSession = async () => {
    if (!confirm('Are you sure you want to delete this session?')) {
        return;
    }

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
    } // TODO uplaod files
};
</script>