<template>
    <div>
        <button>Upload Files(s)</button>
        <p>or</p>
        <button @click="toggleSessionNameInput" :class="showSessionNameInput ? 'scale-0' : 'scale-1'">Join
            Session</button>
        <form action="submit" @submit.prevent="joinSession">
            <input id="sessionInput" type="text" name="session-name" placeholder="Session Name" v-model="sessionName"
                :class="showSessionNameInput ? 'scale-1' : 'scale-0'"></input>
        </form>
    </div>
</template>

<script setup>
let sessionName = ref('');
let showSessionNameInput = ref(false);

const toggleSessionNameInput = () => {
    showSessionNameInput.value = !showSessionNameInput.value;
    const sessionInput = document.getElementById('sessionInput');
    sessionInput.focus();
}

const joinSession = () => {
    const sessionName = document.getElementById('sessionInput').value;
    navigateTo(`/${sessionName}`);
}
</script>