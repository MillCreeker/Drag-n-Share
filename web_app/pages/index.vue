<template>
    <div>
        <DropArea createSessionBeforeUpload="true">
            <div class="h-[10vh]" />

            <UploadButton :createSessionBeforeUpload="true" />

            <p>or</p>

            <HoverButton @click="toggleSessionNameInput" :class="showSessionNameInput ? 'scale-0' : 'scale-1'">
                Join Session
            </HoverButton>

            <form action="submit" @submit.prevent="joinSession">
                <input id="sessionInput" type="text" name="session-name" placeholder="Session Name"
                    class="rounded-lg border-2 border-gray-300 p-2 text-lg transition duration-150 ease-in-out focus:outline-none focus:border-yellow-500"
                    v-model="sessionName" :class="showSessionNameInput ? 'scale-1' : 'scale-0'"
                    @blur="toggleSessionNameInput"></input>
            </form>

            <div class="h-[40vh] md:h-[30vh] lg:h-[30vh]" />
        </DropArea>
    </div>
</template>

<script setup>
import { clearStorage } from '~/public/utils/utils';
import { getIdForSessionName } from '~/public/utils/api';
import UploadButton from '../components/UploadButton.vue';
import { onMounted } from 'vue';
const config = useRuntimeConfig();
const jwtCookie = useCookie('jwt');


let sessionName = ref('');
let showSessionNameInput = ref(false);

const redirectIfHost = async () => {
    if (jwtCookie.value) {
        try {
            const data = await $fetch(`${config.public.apiUri}/session`,
                {
                    headers: {
                        Authorization: `Bearer ${jwtCookie.value}`
                    },
                    server: false
                }
            );

            const results = JSON.parse(data);

            if (results.success) {
                const response = results.response;
                navigateTo(`/${response.sessionId}`);
            }
        } catch (_) {
        }
    }
};

const toggleSessionNameInput = () => {
    showSessionNameInput.value = !showSessionNameInput.value;
    const sessionInput = document.getElementById('sessionInput');
    sessionInput.focus();
};

const joinSession = async () => {
    sessionName.value = sessionName.value.trim();

    try {
        const sessionId = await getIdForSessionName(sessionName.value);
        navigateTo(`/join/${sessionId}`);
    } catch (err) {
        // TODO show it doesn't exist
        console.error(err);
    }
};

onMounted(async () => {
    await redirectIfHost();

    clearStorage();
});
</script>