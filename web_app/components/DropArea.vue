<template>
    <div class="flex justify-end mr-8 md:mr-16 lg:mr-40 mt-4">
        <i v-if="!isServerOnline" class="material-icons text-orange-500">wifi_off</i>
        <i v-else class="material-icons text-orange-500">wifi</i>
    </div>
    <div id="drop-area" class="rounded-lg border-dashed border-2 p-4 m-4 md:mx-8 lg:mx-32 mt-0 text-center"
        :class="{ 'border-yellow-500': isFileHovering, 'border-orange-500': !isFileHovering }">
        <slot></slot>
    </div>
</template>

<script setup>
import { uploadFiles, createSession } from '~/public/utils/api';
import { convertFiles } from '~/public/utils/utils';
const route = useRoute();
const config = useRuntimeConfig();

const { createSessionBeforeUpload, cbRefresh } = defineProps(['createSessionBeforeUpload', 'cbRefresh']);

let isFileHovering = ref(false);
let isServerOnline = ref(false);

onMounted(async () => {
    try {
        await $fetch(`${config.public.apiUri}/`,
            { server: false }
        );

        isServerOnline.value = true;
    } catch (error) {
        throw createError({
            statusCode: 500,
            statusMessage: 'Servers are currently offline. We\'re working on it, promise.',
            fatal: true
        });
    }

    const dropArea = document.getElementById('drop-area');

    dropArea.addEventListener('dragenter', (e) => {
        e.preventDefault();
        e.stopPropagation();
        isFileHovering.value = true;
    });

    dropArea.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();

        if (isFileInContainer(e)) {
            return;
        }

        isFileHovering.value = false;
    });

    dropArea.addEventListener('drop', async (e) => {
        e.preventDefault();
        e.stopPropagation();

        isFileHovering.value = false;

        const rawFiles = e.dataTransfer.files;
        const dT = new DataTransfer();

        for (let i = 0; i < rawFiles.length; i++) {
            const file = rawFiles[i];
            dT.items.add(file);
        }

        const files = await convertFiles(dT.files);

        if (createSessionBeforeUpload) {
            await createSession(files);
            return;
        }

        const sessionId = route.path.split('/')[1];
        await uploadFiles(files, sessionId);

        cbRefresh();
    });

    function isFileInContainer(e) {
        let isInContainer = true;
        const rect = dropArea.getBoundingClientRect();

        if (e.clientY < rect.top ||
            e.clientY >= rect.bottom ||
            e.clientX < rect.left ||
            e.clientX >= rect.right) {
            isInContainer = false
        }

        return isInContainer;
    };

    window.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
    });
    window.addEventListener('drop', (e) => {
        e.preventDefault();
        e.stopPropagation();
    });
});

</script>