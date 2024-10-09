<template>
    <div>
        <DropArea>
            <div class="h-[10vh]" />

            <SubtleButton @click="openFileSelector">
                Upload File(s)
            </SubtleButton>

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
import { createSession, getIdForSessionName } from '~/public/utils/api';
const config = useRuntimeConfig();
const jwtCookie = useCookie('jwt');

// redirect user if they already host a session
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
            navigateTo(`/${response.sessionId}`);
        }
    } catch (_) {
    }
}

function openFileSelector() {
    createSession();
}

let sessionName = ref('');
let showSessionNameInput = ref(false);

const toggleSessionNameInput = () => {
    showSessionNameInput.value = !showSessionNameInput.value;
    const sessionInput = document.getElementById('sessionInput');
    sessionInput.focus();
}

const joinSession = async () => {
    sessionName.value = sessionName.value.trim();

    try {
        const sessionId = await getIdForSessionName(sessionName.value);
        navigateTo(`/join/${sessionId}`);
    } catch (err) {
        // TODO show it doesn't exist
        console.error(err);
    }
}
</script>