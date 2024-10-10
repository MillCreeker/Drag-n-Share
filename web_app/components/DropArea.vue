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
import { uploadFile, createSession } from '~/public/utils/api';
import { convertFiles } from '~/public/utils/utils';

const config = useRuntimeConfig();

const { createSessionAfterUpload } = defineProps(['createSessionAfterUpload']);

let isFileHovering = ref(false);
let isServerOnline = ref(false);

onMounted(async () => {
    try {
        await $fetch(`${config.public.apiUri}/`);

        isServerOnline.value = true;
    } catch (error) {
        console.error(error);
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

        for (let i = 0; i < files.length; i++) {
            await uploadFile(files[i]);
        }

        if (createSessionAfterUpload) {
            await createSession();
        }
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

    // function toBase64(file) {
    //     return new Promise((resolve, reject) => {
    //         const reader = new FileReader();
    //         reader.readAsDataURL(file);
    //         reader.onload = () => resolve(reader.result);
    //         reader.onerror = error => reject(error);
    //     })
    // };

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